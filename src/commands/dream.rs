use std::env;

use async_trait::async_trait;
use reqwest::{multipart, Client, StatusCode};
use twilight_http::client::InteractionClient;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, ImageSource};

use super::{CommandHandler, CommandHandlerData};

#[derive(CommandOption, CreateOption)]
enum StableRatio {
    #[option(name = "square", value = "square")]
    Square,
    #[option(name = "portrait", value = "portrait")]
    Portrait,
    #[option(name = "landscape", value = "landscape")]
    Landscape,
}

#[derive(CommandOption, CreateOption)]
enum Model {
    #[option(name = "SD3", value = "sd3")]
    SD3,
    #[option(name = "SD3 Turbo", value = "sd3-turbo")]
    SD3Turbo,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "dream", desc = "Create an image with Stable Diffusion")]
pub struct DreamCommand {
    /// Prompt for the model to generate.
    prompt: String,
    /// Select an aspect ratio. Uses 1:1 by default.
    aspect_ratio: Option<StableRatio>,
    /// Select a model to use. Uses SD3 Turbo by default.
    model: Option<Model>,
}

struct DreamParams<'a> {
    prompt: &'a str,
    aspect_ratio: &'a str,
    model: &'a str,
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
            Some(r) => match r {
                StableRatio::Square => "1:1",
                StableRatio::Portrait => "9:16",
                StableRatio::Landscape => "16:9",
            },
            None => "1:1",
        };

        let model = match self.model.as_ref() {
            Some(m) => m.value(),
            None => "sd3-turbo",
        };

        let dream_params = DreamParams {
            prompt,
            aspect_ratio,
            model,
        };

        interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![EmbedBuilder::new()
                            .title("Dreaming")
                            .color(0x673AB7)
                            .field(EmbedFieldBuilder::new("Prompt", prompt))
                            .field(details_field(&dream_params))
                            .build()]),
                        ..Default::default()
                    }),
                },
            )
            .await
            .ok();

        let e = match dream(
            &reqwest_client,
            &dream_params,
            &interaction_client,
            interaction_token,
        )
        .await
        {
            Ok(_) => return,
            Err(e) => e,
        };

        interaction_client
            .update_response(interaction_token)
            .embeds(Some(&[EmbedBuilder::new()
                .title("Failed")
                .color(0xE53935)
                .field(EmbedFieldBuilder::new("Prompt", prompt))
                .field(details_field(&dream_params))
                .field(EmbedFieldBuilder::new(
                    "Error",
                    format!("```\n{}\n```", e.message),
                ))
                .build()]))
            .unwrap()
            .await
            .ok();
    }
}

struct DreamError {
    message: String,
}

fn details_field(dream_params: &DreamParams) -> EmbedFieldBuilder {
    EmbedFieldBuilder::new(
        "Model, Aspect Ratio",
        format!("{}, {}", dream_params.model, dream_params.aspect_ratio),
    )
}

async fn dream(
    reqwest_client: &Client,
    dream_params: &DreamParams<'_>,
    interaction_client: &InteractionClient<'_>,
    interaction_token: &str,
) -> Result<(), DreamError> {
    let prompt = dream_params.prompt;
    let aspect_ratio = dream_params.aspect_ratio;
    let model = dream_params.model;
    let form = multipart::Form::new()
        .text("prompt", prompt.to_string())
        .text("aspect_ratio", aspect_ratio.to_string())
        .text("mode", "text-to-image")
        .text("model", model.to_string())
        .text("output_format", "jpeg");

    let submit_request = reqwest_client
        .post("https://api.stability.ai/v2beta/stable-image/generate/sd3")
        .header(
            "Authorization",
            format!("Bearer {}", env::var("STABLE_KEY").unwrap()),
        )
        .header("Accept", "image/*")
        .multipart(form)
        .send()
        .await;

    let response = match submit_request {
        Ok(r) => r,
        Err(e) => {
            return Err(DreamError {
                message: format!("{:#?}", e),
            })
        }
    };

    let status_code = response.status();
    if status_code != StatusCode::OK {
        return Err(DreamError {
            message: format!(
                "Status Code: {}\n{:#?}",
                status_code,
                response
                    .text()
                    .await
                    .unwrap_or("Failed to parse response bytes".to_string())
            ),
        });
    }

    let image = match response.bytes().await {
        Ok(img) => img.to_vec(),
        Err(e) => {
            return Err(DreamError {
                message: format!("{:#?}", e),
            })
        }
    };

    let filename = "image.jpeg".to_string();

    interaction_client
        .update_response(interaction_token)
        .embeds(Some(&[EmbedBuilder::new()
            .title("Completed")
            .color(0x43A047)
            .field(EmbedFieldBuilder::new("Prompt", prompt))
            .field(details_field(&dream_params))
            .image(ImageSource::attachment(&filename).unwrap())
            .build()]))
        .unwrap()
        .await
        .ok();

    interaction_client
        .update_response(interaction_token)
        .attachments(&[Attachment::from_bytes(filename, image, 1)])
        .unwrap()
        .await
        .ok();

    return Ok(());
}
