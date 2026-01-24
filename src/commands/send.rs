use async_trait::async_trait;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use twilight_interactions::command::{CommandModel, CreateCommand, ResolvedUser};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;

use super::{CommandHandler, CommandHandlerData};
use crate::utils::embed;

#[derive(CommandModel, CreateCommand)]
#[command(name = "send", desc = "Send tokens to another user")]
pub struct SendCommand {
    /// The user to send tokens to
    pub recipient: ResolvedUser,
    /// Amount of tokens to send
    pub amount: i64,
}

#[async_trait]
impl CommandHandler for SendCommand {
    async fn handle_command(
        &self,
        command_handler_data: CommandHandlerData<'_>,
        interaction_id: Id<InteractionMarker>,
        interaction_token: &'_ str,
    ) {
        let sender_id = match &command_handler_data.invoking_user {
            Some(user) => user.id.get(),
            None => {
                send_error(&command_handler_data, interaction_id, interaction_token, "Could not identify sender").await;
                return;
            }
        };

        let recipient_id = self.recipient.resolved.id.get();

        // Validate amount
        if self.amount <= 0 {
            send_error(&command_handler_data, interaction_id, interaction_token, "Amount must be greater than 0").await;
            return;
        }

        // Can't send to yourself
        if sender_id == recipient_id {
            send_error(&command_handler_data, interaction_id, interaction_token, "You can't send tokens to yourself!").await;
            return;
        }

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

        // Get sender wallet (must exist)
        let sender_keypair_bytes = match wallet_cache.get_keypair_bytes(sender_id).await {
            Some(bytes) => bytes,
            None => {
                send_error(&command_handler_data, interaction_id, interaction_token, "You don't have a wallet yet. Use `/balance` first!").await;
                return;
            }
        };

        let sender_keypair = match Keypair::try_from(sender_keypair_bytes.as_slice()) {
            Ok(kp) => kp,
            Err(_) => {
                send_error(&command_handler_data, interaction_id, interaction_token, "Failed to load your wallet").await;
                return;
            }
        };

        // Check sender balance (async)
        let sender_pubkey = sender_keypair.pubkey();
        let sender_balance = solana_client.get_token_balance(&sender_pubkey).await.unwrap_or(0);
        
        // Convert amount to raw units (9 decimals)
        let raw_amount = (self.amount as u64) * 1_000_000_000;
        
        if sender_balance < raw_amount {
            let formatted_balance = sender_balance / 1_000_000_000;
            send_error(
                &command_handler_data,
                interaction_id,
                interaction_token,
                &format!("Insufficient balance. You have {} tokens.", formatted_balance)
            ).await;
            return;
        }

        // Get or create recipient wallet
        let (_, recipient_pubkey, _) = wallet_cache.get_or_create_wallet(recipient_id).await;

        // Defer reply since transfer takes time
        command_handler_data
            .interaction_client
            .create_response(
                interaction_id,
                interaction_token,
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: None,
                },
            )
            .await
            .ok();

        // Execute transfer (async)
        match solana_client.transfer_tokens(&sender_keypair, &recipient_pubkey, raw_amount).await {
            Ok(signature) => {
                let solscan_url = format!("https://solscan.io/tx/{}", signature);
                
                command_handler_data
                    .interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[embed::info()
                        .title("✅ Transfer Complete")
                        .description(&format!(
                            "Sent **{}** tokens to <@{}>\n\n[View on Solscan]({})",
                            self.amount, recipient_id, solscan_url
                        ))
                        .build()]))
                    .await
                    .ok();
            }
            Err(e) => {
                log::error!("Transfer failed: {}", e);
                
                command_handler_data
                    .interaction_client
                    .update_response(interaction_token)
                    .content(Some(&format!("❌ Transfer failed: {}", e)))
                    .await
                    .ok();
            }
        }
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
