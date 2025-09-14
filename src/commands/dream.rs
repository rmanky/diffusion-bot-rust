use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use twilight_http::client::InteractionClient;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::{
    InteractionResponse,
    InteractionResponseData,
    InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder, ImageSource};

use super::{CommandHandler, CommandHandlerData};
use crate::commands::google_ai::{post_generative_ai, GoogleAiError};

#[derive(CommandOption, CreateOption)]
enum ImagenAspectRatio {
    #[option(name = "square", value = "1:1")]
    Square,
    #[option(name = "portrait", value = "9:16")]
    Portrait,
    #[option(name = "landscape", value = "16:9")]
    Landscape,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "dream", desc = "Create an image with Imagen")]
pub struct DreamCommand {
    /// Prompt for the model to generate.
    prompt: String,
    /// Select an aspect ratio. Uses 1:1 by default.
    aspect_ratio: Option<ImagenAspectRatio>,
}

struct DreamParams<'a> {
    prompt: &'a str,
    aspect_ratio: &'a str,
}

#[derive(Deserialize)]
struct ImagenResponse {
    predictions: Vec<ImagenPrediction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImagenPrediction {
    bytes_base64_encoded: String,
}

#[async_trait]
impl CommandHandler for DreamCommand {
    async fn handle_command(
        &self,
        command_handler_data: CommandHandlerData<'_>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    ) {
        let interaction_client = command_handler_data.interaction_client;
        let reqwest_client = command_handler_data.reqwest_client;

        let prompt = &self.prompt;

        let aspect_ratio = match self.aspect_ratio.as_ref() {
            Some(r) => r.value(),
            None => "1:1",
        };

        let dream_params = DreamParams {
            prompt,
            aspect_ratio,
        };

        interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &(InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![EmbedBuilder::new()
                            .title("Dreaming")
                            .color(0x673ab7)
                            .field(EmbedFieldBuilder::new("Prompt", prompt))
                            .field(details_field(&dream_params))
                            .build()]),
                        ..Default::default()
                    }),
                }),
            )
            .await
            .ok();

        match dream(&reqwest_client, &dream_params).await {
            Ok((image, key_used)) => {
                let filename = "image.png".to_string();
                let footer_text = format!("Model: Imagen | Key: {}", key_used);
                let footer = EmbedFooterBuilder::new(footer_text).build();

                interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[EmbedBuilder::new()
                        .title("Completed")
                        .color(0x43a047)
                        .field(EmbedFieldBuilder::new("Prompt", prompt))
                        .field(details_field(&dream_params))
                        .footer(footer)
                        .image(ImageSource::attachment(&filename).unwrap())
                        .build()]))
                    .await
                    .ok();

                interaction_client
                    .update_response(interaction_token)
                    .attachments(&[Attachment::from_bytes(filename, image, 1)])
                    .await
                    .ok();
            }
            Err(e) => {
                interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[EmbedBuilder::new()
                        .title("Failed")
                        .color(0xe53935)
                        .field(EmbedFieldBuilder::new("Prompt", prompt))
                        .field(details_field(&dream_params))
                        .field(EmbedFieldBuilder::new(
                            "Error",
                            format!("```\n{}\n```", e.message),
                        ))
                        .build()]))
                    .await
                    .ok();
            }
        };
    }
}

struct DreamError {
    message: String,
}

fn details_field(dream_params: &DreamParams) -> EmbedFieldBuilder {
    EmbedFieldBuilder::new("Aspect Ratio", dream_params.aspect_ratio.to_string())
}

async fn dream(
    reqwest_client: &Client,
    dream_params: &DreamParams<'_>,
) -> Result<(Vec<u8>, &'static str), DreamError> {
    let prompt = dream_params.prompt;
    let aspect_ratio = dream_params.aspect_ratio;

    let request_body = json!({
        "instances": [
            { "prompt": prompt }
        ],
        "parameters": {
            "sampleCount": 1,
            "aspectRatio": aspect_ratio,
        }
    });

    let api_url =
        "https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-generate-001:predict";

    let (text, key_name) = post_generative_ai(reqwest_client, api_url, &request_body)
        .await
        .map_err(|e: GoogleAiError| DreamError { message: e.message })?;

    let imagen_response: ImagenResponse = serde_json::from_str(&text).map_err(|e| DreamError {
        message: format!(
            "JSON Parse Error with key {}: {}\nResponse: {}",
            key_name, e, text
        ),
    })?;

    let base64_image = imagen_response
        .predictions
        .get(0)
        .ok_or(DreamError {
            message: "No predictions in response".to_string(),
        })?
        .bytes_base64_encoded
        .clone();

    let image = general_purpose::STANDARD
        .decode(base64_image)
        .map_err(|e| DreamError {
            message: format!("Base64 Decode Error: {}", e),
        })?;

    Ok((image, key_name))
}