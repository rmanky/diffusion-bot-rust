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
                            .image(ImageSource::url("https://miro.medium.com/v2/resize:fit:1400/1*BdM6bCmvyFJKzkhbQ3Uccw.jpeg").unwrap())
                            .description(
                            "
                            Welcome to the future, again!\n\
                            - SDXL 0.8 is dead, long live SDXL 1.0!\n\
                            - Added options for aspect ratio!\n\
                            - I haven't even tested this yet!\n\
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
