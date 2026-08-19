// AI assistant chat: a provider-agnostic bridge to an LLM that can chat with the
// user and, when asked, return a patch of develop-slider adjustments to apply to
// the currently open image. Supports LM Studio (local, OpenAI-compatible),
// OpenAI, and Anthropic. The model is instructed to answer with a single JSON
// object `{ "reply": string, "adjustments": object|null }`; we parse that out
// (robustly, tolerating code fences / stray prose) and hand it back to the UI,
// which clamps and applies the adjustments. Images can be attached to the latest
// user turn (vision), and the model can be overridden per-request so the UI can
// offer a live model picker.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Write as _;
use std::process::{Command, Stdio};
use tauri::AppHandle;

use crate::app_settings;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImageAttachment {
    pub media_type: String,
    pub data: String, // base64, without the `data:` prefix
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantResponse {
    pub reply: String,
    pub adjustments: Option<Value>,
    pub metadata: Option<Value>,
    pub tags: Option<Value>,
    pub rating: Option<Value>,
    pub color_label: Option<Value>,
    pub filename: Option<Value>,
    pub provider: String,
    pub model: String,
}

const SYSTEM_PROMPT: &str = r#"You are the editing assistant inside RapidRAW, a RAW photo editor. You help the user by chatting and, when they ask, by (a) adjusting the develop sliders and (b) writing text metadata fields of the currently open image.

You may set these numeric adjustment fields. Values are ABSOLUTE (the final slider value, not a delta):
- exposure: -5..5 (overall brightness / exposure in stops)
- contrast: -100..100
- highlights: -100..100 (recover bright areas with negative values)
- shadows: -100..100 (lift dark areas with positive values)
- whites: -100..100
- blacks: -100..100
- temperature: -100..100 (negative = cooler/bluer, positive = warmer/yellower)
- tint: -100..100 (negative = greener, positive = more magenta)
- vibrance: -100..100
- saturation: -100..100
- hue: -180..180
- clarity: -100..100
- dehaze: -100..100
- structure: -100..100
- sharpness: -100..100

You may also set these TEXT metadata fields (string values, written to the image's metadata). Use exactly these lowercase keys:
- title (the image title / description)
- author (the creator / artist)
- copyright
- comments

You may also organize the image:
- tags: {"add": ["keyword", ...], "remove": ["keyword", ...]} — keyword/tag labels to add or remove
- rating: an integer 0-5 (star rating; 0 clears it)
- colorLabel: one of "red", "yellow", "green", "blue", "purple", or "none" (to clear)
- filename: a new file name for the image, WITHOUT the extension (the extension is kept automatically). This renames the actual file on disk. Use only characters valid in a filename. Always propose the plain name you actually want — the app resolves collisions itself by appending -001, -002, and reports the final name back to you. Never invent a numeric suffix to dodge a clash you cannot see, and never ask the user whether a name is taken.

You have permission to edit ALL of the above, including renaming the file. Whatever the user asks to store (a code, a note, keywords), pick the field they name; if they don't name one, choose the most fitting field (e.g. keywords -> tags, a title/code -> title, "rename the file to X" -> filename).

Rules:
- ALWAYS respond with a single JSON object and NOTHING else, no markdown, no code fences:
  {"reply": "<short friendly message>", "adjustments": {<only fields you change>}, "metadata": {<only text fields you change>}, "tags": {"add": [...], "remove": [...]}, "rating": <0-5>, "colorLabel": "<color>", "filename": "<new name without extension>"}
- Set any field you are NOT changing to null (adjustments, metadata, tags, rating, colorLabel, filename).
- Use exactly the lowercase keys listed above (e.g. "title", not "Title").
- Only include fields you actually want to change; use absolute values within the ranges above.
- NEVER change adjustments, rating, or colorLabel unless the user EXPLICITLY asks for that kind of change. For a metadata / title / filename / tag request, do NOT touch adjustments, rating, or colorLabel at all — omit them (or set them to null). Applying an unrequested exposure change can black out the image.
- Take the current adjustments and current metadata (provided below) into account so your changes are sensible.
- If an image is attached, look at it and base your edits on what you see.
- When the user asks you to read/OCR text from the image and store it (e.g. "read the code on the label and write it to the title", or "put it on the tags"), extract the exact text, apply any requested transformation, and put the result in the field they named.
- CRITICAL: Describing a change in "reply" does NOTHING. A change is applied ONLY if you put it in its structured field (metadata, tags, rating, colorLabel, filename, adjustments). Never say you changed something without also filling the matching field in the SAME response. If the user says "do the same" or refers to an earlier workflow, re-emit all the fields now.
- Keep "reply" concise and say what you changed."#;

fn default_endpoint(provider: &str) -> &'static str {
    match provider {
        "openai" => "https://api.openai.com/v1",
        "anthropic" => "https://api.anthropic.com/v1",
        "claudecode" => "claude", // path to the Claude Code CLI binary (on PATH)
        _ => "http://localhost:1234/v1", // lmstudio
    }
}

fn default_model(provider: &str) -> &'static str {
    match provider {
        "openai" => "gpt-4o-mini",
        "anthropic" => "claude-opus-5",
        "claudecode" => "claude-sonnet-5",
        _ => "local-model", // lmstudio uses whatever model is loaded
    }
}

