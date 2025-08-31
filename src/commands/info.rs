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
                &(InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![EmbedBuilder::new()
                            .title("Nano-Banana")
                            .image(ImageSource::url("https://i.ibb.co/WvRg3ccW/image.png").unwrap())
                            .description(
                                "The latest and greatest from the Big G, now available via `/nano`.\n_this costs me $0.04 per image so plz don't spam okay thx bye_",
                            )
                            .color(0xc2185b)
                            .build()]),
                        ..Default::default()
                    }),
                }),
            )
            .await
            .ok();
    }
}
