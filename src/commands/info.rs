use twilight_http::client::InteractionClient;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::CommandBuilder;
use twilight_util::builder::embed::EmbedBuilder;

pub fn command_definition() -> Command {
    CommandBuilder::new(
        "info",
        "Display general information about the bot",
        CommandType::ChatInput,
    )
    .build()
}

pub async fn handle_command(
    http: InteractionClient<'_>,
    interaction: &Interaction,
    _cmd: &CommandData,
) {
    http.create_response(
        interaction.id,
        &interaction.token,
        &InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                embeds: Some(vec![EmbedBuilder::new()
                    .title("Info")
                    .description("This is information about the bot")
                    .build()]),
                ..Default::default()
            }),
        },
    )
    .exec()
    .await
    .ok();
}
