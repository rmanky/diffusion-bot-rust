use std::sync::Arc;

use twilight_http::Client;
use twilight_model::{
    application::{
        command::Command,
        interaction::{Interaction, InteractionData},
    },
    id::{marker::ApplicationMarker, Id},
};

mod dream;

pub fn command_definitions() -> Vec<Command> {
    vec![dream::command_definition()]
}

pub async fn handle_interaction(
    interaction: Interaction,
    application_id: Id<ApplicationMarker>,
    http: Arc<Client>,
) {
    let interaction_client = http.interaction(application_id);
    match interaction.data.clone() {
        Some(data) => match data {
            InteractionData::ApplicationCommand(cmd) => match cmd.name.as_str() {
                "dream" => dream::handle_command(interaction_client, interaction, cmd).await,
                _ => {}
            },
            _ => todo!(),
        },
        None => todo!(),
    }
}
