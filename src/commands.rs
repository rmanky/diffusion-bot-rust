use async_trait::async_trait;
use reqwest::Client as ReqwestClient;
use twilight_http::Client as TwilightClient;
use twilight_model::{
    application::{
        command::Command,
        interaction::{Interaction, InteractionData},
    },
    id::{marker::ApplicationMarker, Id},
};

mod dream;
mod info;

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
        vec![dream::command_definition(), info::command_definition()]
    }

    async fn handle_interaction(
        &self,
        interaction: Interaction,
        application_id: Id<ApplicationMarker>,
    ) {
        let interaction_client = self.twilight_client.interaction(application_id);
        if let Some(ref data) = interaction.data {
            match data {
                InteractionData::ApplicationCommand(ref cmd) => match cmd.name.as_str() {
                    "dream" => dream::handle_command(interaction_client, &interaction, cmd).await,
                    "info" => info::handle_command(interaction_client, &interaction, cmd).await,
                    _ => {}
                },
                &_ => {}
            }
        }
    }
}
