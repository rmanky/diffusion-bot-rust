use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::InteractionMarker;
use twilight_model::id::Id;
use twilight_model::util::ImageHash;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, ImageSource};

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
        let user = match &command_handler_data.invoking_user {
            Some(user) => user,
            None => {
                send_error(
                    &command_handler_data,
                    interaction_id,
                    interaction_token,
                    "Could not identify user",
                )
                .await;
                return;
            }
        };

        let user_id = user.id.get();

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

        let response_embed = match &self.scope {
            Some(BalanceScope::All) => {
                let mut lines = Vec::new();

                let bot_id = command_handler_data.bot_user_id;
                let treasury_pubkey = solana_client.fee_payer_pubkey();
                let treasury_balance = solana_client
                    .get_token_balance(&treasury_pubkey)
                    .await
                    .unwrap_or(0);
                lines.push(format!(
                    "- <@{}>: **{}** tokens",
                    bot_id,
                    format_balance(treasury_balance)
                ));

                for (discord_id, pubkey) in wallet_deriver.get_all_cached().await {
                    let balance = solana_client.get_token_balance(&pubkey).await.unwrap_or(0);
                    lines.push(format!(
                        "- <@{}>: **{}** tokens",
                        discord_id,
                        format_balance(balance)
                    ));
                }

                EmbedBuilder::new()
                    .color(embed::SUCCESS_COLOR)
                    .title("All Balances")
                    .description(lines.join("\n"))
                    .build()
            }
            None => {
                let pubkey = wallet_deriver.get_pubkey(user_id).await;
                let balance = solana_client.get_token_balance(&pubkey).await.unwrap_or(0);
                let formatted_balance = format_balance(balance);
                let solscan_url = format!("https://solscan.io/account/{}", pubkey);

                let avatar_url = get_avatar_url(user.id.get(), user.avatar);

                let pubkey_str = pubkey.to_string();
                let short_address = format!(
                    "{}...{}",
                    &pubkey_str[..6],
                    &pubkey_str[pubkey_str.len() - 4..]
                );

                let mut builder = EmbedBuilder::new()
                    .color(embed::SUCCESS_COLOR)
                    .field(EmbedFieldBuilder::new(
                        "Balance",
                        format!("**{}** tokens", formatted_balance),
                    ))
                    .field(EmbedFieldBuilder::new(
                        "Wallet",
                        format!("[{}]({})", short_address, solscan_url),
                    ));

                if let Ok(thumbnail) = ImageSource::url(&avatar_url) {
                    builder = builder.thumbnail(thumbnail);
                }

                builder.build()
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
                        embeds: Some(vec![response_embed]),
                        ..Default::default()
                    }),
                },
            )
            .await
            .ok();
    }
}

fn get_avatar_url(user_id: u64, avatar: Option<ImageHash>) -> String {
    match avatar {
        Some(hash) => format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png?size=128",
            user_id, hash
        ),
        None => {
            let index = (user_id >> 22) % 6;
            format!("https://cdn.discordapp.com/embed/avatars/{}.png", index)
        }
    }
}

fn format_balance(raw_amount: u64) -> String {
    let whole = raw_amount / 1_000_000_000;
    let frac = raw_amount % 1_000_000_000;

    if frac == 0 {
        format!("{}", whole)
    } else {
        let frac_str = format!("{:09}", frac);
        let trimmed = frac_str.trim_end_matches('0');
        let display_frac = if trimmed.len() > 2 {
            &trimmed[..2]
        } else {
            trimmed
        };
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
                    embeds: Some(vec![embed::failure(message).build()]),
                    ..Default::default()
                }),
            },
        )
        .await
        .ok();
}
