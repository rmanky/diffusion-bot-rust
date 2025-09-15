use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::ImageSource;

use super::{CommandHandler, CommandHandlerData};
use crate::embed;

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
                        embeds: Some(vec![embed::info()
                            .title("Image(n) A World")
                            .image(ImageSource::url("https://i.ibb.co/yB8nRPKc/image.png").unwrap())
                            .description(
                                "`/dream` is now Imagen 4. It's like Imagen 3, but better.`",
                            )
                            .build()]),
                        ..Default::default()
                    }),
                }),
            )
            .await
            .ok();
    }
}
