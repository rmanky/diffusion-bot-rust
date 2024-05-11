use async_trait::async_trait;
use reqwest::Client as ReqwestClient;
use twilight_http::{client::InteractionClient, Client as TwilightClient};
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::{
        command::Command,
        interaction::{Interaction, InteractionData},
    },
    channel::Channel,
    id::{
        marker::{ApplicationMarker, InteractionMarker},
        Id,
    },
};

use self::{chat::ChatCommand, dream::DreamCommand, horde::HordeCommand, info::InfoCommand};

mod chat;
mod dream;
mod horde;
mod info;

pub struct CommandHandlerData<'a> {
    pub channel: Channel,
    pub reqwest_client: ReqwestClient,
    pub interaction_client: InteractionClient<'a>,
}

#[async_trait]
pub trait CommandHandler {
    async fn handle_command(
        &self,
        command_handler_data: CommandHandlerData<'_>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    );
}

pub struct CommandDelegateData {
    pub reqwest_client: ReqwestClient,
    pub twilight_client: TwilightClient,
}

#[async_trait]
pub trait CommandDelegate {
    fn command_definitions(&self) -> Vec<Command>;
    async fn handle_interaction(
        &self,
        interaction: Interaction,
        application_id: Id<ApplicationMarker>,
    );
}

#[async_trait]
impl CommandDelegate for CommandDelegateData {
    fn command_definitions(&self) -> Vec<Command> {
        [
            HordeCommand::create_command(),
            DreamCommand::create_command(),
            InfoCommand::create_command(),
            ChatCommand::create_command(),
        ]
        .map(std::convert::Into::into)
        .to_vec()
    }

    async fn handle_interaction(
        &self,
        interaction: Interaction,
        application_id: Id<ApplicationMarker>,
    ) {
        if let Some(InteractionData::ApplicationCommand(command_data)) = interaction.data {
            let channel = match interaction.channel_id {
                Some(v) => match self.twilight_client.channel(v).await {
                    Ok(c) => match c.model().await {
                        Ok(m) => m,
                        Err(_) => return,
                    },
                    Err(_) => return,
                },
                None => return,
            };

            let command_handler_data = CommandHandlerData {
                channel,
                interaction_client: self.twilight_client.interaction(application_id),
                reqwest_client: self.reqwest_client.to_owned(),
            };

            match command_data.name.as_str() {
                "horde" => {
                    if let Ok(horde_command) =
                        HordeCommand::from_interaction((*command_data).into())
                    {
                        horde_command
                            .handle_command(
                                command_handler_data,
                                interaction.id,
                                &interaction.token,
                            )
                            .await
                    }
                }
                "dream" => {
                    if let Ok(dream_command) =
                        DreamCommand::from_interaction((*command_data).into())
                    {
                        dream_command
                            .handle_command(
                                command_handler_data,
                                interaction.id,
                                &interaction.token,
                            )
                            .await
                    }
                }
                "info" => {
                    if let Ok(info_command) = InfoCommand::from_interaction((*command_data).into())
                    {
                        info_command
                            .handle_command(
                                command_handler_data,
                                interaction.id,
                                &interaction.token,
                            )
                            .await
                    }
                }
                "chat" => {
                    if let Ok(chat_command) = ChatCommand::from_interaction((*command_data).into())
                    {
                        chat_command
                            .handle_command(
                                command_handler_data,
                                interaction.id,
                                &interaction.token,
                            )
                            .await
                    }
                }
                &_ => {}
            }
        }
    }
}
