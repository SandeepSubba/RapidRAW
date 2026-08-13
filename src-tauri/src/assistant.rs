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
use tauri::AppHandle;

use crate::app_settings;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Clone)]
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

You have permission to edit ALL of the above. Whatever the user asks to store (a code, a note, keywords), pick the field they name; if they don't name one, choose the most fitting field (e.g. keywords -> tags, a title/code -> title).

Rules:
- ALWAYS respond with a single JSON object and NOTHING else, no markdown, no code fences:
  {"reply": "<short friendly message>", "adjustments": {<only fields you change>}, "metadata": {<only text fields you change>}, "tags": {"add": [...], "remove": [...]}, "rating": <0-5>, "colorLabel": "<color>"}
- Set any field you are NOT changing to null (adjustments, metadata, tags, rating, colorLabel).
- Use exactly the lowercase keys listed above (e.g. "title", not "Title").
- Only include fields you actually want to change; use absolute values within the ranges above.
- Take the current adjustments and current metadata (provided below) into account so your changes are sensible.
- If an image is attached, look at it and base your edits on what you see.
- When the user asks you to read/OCR text from the image and store it (e.g. "read the code on the label and write it to the title", or "put it on the tags"), extract the exact text, apply any requested transformation, and put the result in the field they named.
- Keep "reply" concise and say what you changed."#;

fn default_endpoint(provider: &str) -> &'static str {
    match provider {
        "openai" => "https://api.openai.com/v1",
        "anthropic" => "https://api.anthropic.com/v1",
        _ => "http://localhost:1234/v1", // lmstudio
    }
}

fn default_model(provider: &str) -> &'static str {
    match provider {
        "openai" => "gpt-4o-mini",
        "anthropic" => "claude-opus-5",
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
    let body = json!({
        "model": model,
        "messages": msgs,
        "temperature": 0.3,
        "stream": false,
    });

    let client = reqwest::Client::new();
    let mut req = client.post(&url).json(&body);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Could not reach {} at {}: {}", provider_label, url, e))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("{} error {}: {}", provider_label, status, truncate(&text, 500)));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("Bad JSON from {}: {}", provider_label, e))?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("{} returned no message content", provider_label))
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
    let body = json!({
        "model": model,
        "max_tokens": 1024,
        "system": system,
        "messages": msgs,
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
        return Err(format!("Anthropic error {}: {}", status, truncate(&text, 500)));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("Bad JSON from Anthropic: {}", e))?;
    // Concatenate every text block. Newer models can emit a non-text block first
    // (e.g. a thinking block), so we must not assume content[0] is the answer.
    if let Some(blocks) = v["content"].as_array() {
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
    // No text came back — surface why (stop_reason + a snippet) instead of a bare message.
    let stop = v["stop_reason"].as_str().unwrap_or("unknown");
    Err(format!(
        "Anthropic returned no text content (stop_reason: {}). Response: {}",
        stop,
        truncate(&text, 400)
    ))
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
        return Err(format!("Error {}: {}", status, truncate(&text, 400)));
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
    let models = fetch_models(&cfg.provider, &cfg.endpoint, &cfg.api_key).await?;
    let label = match cfg.provider.as_str() {
        "openai" => "OpenAI",
        "anthropic" => "Anthropic",
        _ => "LM Studio",
    };
    Ok(format!("Connected to {} — {} model(s) available", label, models.len()))
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
        Some(a) => serde_json::to_string(a).unwrap_or_else(|_| "unavailable".to_string()),
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
        provider: cfg.provider,
        model,
    })
}
