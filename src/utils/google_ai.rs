use std::env;

use base64::{engine::general_purpose, Engine as _};
use log::info;
use reqwest::Client;
use serde_json::{json, Value};

#[derive(Debug)]
pub struct GoogleAiError {
    pub message: String,
}

#[derive(Debug)]
pub struct GoogleApiKey<'a> {
    pub tier: &'a str,
    pub env_var: &'a str,
}

pub const GOOGLE_API_FREE_KEY: GoogleApiKey = GoogleApiKey {
    tier: "free",
    env_var: "GOOGLE_API_FREE_KEY",
};

pub const GOOGLE_API_PAID_KEY: GoogleApiKey = GoogleApiKey {
    tier: "paid",
    env_var: "GEMINI_API_KEY",
};

struct GoogleAiResponse<'a> {
    pub text: String,
    pub tier_used: &'a str,
}

async fn post<'a>(
    reqwest_client: &Client,
    api_url: &str,
    request_body: &Value,
    keys_to_try: &[GoogleApiKey<'a>],
) -> Result<GoogleAiResponse<'a>, GoogleAiError> {
    let mut last_error_message = "No API keys configured or all attempts failed".to_string();
    for google_api_key in keys_to_try {
        let tier_used = google_api_key.tier;
        let api_key = match env::var(google_api_key.env_var) {
            Ok(key) if !key.is_empty() => key,
            _ => continue,
        };

        let response_result = reqwest_client
            .post(api_url)
            .header("x-goog-api-key", api_key)
            .json(request_body)
            .send()
            .await;

        let response = match response_result {
            Ok(resp) => resp,
            Err(e) => {
                last_error_message = format!("Request failed with tier {}: {}", tier_used, e);
                info!("{}", last_error_message);
                continue;
            }
        };

        let status_code = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status_code.is_success() {
            last_error_message = format!(
                "API Error with tier {} ({}):\n{}",
                tier_used, status_code, text
            );
            info!("{}", last_error_message);
            continue;
        }

        return Ok(GoogleAiResponse { text, tier_used });
    }

    Err(GoogleAiError {
        message: last_error_message,
    })
}

pub async fn generate_image<'a>(
    client: &Client,
    model: &str,
    parts: Vec<Value>,
    aspect_ratio: Option<&str>,
    keys: &[GoogleApiKey<'a>],
) -> Result<(Vec<u8>, &'a str), GoogleAiError> {
    let config = match aspect_ratio {
        Some(aspect_ratio) => json!({
            "responseModalities": ["IMAGE"],
            "imageConfig": { "aspectRatio": aspect_ratio }
        }),
        None => json!({ "responseModalities": ["IMAGE"] }),
    };
    let url =
        format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent");
    let response = post(
        client,
        &url,
        &json!({ "contents": [{ "parts": parts }], "generationConfig": config }),
        keys,
    )
    .await?;
    let value: Value = serde_json::from_str(&response.text).map_err(|e| GoogleAiError {
        message: format!(
            "JSON Parse Error with tier {} {}: \nResponse: {}",
            response.tier_used, e, response.text
        ),
    })?;
    if let Some(reason) = value
        .pointer("/promptFeedback/blockReason")
        .and_then(Value::as_str)
    {
        return Err(GoogleAiError {
            message: format!("Request blocked by safety filter: {reason}"),
        });
    }
    let data = value["candidates"]
        .as_array()
        .and_then(|candidates| {
            candidates
                .iter()
                .flat_map(|c| c["content"]["parts"].as_array())
                .flatten()
                .find_map(|part| part["inlineData"]["data"].as_str())
        })
        .ok_or_else(|| GoogleAiError {
            message: "Response contained no image data.".to_string(),
        })?;
    general_purpose::STANDARD
        .decode(data)
        .map(|image| (image, response.tier_used))
        .map_err(|e| GoogleAiError {
            message: format!("Base64 Decode Error: {e}"),
        })
}
