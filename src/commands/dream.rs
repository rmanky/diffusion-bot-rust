use std::time::Duration;

use tokio::time::sleep;
use twilight_http::client::InteractionClient;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::{CommandBuilder, StringBuilder};
use twilight_util::builder::embed::EmbedBuilder;

pub fn command_definition() -> Command {
    CommandBuilder::new(
        "dream",
        "Create an image with Stable Diffusion",
        CommandType::ChatInput,
    )
    .option(StringBuilder::new("prompt", "Prompt for the model to generate").required(true))
    .build()
}

pub async fn handle_command(
    http: InteractionClient<'_>,
    interaction: Interaction,
    _cmd: Box<CommandData>,
) {
    http.create_response(
        interaction.id,
        &interaction.token,
        &InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                embeds: Some(vec![EmbedBuilder::new()
                    .title("Dream")
                    .description("Life is but a dream")
                    .build()]),
                ..Default::default()
            }),
        },
    )
    .exec()
    .await
    .ok();

    sleep(Duration::from_secs(3)).await;

    http.update_response(&interaction.token)
        .embeds(Some(&[EmbedBuilder::new()
            .title("Dream")
            .description("Or is it?")
            .build()]))
        .unwrap()
        .exec()
        .await
        .ok();
}
