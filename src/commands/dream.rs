use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::{EmbedFieldBuilder, EmbedFooterBuilder, ImageSource};

use super::{CommandHandler, CommandHandlerData};
use crate::utils::embed;
use crate::utils::google_ai::{
    generate_image, GoogleAiError, GOOGLE_API_FREE_KEY, GOOGLE_API_PAID_KEY,
};

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
#[command(name = "dream", desc = "Create an image with Gemini 3.1 Flash Lite")]
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
                        embeds: Some(vec![embed::pending("Dreaming", "")
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
            Ok((image, tier_used)) => {
                let filename = "image.png".to_string();
                let footer_text =
                    format!("Model: gemini-3.1-flash-lite-image | Tier: {}", tier_used);
                let footer = EmbedFooterBuilder::new(footer_text).build();

                interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[embed::success()
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
                    .embeds(Some(&[embed::failure(&e.message)
                        .field(EmbedFieldBuilder::new("Prompt", prompt))
                        .field(details_field(&dream_params))
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

    generate_image(
        reqwest_client,
        "gemini-3.1-flash-lite-image",
        vec![json!({ "text": prompt })],
        Some(aspect_ratio),
        &[GOOGLE_API_FREE_KEY, GOOGLE_API_PAID_KEY],
    )
    .await
    .map_err(|e: GoogleAiError| DreamError { message: e.message })
}
