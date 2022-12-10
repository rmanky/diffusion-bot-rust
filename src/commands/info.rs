use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, ImageSource};

use super::{CommandHandler, CommandHandlerData};

#[derive(CommandModel, CreateCommand)]
#[command(name = "info", desc = "Display general information about the bot")]
pub struct InfoCommand {}

#[async_trait]
impl CommandHandler for InfoCommand {
    async fn handle_command(
        &self,
        command_handler_data: CommandHandlerData<'_>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    ) {
        command_handler_data
            .interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![EmbedBuilder::new()
                            .title("Information")
                            .image(ImageSource::url("https://media4.giphy.com/media/2dK0W3oUksQk0Xz8OK/giphy.gif").unwrap())
                            .description("The bot can now generate images from multiple Stable Diffusion based models. You do not have to append the style tag to your prompt, simply select a `model` when running the command.")
                            .color(0x1E88E5)
                            .field(EmbedFieldBuilder::new("Stable Diffusion v1.5", "The not-latest but still greatest version of Stable Diffusion."))
                            .field(EmbedFieldBuilder::new("Elden Ring Diffusion", "Stable Diffusion fine tuned on the game art from Elden Ring."))
                            .field(EmbedFieldBuilder::new("OpenJourney", "Stable Diffusion fine tuned on Midjourney v4."))
                            .field(EmbedFieldBuilder::new("Arcane Diffusion", "Stable Diffusion fine tuned on Arcane."))
                            .build()]),
                        ..Default::default()
                    }),
                },
            )
            .await
            .ok();
    }
}