fn build_url(base: &str, path: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    // Allow the user to paste a full chat endpoint; don't double-append.
    if base.ends_with("/chat/completions") || base.ends_with("/messages") {
        return base.to_string();
    }
    format!("{}{}", base, path)
}

fn models_url(base: &str) -> String {
    format!("{}/models", base.trim().trim_end_matches('/'))
}

// Turn a provider's error response into a readable message. OpenAI and Anthropic
// both nest a human message at error.message; fall back to the raw body.
fn provider_error(label: &str, status: reqwest::StatusCode, body: &str) -> String {
    let mut msg = None;
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        // error.message (OpenAI/Anthropic), top-level message, or a bare string
        // error field (LM Studio returns {"error":"...context size..."}).
        msg = v["error"]["message"]
            .as_str()
            .or_else(|| v["message"].as_str())
            .or_else(|| v["error"].as_str())
            .map(|s| s.to_string());
    }
    let msg = msg.unwrap_or_else(|| format!("error {}: {}", status, truncate(body, 400)));
    // The most common local-model failure: the image + prompt overflow a small
    // context window. Give an actionable hint instead of a cryptic token count.
    let lower = msg.to_lowercase();
    if lower.contains("context size") || lower.contains("context length") || lower.contains("context window") {
        return format!(
            "{}: {}\n\nThe image + prompt are larger than the model's context window. In LM Studio, reload the model with a larger context length (8192 or more), or attach a smaller image.",
            label, msg
        );
    }
    format!("{}: {}", label, msg)
}

