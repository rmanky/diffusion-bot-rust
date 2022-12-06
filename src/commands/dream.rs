use std::env;

use async_trait::async_trait;
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
enum TimeUnit {
    #[option(
        name = "Stable Diffusion 1.5",
        value = "runwayml/stable-diffusion-v1-5"
    )]
    SD15,
    #[option(
        name = "Elden Ring Diffusion [elden ring style]",
        value = "nitrosocke/elden-ring-diffusion"
    )]
    Elden,
    #[option(name = "Anything V3", value = "Linaqruf/anything-v3.0")]
    Anything,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "dream", desc = "Create an image with Stable Diffusion")]
pub struct DreamCommand {
    #[command(desc = "Prompt for the model to generate")]
    prompt: String,
    #[command(desc = "Pre-trained weights for the model to use")]
    model: Option<TimeUnit>,
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

        interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![EmbedBuilder::new()
                            .title("Processing")
                            .color(0x5E35B1)
                            .field(EmbedFieldBuilder::new("Prompt", &self.prompt))
                            .build()]),
                        ..Default::default()
                    }),
                },
            )
            .await
            .ok();

        let result = reqwest_client
            .post(format!(
                "https://api-inference.huggingface.co/models/{}",
                &self.model.as_ref().unwrap_or(&TimeUnit::SD15).value()
            ))
            .header(
                "Authorization",
                format!("Bearer {}", env::var("HUGGINGFACE_TOKEN").unwrap()),
            )
            .header("x-use-cache", "false")
            .body(format!("{{\"inputs\":\"{}\"}}", &self.prompt))
            .send()
            .await
            .ok();

        if let Some(image) = result {
            if let Ok(image_bytes) = image.bytes().await {
                interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[EmbedBuilder::new()
                        .title("Completed")
                        .color(0x43A047)
                        .field(EmbedFieldBuilder::new("Prompt", &self.prompt))
                        .image(ImageSource::attachment("image.png").unwrap())
                        .build()]))
                    .unwrap()
                    .attachments(&[Attachment::from_bytes(
                        "image.png".to_string(),
                        image_bytes.to_vec(),
                        0,
                    )])
                    .unwrap()
                    .await
                    .ok();

                return;
            }
        }

        interaction_client
                .update_response(interaction_token)
                .embeds(Some(&[EmbedBuilder::new()
                    .title("Failed")
                    .color(0xE53935)
                    .field(EmbedFieldBuilder::new("Prompt", &self.prompt))
                    .field(EmbedFieldBuilder::new(
                        "Error",
                        "An error has occurred, but Rust is hard and I haven't figured out error handling yet"
                    ))
                    .build()]))
                .unwrap()
                .await
                .ok();
    }
}
