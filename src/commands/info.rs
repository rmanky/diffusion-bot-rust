use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

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
                            .title("Stable Diffusion, Now Extra Large!")
                            .image(ImageSource::url("https://mma.prnewswire.com/media/1921904/Stability_AI_logo_Logo.jpg").unwrap())
                            .description(
                            "
                            Why am I doing this at 11:33 PM on a Friday?\n \
                            - Replicate is dead, Stablity.AI is the future!\n\
                            - `/dream` now uses Stable Diffusion XL, the next generation in diffusion technology.\n\
                            - There are now over 15 styles to choose from.\n\
                            - I'm probably going to run out of credits at some point.\n\
                            ")
                            .color(0xC2185B)
                            .build()]),
                        ..Default::default()
                    }),
                },
            )
            .await
            .ok();
    }
}
