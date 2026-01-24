use async_trait::async_trait;
use reqwest::Client as ReqwestClient;
use std::sync::Arc;
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
    user::User,
};

use crate::solana::client::SolanaClient;
use crate::solana::storage::WalletCache;

use self::{
    balance::BalanceCommand, chat::ChatCommand, dream::DreamCommand, horde::HordeCommand,
    info::InfoCommand, nano::NanoCommand, send::SendCommand, stats::StatsCommand,
};

mod balance;
mod chat;
mod dream;
mod horde;
mod info;
mod nano;
mod send;
mod stats;

pub struct CommandHandlerData<'a> {
    pub channel: Channel,
    pub reqwest_client: ReqwestClient,
    pub interaction_client: InteractionClient<'a>,
    pub twilight_client: &'a TwilightClient,
    pub invoking_user: Option<User>,
    pub solana_client: Option<Arc<SolanaClient>>,
    pub wallet_cache: Option<Arc<WalletCache>>,
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
    pub solana_client: Option<Arc<SolanaClient>>,
    pub wallet_cache: Option<Arc<WalletCache>>,
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
            NanoCommand::create_command(),
            StatsCommand::create_command(),
            BalanceCommand::create_command(),
            SendCommand::create_command(),
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
            let channel = match interaction.channel {
                Some(c) => c,
                None => {
                    log::warn!("Received a command from an unknown channel.");
                    return;
                }
            };

            let command_handler_data = CommandHandlerData {
                channel,
                interaction_client: self.twilight_client.interaction(application_id),
                reqwest_client: self.reqwest_client.to_owned(),
                twilight_client: &self.twilight_client,
                invoking_user: interaction.member.as_ref().and_then(|m| m.user.clone()),
                solana_client: self.solana_client.clone(),
                wallet_cache: self.wallet_cache.clone(),
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
                "nano" => {
                    if let Ok(nano_command) = NanoCommand::from_interaction((*command_data).into())
                    {
                        nano_command
                            .handle_command(
                                command_handler_data,
                                interaction.id,
                                &interaction.token,
                            )
                            .await
                    }
                }
                "stats" => {
                    if let Ok(stats_command) =
                        StatsCommand::from_interaction((*command_data).into())
                    {
                        stats_command
                            .handle_command(
                                command_handler_data,
                                interaction.id,
                                &interaction.token,
                            )
                            .await
                    }
                }
                "balance" => {
                    if let Ok(balance_command) =
                        BalanceCommand::from_interaction((*command_data).into())
                    {
                        balance_command
                            .handle_command(
                                command_handler_data,
                                interaction.id,
                                &interaction.token,
                            )
                            .await
                    }
                }
                "send" => {
                    if let Ok(send_command) =
                        SendCommand::from_interaction((*command_data).into())
                    {
                        send_command
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
