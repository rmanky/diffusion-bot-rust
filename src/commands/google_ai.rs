use std::env;

use log::info;
use reqwest::Client;
use serde_json::Value;

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

pub struct GoogleAiResponse<'a> {
    pub text: String,
    pub tier_used: &'a str,
}

pub async fn post_generative_ai<'a>(
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

        return Ok(GoogleAiResponse {
            text,
            tier_used: tier_used,
        });
    }

    Err(GoogleAiError {
        message: last_error_message,
    })
}