fn normalize_role(role: &str) -> &str {
    if role == "assistant" {
        "assistant"
    } else {
        "user"
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn strip_code_fences(s: &str) -> String {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix("```") {
        // drop an optional language tag on the first line, and the closing fence
        let rest = rest.splitn(2, '\n').nth(1).unwrap_or(rest);
        return rest.trim_end_matches("```").trim().to_string();
    }
    t.to_string()
}

fn non_empty_object(v: Option<&Value>) -> Option<Value> {
    v.cloned().and_then(|a| match a {
        Value::Null => None,
        Value::Object(ref m) if m.is_empty() => None,
        other => Some(other),
    })
}

#[derive(Default)]
struct Parsed {
    reply: String,
    adjustments: Option<Value>,
    metadata: Option<Value>,
    tags: Option<Value>,
    rating: Option<Value>,
    color_label: Option<Value>,
    filename: Option<Value>,
}

fn extract(v: &Value, original: &str) -> Parsed {
    let reply = v
        .get("reply")
        .and_then(|r| r.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| original.to_string());
    Parsed {
        reply,
        adjustments: non_empty_object(v.get("adjustments")),
        metadata: non_empty_object(v.get("metadata")),
        tags: non_empty_object(v.get("tags")),
        // Accept a few likely spellings for the color-label key.
        rating: v.get("rating").cloned().filter(|r| r.is_number()),
        color_label: v
            .get("colorLabel")
            .or_else(|| v.get("color_label"))
            .or_else(|| v.get("color"))
            .cloned()
            .filter(|c| c.is_string()),
        filename: v
            .get("filename")
            .or_else(|| v.get("fileName"))
            .cloned()
            .filter(|f| f.is_string()),
    }
}

fn parse_assistant_content(content: &str) -> Parsed {
    let cleaned = strip_code_fences(content);
    if let Ok(v) = serde_json::from_str::<Value>(&cleaned) {
        return extract(&v, content);
    }
    // Fall back to the widest {...} span in the text.
    if let (Some(start), Some(end)) = (cleaned.find('{'), cleaned.rfind('}')) {
        if end > start {
            if let Ok(v) = serde_json::from_str::<Value>(&cleaned[start..=end]) {
                return extract(&v, content);
            }
        }
    }
    Parsed {
        reply: content.trim().to_string(),
        ..Default::default()
    }
}

// Build the OpenAI-compatible content for one message. Plain string unless this
// is the latest user turn and images are attached, in which case a content array.
fn openai_content(text: &str, images: &[ImageAttachment], attach: bool) -> Value {
    if !attach || images.is_empty() {
        return Value::String(text.to_string());
    }
    let mut parts = vec![json!({ "type": "text", "text": text })];
    for img in images {
        parts.push(json!({
            "type": "image_url",
            "image_url": { "url": format!("data:{};base64,{}", img.media_type, img.data) }
        }));
    }
    Value::Array(parts)
}

// Anthropic wants image blocks before the text block.
fn anthropic_content(text: &str, images: &[ImageAttachment], attach: bool) -> Value {
    if !attach || images.is_empty() {
        return Value::String(text.to_string());
    }
    let mut parts: Vec<Value> = images
        .iter()
        .map(|img| {
            json!({
                "type": "image",
                "source": { "type": "base64", "media_type": img.media_type, "data": img.data }
            })
        })
        .collect();
    parts.push(json!({ "type": "text", "text": text }));
    Value::Array(parts)
}

async fn call_openai_compatible(
    base: &str,
    api_key: &str,
    model: &str,
    system: &str,
    messages: &[ChatMessage],
    images: &[ImageAttachment],
    provider_label: &str,
) -> Result<String, String> {
    let url = build_url(base, "/chat/completions");
    let last_idx = messages.len().saturating_sub(1);
    let mut msgs = vec![json!({ "role": "system", "content": system })];
    for (i, m) in messages.iter().enumerate() {
        let role = normalize_role(&m.role);
        let attach = i == last_idx && role == "user";
        msgs.push(json!({ "role": role, "content": openai_content(&m.content, images, attach) }));
    }
    // Force HTTP/1.1: reqwest+rustls otherwise negotiates HTTP/2 via ALPN, and
    // some endpoints (seen with api.moonshot.ai) reset the h2 connection, which
    // surfaces as an opaque "error sending request" with no HTTP response.
    let client = reqwest::Client::builder()
        .http1_only()
        .build()
        .map_err(|e| format!("HTTP client init failed: {}", e))?;
    // NOTE: no `temperature` — some models (e.g. Moonshot/Kimi) reject any value
    // other than their fixed default and 400 the whole request. The JSON schema
    // constrains the output shape regardless, so a custom temperature isn't worth
    // the compatibility cost.
    let base_body = json!({
        "model": model,
        "messages": msgs,
        "stream": false,
    });

    // One attempt with a specific body; returns Ok(content) or Err(message).
    let attempt = |body: Value| {
        let client = client.clone();
        let url = url.clone();
        let api_key = api_key.to_string();
        let provider_label = provider_label.to_string();
        async move {
            let mut req = client.post(&url).json(&body);
            if !api_key.is_empty() {
                req = req.bearer_auth(&api_key);
            }
            let resp = req.send().await.map_err(|e| {
                // Include the underlying cause chain — reqwest's top-level message
                // ("error sending request for url ...") hides whether it was TLS,
                // DNS, a reset connection, or a timeout.
                let mut msg = format!("Could not reach {} at {}: {}", provider_label, url, e);
                let mut src = std::error::Error::source(&e);
                while let Some(s) = src {
                    msg.push_str(" | caused by: ");
                    msg.push_str(&s.to_string());
                    src = std::error::Error::source(s);
                }
                msg
            })?;
            let status = resp.status();
            let text = resp.text().await.map_err(|e| e.to_string())?;
            if !status.is_success() {
                return Err(provider_error(&provider_label, status, &text));
            }
            let v: Value =
                serde_json::from_str(&text).map_err(|e| format!("Bad JSON from {}: {}", provider_label, e))?;
            v["choices"][0]["message"]["content"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| format!("{} returned no message content", provider_label))
        }
    };

    // Prefer a strict typed schema so small models place fields correctly.
    let mut schema_body = base_body.clone();
    schema_body["response_format"] = edits_response_format();
    match attempt(schema_body).await {
        Ok(content) => Ok(content),
        Err(e) => {
            // Fall back to a plain request if the server doesn't support
            // response_format / json_schema; otherwise surface the real error.
            let el = e.to_lowercase();
            let unsupported = el.contains("response_format")
                || el.contains("response format")
                || el.contains("json_schema")
                || el.contains("json schema");
            if unsupported {
                attempt(base_body).await
            } else {
                Err(e)
            }
        }
    }
}

async fn call_anthropic(
    base: &str,
    api_key: &str,
    model: &str,
    system: &str,
    messages: &[ChatMessage],
    images: &[ImageAttachment],
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("Anthropic API key is not set (Settings → AI Assistant).".to_string());
    }
    let url = build_url(base, "/messages");
    let last_idx = messages.len().saturating_sub(1);
    let msgs: Vec<Value> = messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let role = normalize_role(&m.role);
            let attach = i == last_idx && role == "user";
            json!({ "role": role, "content": anthropic_content(&m.content, images, attach) })
        })
        .collect();
    // Force a structured tool call so the model can't just narrate ("I set the
    // title…") without emitting the fields we actually apply.
    let body = json!({
        "model": model,
        "max_tokens": 1024,
        "system": system,
        "messages": msgs,
        "tools": [apply_edits_tool()],
        "tool_choice": { "type": "tool", "name": "apply_edits" },
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Could not reach Anthropic at {}: {}", url, e))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(provider_error("Anthropic", status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("Bad JSON from Anthropic: {}", e))?;
    if let Some(blocks) = v["content"].as_array() {
        // Preferred path: the forced tool call carries the structured fields in its
        // `input`. Serialize it back to JSON so the shared parser can read it.
        if let Some(input) = blocks
            .iter()
            .find(|b| b["type"] == "tool_use" && b["name"] == "apply_edits")
            .map(|b| &b["input"])
        {
            if let Ok(s) = serde_json::to_string(input) {
                return Ok(s);
            }
        }
        // Fallback: concatenate any text blocks (newer models can emit a non-text
        // block first, so we don't assume content[0] is the answer).
        let joined: String = blocks
            .iter()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if !joined.trim().is_empty() {
            return Ok(joined);
        }
    }
    // No content came back — surface why (stop_reason + a snippet) instead of a bare message.
    let stop = v["stop_reason"].as_str().unwrap_or("unknown");
    Err(format!(
        "Anthropic returned no content (stop_reason: {}). Response: {}",
        stop,
        truncate(&text, 400)
    ))
}

// The schema the model is forced to fill. Optional fields are simply omitted when
// there's no change; `extract()` treats a missing field as "no change".
fn apply_edits_tool() -> Value {
    json!({
        "name": "apply_edits",
        "description": "Reply to the user and apply any requested edits to the currently open image. Populate a field ONLY when you want to change it; describing a change in `reply` without filling its field does nothing.",
        "input_schema": {
            "type": "object",
            "properties": {
                "reply": { "type": "string", "description": "Short, friendly message to show the user." },
                "adjustments": { "type": "object", "description": "Develop-slider changes, absolute values (e.g. {\"exposure\": 0.3, \"contrast\": 10})." },
                "metadata": {
                    "type": "object",
                    "description": "Text metadata to write.",
                    "properties": {
                        "title": { "type": "string" },
                        "author": { "type": "string" },
                        "copyright": { "type": "string" },
                        "comments": { "type": "string" }
                    }
                },
                "tags": {
                    "type": "object",
                    "description": "Keyword tags to add/remove.",
                    "properties": {
                        "add": { "type": "array", "items": { "type": "string" } },
                        "remove": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "rating": { "type": "integer", "minimum": 0, "maximum": 5, "description": "Star rating; 0 clears." },
                "colorLabel": { "type": "string", "enum": ["red", "yellow", "green", "blue", "purple", "none"] },
                "filename": { "type": "string", "description": "New file name WITHOUT extension (renames the file on disk)." }
            },
            "required": ["reply"]
        }
    })
}

// OpenAI-compatible `response_format` that pins the exact JSON shape. Small local
// models otherwise misplace fields (e.g. dumping `title`/`filename` into
// `adjustments`). The typed properties + `additionalProperties:false` stop
// misplacement, while `required` is ONLY `["reply"]` so the model is free to OMIT
// fields it isn't changing. (Requiring every field pushes weak models to invent
// values — e.g. exposure -5, which blacks out the image — so we must not do that.)
fn edits_response_format() -> Value {
    let num = json!({ "type": ["number", "null"] });
    let adjustments_props: Value = {
        let keys = [
            "exposure",
            "contrast",
            "highlights",
            "shadows",
            "whites",
            "blacks",
            "temperature",
            "tint",
            "vibrance",
            "saturation",
            "hue",
            "clarity",
            "dehaze",
            "structure",
            "sharpness",
        ];
        let mut m = serde_json::Map::new();
        for k in keys {
            m.insert(k.to_string(), num.clone());
        }
        Value::Object(m)
    };
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": "rapidraw_edits",
            "strict": false,
            "schema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["reply"],
                "properties": {
                    "reply": { "type": "string" },
                    "adjustments": {
                        "type": ["object", "null"],
                        "additionalProperties": false,
                        "properties": adjustments_props
                    },
                    "metadata": {
                        "type": ["object", "null"],
                        "additionalProperties": false,
                        "properties": {
                            "title": { "type": ["string", "null"] },
                            "author": { "type": ["string", "null"] },
                            "copyright": { "type": ["string", "null"] },
                            "comments": { "type": ["string", "null"] }
                        }
                    },
                    "tags": {
                        "type": ["object", "null"],
                        "additionalProperties": false,
                        "properties": {
                            "add": { "type": ["array", "null"], "items": { "type": "string" } },
                            "remove": { "type": ["array", "null"], "items": { "type": "string" } }
                        }
                    },
                    "rating": { "type": ["integer", "null"], "minimum": 0, "maximum": 5 },
                    "colorLabel": { "type": ["string", "null"], "enum": ["red", "yellow", "green", "blue", "purple", "none", null] },
                    "filename": { "type": ["string", "null"] }
                }
            }
        }
    })
}

