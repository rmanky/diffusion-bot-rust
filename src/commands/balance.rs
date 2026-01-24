use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;

use super::{CommandHandler, CommandHandlerData};
use crate::utils::embed;

#[derive(CommandOption, CreateOption)]
pub enum BalanceScope {
    #[option(name = "all", value = "all")]
    All,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "balance", desc = "Check your token balance")]
pub struct BalanceCommand {
    /// Show all users' balances
    pub scope: Option<BalanceScope>,
}

#[async_trait]
impl CommandHandler for BalanceCommand {
    async fn handle_command(
        &self,
        command_handler_data: CommandHandlerData<'_>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    ) {
        let user_id = match &command_handler_data.invoking_user {
            Some(user) => user.id.get(),
            None => {
                send_error(&command_handler_data, interaction_id, interaction_token, "Could not identify user").await;
                return;
            }
        };

        // Get shared Solana client
        let solana_client = match &command_handler_data.solana_client {
            Some(c) => c,
            None => {
                send_error(&command_handler_data, interaction_id, interaction_token, "Solana is not configured").await;
                return;
            }
        };

        // Get wallet cache
        let wallet_cache = match &command_handler_data.wallet_cache {
            Some(c) => c,
            None => {
                send_error(&command_handler_data, interaction_id, interaction_token, "Wallet cache not available").await;
                return;
            }
        };

        let response_content = match &self.scope {
            Some(BalanceScope::All) => {
                // Show all balances
                let mut lines = vec!["**Token Balances**\n".to_string()];
                
                let all_pubkeys = wallet_cache.get_all_pubkeys().await;
                
                for (discord_id, pubkey) in all_pubkeys {
                    let balance = solana_client.get_token_balance(&pubkey).await.unwrap_or(0);
                    let formatted_balance = format_balance(balance);
                    lines.push(format!("<@{}>: **{}**", discord_id, formatted_balance));
                }

                if lines.len() == 1 {
                    lines.push("No wallets registered yet.".to_string());
                }

                lines.join("\n")
            }
            None => {
                // Show just the caller's balance - get or create wallet
                let (_, pubkey, _) = wallet_cache.get_or_create_wallet(user_id).await;
                let balance = solana_client.get_token_balance(&pubkey).await.unwrap_or(0);
                let formatted_balance = format_balance(balance);
                format!("Your balance: **{}** tokens", formatted_balance)
            }
        };

        command_handler_data
            .interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![embed::info()
                            .title("💰 Balance")
                            .description(&response_content)
                            .build()]),
                        ..Default::default()
                    }),
                },
            )
            .await
            .ok();
    }
}

/// Format token balance (assuming 9 decimals like most SPL tokens)
fn format_balance(raw_amount: u64) -> String {
    // Assuming 9 decimals (standard for SPL tokens)
    let whole = raw_amount / 1_000_000_000;
    let frac = raw_amount % 1_000_000_000;
    
    if frac == 0 {
        format!("{}", whole)
    } else {
        // Show up to 2 decimal places
        let frac_str = format!("{:09}", frac);
        let trimmed = frac_str.trim_end_matches('0');
        let display_frac = if trimmed.len() > 2 { &trimmed[..2] } else { trimmed };
        format!("{}.{}", whole, display_frac)
    }
}

async fn send_error(
    command_handler_data: &CommandHandlerData<'_>,
    interaction_id: Id<InteractionMarker>,
    interaction_token: &str,
    message: &str,
) {
    command_handler_data
        .interaction_client
        .create_response(
            interaction_id,
            interaction_token,
            &InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    content: Some(format!("❌ {}", message)),
                    ..Default::default()
                }),
            },
        )
        .await
        .ok();
}
