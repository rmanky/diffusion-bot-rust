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
                            .title("Stable Diffusion 3")
                            .image(ImageSource::url("https://i.imgur.com/waJD7VV.png").unwrap())
                            .description("
                            We are pleased to announce the availability of Stable Diffusion 3 and Stable Diffusion 3 Turbo on ~~the Stability AI Developer Platform API~~ _Diffusion Bot_.\n
                            As revealed in the Stable Diffusion 3 [research paper](https://stability.ai/news/stable-diffusion-3-research-paper), this model is equal to or outperforms state-of-the-art text-to-image generation systems such as DALL-E 3 and Midjourney v6 in typography and prompt adherence, based on human preference evaluations. 
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