// Flatten a conversation (plus any image file references) into a single prompt
// for the Claude Code CLI, which we drive over stdin.
fn build_cli_prompt(messages: &[ChatMessage], image_files: &[String]) -> String {
    let mut s = String::new();
    for m in messages {
        let role = if normalize_role(&m.role) == "assistant" { "Assistant" } else { "User" };
        s.push_str(role);
        s.push_str(": ");
        s.push_str(&m.content);
        s.push_str("\n\n");
    }
    if !image_files.is_empty() {
        s.push_str("Read the following image file(s) in the current directory and base your answer on what you actually see in them:\n");
        for f in image_files {
            s.push_str("- ");
            s.push_str(f);
            s.push('\n');
        }
        s.push('\n');
    }
    s.push_str("Respond now as the RapidRAW assistant with the single required JSON object and nothing else.");
    s
}

// Use the user's logged-in Claude Code CLI (subscription auth) instead of an API
// key. We write any attached images to a temp dir, run `claude -p` there (so it
// won't pick up a project CLAUDE.md), let it Read the images, and take the JSON
// out of the CLI's result envelope.
async fn call_claude_code(
    binary: &str,
    model: &str,
    system: &str,
    messages: &[ChatMessage],
    images: &[ImageAttachment],
) -> Result<String, String> {
    let binary = binary.trim().to_string();
    let binary = if binary.is_empty() { "claude".to_string() } else { binary };
    let model = model.to_string();
    let system = system.to_string();
    let messages: Vec<ChatMessage> = messages
        .iter()
        .map(|m| ChatMessage { role: m.role.clone(), content: m.content.clone() })
        .collect();
    let images = images.to_vec();

    tauri::async_runtime::spawn_blocking(move || {
        use base64::Engine as _;

        // Unique temp working dir; also the CWD so no project CLAUDE.md is loaded.
        let dir = std::env::temp_dir().join(format!(
            "rapidraw-cc-{}-{}",
            std::process::id(),
            messages.len()
        ));
        std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't create temp dir: {}", e))?;

        let mut image_files = Vec::new();
        for (i, img) in images.iter().enumerate() {
            let ext = if img.media_type.contains("png") { "png" } else { "jpg" };
            let fname = format!("image_{}.{}", i, ext);
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&img.data) {
                if std::fs::write(dir.join(&fname), &bytes).is_ok() {
                    image_files.push(fname);
                }
            }
        }

        // The system prompt embeds the current adjustments/metadata JSON, which
        // can exceed the OS argv limit (E2BIG) once an image with masks is open.
        // stdin has no such limit, so prepend it to the piped prompt instead of
        // passing it via --append-system-prompt.
        let prompt = format!("{}\n\n{}", system, build_cli_prompt(&messages, &image_files));

        let mut cmd = Command::new(&binary);
        cmd.current_dir(&dir)
            .arg("-p")
            .arg("--output-format")
            .arg("json")
            .arg("--model")
            .arg(&model)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // The CLI is a console app, so Windows hands it a console window that
        // flashes up for the life of every chat turn. All three streams are piped
        // — nothing is ever shown there — so suppress it.
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        // Only the Read tool is ever needed (to look at the images); nothing can
        // write, run bash, or edit.
        if !image_files.is_empty() {
            cmd.arg("--allowedTools").arg("Read");
        }

        let mut child = cmd.spawn().map_err(|e| {
            format!(
                "Couldn't launch Claude Code ('{}'): {}. Make sure Claude Code is installed and logged in, or set the binary path in Settings.",
                binary, e
            )
        })?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(prompt.as_bytes());
            // stdin dropped here → closed, so the CLI stops waiting for input.
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Claude Code failed: {}", e))?;
        let _ = std::fs::remove_dir_all(&dir);

        let stdout = String::from_utf8_lossy(&output.stdout);
        // On failure the CLI still prints its JSON envelope with the readable
        // message at "result" — prefer that over dumping the raw envelope.
        if !output.status.success() && serde_json::from_str::<Value>(stdout.trim()).is_err() {
            let err = String::from_utf8_lossy(&output.stderr);
            let detail = if !err.trim().is_empty() { err } else { stdout.clone() };
            return Err(format!("Claude Code error: {}", truncate(detail.trim(), 400)));
        }

        let v: Value = serde_json::from_str(stdout.trim())
            .map_err(|e| format!("Unexpected Claude Code output: {} — {}", e, truncate(&stdout, 300)))?;
        if v["is_error"].as_bool().unwrap_or(false) {
            let msg = v["result"]
                .as_str()
                .or_else(|| v["api_error_status"].as_str())
                .unwrap_or("unknown error");
            return Err(format!("Claude Code: {}", msg));
        }
        v["result"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Claude Code returned no result".to_string())
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

struct ResolvedConfig {
    provider: String,
    endpoint: String,
    api_key: String,
    model: String,
}

fn resolve_config(app_handle: &AppHandle) -> Result<ResolvedConfig, String> {
    let settings = app_settings::load_settings(app_handle.clone())?;
    let provider = settings
        .assistant_provider
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "lmstudio".to_string());
    let api_key = settings.assistant_api_key.clone().unwrap_or_default();
    let endpoint = settings
        .assistant_endpoint
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default_endpoint(&provider).to_string());
    let model = settings
        .assistant_model
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default_model(&provider).to_string());
    Ok(ResolvedConfig {
        provider,
        endpoint,
        api_key,
        model,
    })
}

