use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use image::{
    DynamicImage, GenericImageView, ImageFormat, RgbaImage, codecs::jpeg::JpegEncoder, imageops,
};
use reqwest::{Client, multipart};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::time::SystemTime;

#[derive(Serialize)]
struct InpaintRequest {
    source_id: String,
    prompt: String,
    negative_prompt: String,
    mask_image_base64: String,
    seed: i64,
}

#[derive(Deserialize)]
struct MiddlewareResponse {
    x: u32,
    y: u32,
    color: String,
}

pub fn generate_source_id(path_str: &str) -> Result<String> {
    let path = Path::new(path_str);
    let metadata = fs::metadata(path)?;
    let mod_time = metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let mut hasher = blake3::Hasher::new();
    hasher.update(path_str.as_bytes());
    hasher.update(&mod_time.to_le_bytes());
    Ok(hasher.finalize().to_hex().to_string())
}

fn image_to_base64(img: &DynamicImage) -> Result<String> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Png)?;
    Ok(general_purpose::STANDARD.encode(buf.get_ref()))
}

fn image_to_jpeg_bytes(img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    let mut encoder = JpegEncoder::new_with_quality(&mut buf, quality);
    encoder.encode_image(&img.to_rgb8())?;
    Ok(buf.into_inner())
}

async fn upload_source_image(
    client: &Client,
    base_url: &str,
    source_id: &str,
    image: &DynamicImage,
    token: Option<&str>,
) -> Result<()> {
    let jpeg_bytes = image_to_jpeg_bytes(image, 95)?;

    let part = multipart::Part::bytes(jpeg_bytes)
        .file_name("source.jpg")
        .mime_str("image/jpeg")?;

    let form = multipart::Form::new()
        .text("source_id", source_id.to_string())
        .part("file", part);

    let mut req = client
        .post(format!("{}/upload_source", base_url))
        .multipart(form);

    if let Some(auth_token) = token {
        req = req.bearer_auth(auth_token);
    }

    let res = req.send().await?;

    if !res.status().is_success() {
        return Err(anyhow!("Upload failed: {}", res.text().await?));
    }
    Ok(())
}

fn composite_full_res(
    response: MiddlewareResponse,
    full_width: u32,
    full_height: u32,
) -> Result<RgbaImage> {
    let crop_color_bytes = general_purpose::STANDARD.decode(&response.color)?;
    let crop_color = image::load_from_memory(&crop_color_bytes)?;

    let mut full_color = RgbaImage::new(full_width, full_height);
    imageops::overlay(
        &mut full_color,
        &crop_color,
        response.x.into(),
        response.y.into(),
    );

    Ok(full_color)
}

pub async fn check_status(address: &str) -> Result<bool> {
    let client = Client::new();
    let res = client
        .get(format!("http://{}/health", address))
        .send()
        .await;
    Ok(res.is_ok())
}

// Once warm the local stack holds ~9GB of SDXL weights, so it is not kept
// running: ~/start-ai.sh stops itself when idle and is booted back up here on
// the first generative edit.
const LAUNCHER: &str = "start-ai.sh";

pub fn is_local(address: &str) -> bool {
    ["127.0.0.1", "localhost", "[::1]"]
        .iter()
        .any(|h| address.starts_with(h))
}

pub fn local_launcher() -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(&std::env::var_os("HOME")?).join(LAUNCHER);
    path.is_file().then_some(path)
}

/// Bring the local stack up if it isn't already. No-op when the connector
/// answers; an error here means the generative edit genuinely can't proceed.
pub async fn ensure_local_stack(address: &str) -> Result<()> {
    if check_status(address).await.unwrap_or(false) {
        return Ok(());
    }
    if !is_local(address) {
        return Err(anyhow!("AI Connector at {address} is not reachable"));
    }
    let Some(script) = local_launcher() else {
        return Err(anyhow!(
            "AI Connector is not running, and ~/{LAUNCHER} was not found to start it"
        ));
    };

    log::info!("[ai] connector down — booting local stack via {}", script.display());
    std::process::Command::new("/bin/bash")
        .arg(&script)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    // Cold start is ComfyUI's boot plus its model-dir scan — poll rather than
    // guess at a fixed delay.
    for _ in 0..90 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if check_status(address).await.unwrap_or(false) {
            log::info!("[ai] local stack ready");
            return Ok(());
        }
    }
    Err(anyhow!("Local AI stack did not come up within 3 minutes"))
}

pub async fn process_inpainting(
    base_url: &str,
    source_path: &str,
    full_source_image: &DynamicImage,
    mask_image: &DynamicImage,
    prompt: String,
    token: Option<&str>,
) -> Result<RgbaImage> {
    let client = Client::new();
    let source_id = generate_source_id(source_path)?;
    let mask_b64 = image_to_base64(mask_image)?;
    let (w, h) = full_source_image.dimensions();

    let payload = InpaintRequest {
        source_id: source_id.clone(),
        prompt,
        negative_prompt: "blur, low quality, distortion, watermark".to_string(),
        mask_image_base64: mask_b64,
        seed: 0,
    };

    let url = format!("{}/inpaint", base_url);

    let mut req = client.post(&url).json(&payload);
    if let Some(auth_token) = token {
        req = req.bearer_auth(auth_token);
    }

    let response = req.send().await?;

    let middleware_data: MiddlewareResponse = if response.status() == 404 {
        upload_source_image(&client, base_url, &source_id, full_source_image, token).await?;

        let mut retry_req = client.post(&url).json(&payload);
        if let Some(auth_token) = token {
            retry_req = retry_req.bearer_auth(auth_token);
        }

        let retry_res = retry_req.send().await?;
        if !retry_res.status().is_success() {
            return Err(anyhow!(
                "AI generation failed after upload: {}",
                retry_res.text().await?
            ));
        }
        retry_res.json().await?
    } else if !response.status().is_success() {
        return Err(anyhow!("AI generation failed: {}", response.text().await?));
    } else {
        response.json().await?
    };

    composite_full_res(middleware_data, w, h)
}
