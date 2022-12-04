use async_trait::async_trait;
use reqwest::Client as ReqwestClient;
use twilight_http::{client::InteractionClient, Client as TwilightClient};
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::{
        command::Command,
        interaction::{Interaction, InteractionData},
    },
    id::{
        marker::{ApplicationMarker, InteractionMarker},
        Id,
    },
};

use self::{dream::DreamCommand, info::InfoCommand};

mod dream;
mod info;

#[async_trait]
pub trait CommandHandler {
    async fn handle_command(
        &self,
        interaction_client: InteractionClient<'_>,
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
        vec![DreamCommand::create_command().into()]
    }

    async fn handle_interaction(
        &self,
        interaction: Interaction,
        application_id: Id<ApplicationMarker>,
    ) {
        let interaction_client = self.twilight_client.interaction(application_id);

        if let Some(InteractionData::ApplicationCommand(command_data)) = interaction.data {
            match command_data.name.as_str() {
                "dream" => {
                    if let Ok(dream_command) =
                        DreamCommand::from_interaction((*command_data).into())
                    {
                        dream_command
                            .handle_command(interaction_client, interaction.id, &interaction.token)
                            .await
                    }
                }
                "info" => {
                    if let Ok(info_command) = InfoCommand::from_interaction((*command_data).into())
                    {
                        info_command
                            .handle_command(interaction_client, interaction.id, &interaction.token)
                            .await
                    }
                }
                &_ => {}
            }
        }
    }
}