async fn fetch_models(provider: &str, endpoint: &str, api_key: &str) -> Result<Vec<String>, String> {
    // Claude Code has no /models endpoint; offer the current Claude models.
    if provider == "claudecode" {
        return Ok(vec![
            "claude-opus-5".to_string(),
            "claude-sonnet-5".to_string(),
            "claude-haiku-4-5".to_string(),
            "claude-opus-4-8".to_string(),
        ]);
    }
    let url = models_url(endpoint);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if provider == "anthropic" {
        if api_key.is_empty() {
            return Err("Anthropic API key is not set.".to_string());
        }
        req = req
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01");
    } else if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Could not reach {}: {}", url, e))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(provider_error("Provider", status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut models: Vec<String> = v["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    models.sort();
    Ok(models)
}

#[tauri::command]
pub async fn assistant_list_models(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let cfg = resolve_config(&app_handle)?;
    fetch_models(&cfg.provider, &cfg.endpoint, &cfg.api_key).await
}

#[tauri::command]
pub async fn assistant_test_connection(app_handle: AppHandle) -> Result<String, String> {
    let cfg = resolve_config(&app_handle)?;

    // For Claude Code, actually run the CLI once so we confirm it's installed and
    // logged in (fetch_models is just a static list for it).
    if cfg.provider == "claudecode" {
        let ping = vec![ChatMessage {
            role: "user".to_string(),
            content: "ping".to_string(),
        }];
        call_claude_code(
            &cfg.endpoint,
            &cfg.model,
            "Reply with ONLY {\"reply\":\"ok\"}",
            &ping,
            &[],
        )
        .await?;
        return Ok("Connected to Claude Code (using your Claude subscription)".to_string());
    }

    let models = fetch_models(&cfg.provider, &cfg.endpoint, &cfg.api_key).await?;
    let label = match cfg.provider.as_str() {
        "openai" => "OpenAI",
        "anthropic" => "Anthropic",
        _ => "LM Studio",
    };
    Ok(format!("Connected to {} — {} model(s) available", label, models.len()))
}

