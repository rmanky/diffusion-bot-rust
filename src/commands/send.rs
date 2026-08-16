use async_trait::async_trait;
use solana_sdk::signer::Signer;
use twilight_interactions::command::{CommandModel, CreateCommand, ResolvedUser};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::EmbedFieldBuilder;

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
                send_error(
                    &command_handler_data,
                    interaction_id,
                    interaction_token,
                    "Could not identify sender",
                )
                .await;
                return;
            }
        };

        let recipient_id = self.recipient.resolved.id.get();

        if self.amount <= 0 {
            send_error(
                &command_handler_data,
                interaction_id,
                interaction_token,
                "Amount must be greater than 0",
            )
            .await;
            return;
        }

        if sender_id == recipient_id {
            send_error(
                &command_handler_data,
                interaction_id,
                interaction_token,
                "You can't send tokens to yourself!",
            )
            .await;
            return;
        }

        let solana_client = match &command_handler_data.solana_client {
            Some(c) => c,
            None => {
                send_error(
                    &command_handler_data,
                    interaction_id,
                    interaction_token,
                    "Solana is not configured",
                )
                .await;
                return;
            }
        };

        let wallet_deriver = match &command_handler_data.wallet_deriver {
            Some(d) => d,
            None => {
                send_error(
                    &command_handler_data,
                    interaction_id,
                    interaction_token,
                    "Wallet system not available",
                )
                .await;
                return;
            }
        };

        let sender_keypair = wallet_deriver.get_keypair(sender_id).await;

        let sender_pubkey = sender_keypair.pubkey();
        let sender_balance = solana_client
            .get_token_balance(&sender_pubkey)
            .await
            .unwrap_or(0);

        let raw_amount = (self.amount as u64) * 1_000_000_000;

        if sender_balance < raw_amount {
            let formatted_balance = sender_balance / 1_000_000_000;
            send_error(
                &command_handler_data,
                interaction_id,
                interaction_token,
                &format!(
                    "Insufficient balance. You have {} tokens.",
                    formatted_balance
                ),
            )
            .await;
            return;
        }

        let recipient_pubkey = if self.recipient.resolved.bot {
            if recipient_id == command_handler_data.bot_user_id {
                solana_client.fee_payer_pubkey()
            } else {
                send_error(
                    &command_handler_data,
                    interaction_id,
                    interaction_token,
                    "You can't send tokens to other bots!",
                )
                .await;
                return;
            }
        } else {
            wallet_deriver.get_pubkey(recipient_id).await
        };

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

        match solana_client
            .transfer_tokens(&sender_keypair, &recipient_pubkey, raw_amount)
            .await
        {
            Ok(signature) => {
                let solscan_url = format!("https://solscan.io/tx/{}", signature);

                let success_embed = embed::success()
                    .title("Transfer Complete")
                    .field(EmbedFieldBuilder::new("From", format!("<@{}>", sender_id)).inline())
                    .field(EmbedFieldBuilder::new("To", format!("<@{}>", recipient_id)).inline())
                    .field(EmbedFieldBuilder::new(
                        "Amount",
                        format!("**{}** tokens", self.amount),
                    ))
                    .field(EmbedFieldBuilder::new(
                        "Transaction",
                        format!("[View on Solscan]({})", solscan_url),
                    ))
                    .build();

                command_handler_data
                    .interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[success_embed]))
                    .await
                    .ok();
            }
            Err(e) => {
                log::error!("Transfer failed: {}", e);

                command_handler_data
                    .interaction_client
                    .update_response(interaction_token)
                    .embeds(Some(&[
                        embed::failure(format!("Transfer failed: {}", e)).build()
                    ]))
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
                    embeds: Some(vec![embed::failure(message).build()]),
                    ..Default::default()
                }),
            },
        )
        .await
        .ok();
}
