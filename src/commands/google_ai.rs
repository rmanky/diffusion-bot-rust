use std::env;

use log::info;
use reqwest::Client;
use serde_json::Value;

#[derive(Debug)]
pub struct GoogleAiError {
    pub message: String,
}

pub async fn post_generative_ai(
    reqwest_client: &Client,
    api_url: &str,
    request_body: &Value,
) -> Result<(String, &'static str), GoogleAiError> {
    let keys_to_try = [("free", "GEMINI_API_FREE_KEY"), ("paid", "GEMINI_API_KEY")];
    let mut last_error_message = "No API keys configured or all attempts failed".to_string();

    for (key_name, env_var) in keys_to_try {
        let api_key = match env::var(env_var) {
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
                last_error_message = format!("Request failed with key {}: {}", key_name, e);
                info!("{}", last_error_message);
                continue;
            }
        };

        let status_code = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status_code.is_success() {
            last_error_message = format!(
                "API Error with key {} ({}):\n{}",
                key_name, status_code, text
            );
            info!("{}", last_error_message);
            continue;
        }

        return Ok((text, key_name));
    }

    Err(GoogleAiError {
        message: last_error_message,
    })
}