// Replace any string value over 1KB (mask bitmaps, embedded images) with a
// placeholder so the adjustments context stays small. Real slider values and
// names are all far below this.
fn strip_bulky_strings(v: &mut Value) {
    match v {
        Value::String(s) if s.len() > 1024 => *s = "<large data omitted>".to_string(),
        Value::Array(arr) => arr.iter_mut().for_each(strip_bulky_strings),
        Value::Object(map) => map.values_mut().for_each(strip_bulky_strings),
        _ => {}
    }
}

#[tauri::command]
pub async fn assistant_chat(
    messages: Vec<ChatMessage>,
    adjustments: Option<Value>,
    current_metadata: Option<Value>,
    images: Option<Vec<ImageAttachment>>,
    model: Option<String>,
    app_handle: AppHandle,
) -> Result<AssistantResponse, String> {
    let cfg = resolve_config(&app_handle)?;
    let model = model
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(cfg.model);
    let images = images.unwrap_or_default();

    let adj_context = match &adjustments {
        Some(a) => {
            // Mask bitmaps etc. are embedded in the adjustments as huge base64
            // strings; they blow the model context (and argv/stdin limits) and
            // carry no meaning for the model. Strip them, keep the structure.
            let mut a = a.clone();
            strip_bulky_strings(&mut a);
            serde_json::to_string(&a).unwrap_or_else(|_| "unavailable".to_string())
        }
        None => "none (no image is currently open, so you cannot apply edits)".to_string(),
    };
    let meta_context = match &current_metadata {
        Some(m) => serde_json::to_string(m).unwrap_or_else(|_| "unavailable".to_string()),
        None => "none".to_string(),
    };
    let system = format!(
        "{}\n\nCurrent adjustments JSON:\n{}\n\nCurrent metadata JSON:\n{}",
        SYSTEM_PROMPT, adj_context, meta_context
    );

    let content = match cfg.provider.as_str() {
        "anthropic" => call_anthropic(&cfg.endpoint, &cfg.api_key, &model, &system, &messages, &images).await?,
        "claudecode" => call_claude_code(&cfg.endpoint, &model, &system, &messages, &images).await?,
        "openai" => {
            call_openai_compatible(&cfg.endpoint, &cfg.api_key, &model, &system, &messages, &images, "OpenAI").await?
        }
        _ => {
            call_openai_compatible(&cfg.endpoint, &cfg.api_key, &model, &system, &messages, &images, "LM Studio").await?
        }
    };

    let parsed = parse_assistant_content(&content);
    Ok(AssistantResponse {
        reply: parsed.reply,
        adjustments: parsed.adjustments,
        metadata: parsed.metadata,
        tags: parsed.tags,
        rating: parsed.rating,
        color_label: parsed.color_label,
        filename: parsed.filename,
        provider: cfg.provider,
        model,
    })
}
