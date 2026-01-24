use crate::solana::{client::SolanaClient, storage::WalletDeriver};
use std::env;
use std::sync::Arc;

pub async fn initialize_admin_if_needed(
    solana_client: &Arc<SolanaClient>,
    wallet_deriver: &Arc<WalletDeriver>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let admin_id: u64 = env::var("DISCORD_ADMIN_ID")
        .map_err(|_| "DISCORD_ADMIN_ID not set")?
        .parse()
        .map_err(|_| "DISCORD_ADMIN_ID must be u64")?;

    let admin_pubkey = wallet_deriver.get_pubkey(admin_id).await;
    let balance = solana_client.get_token_balance(&admin_pubkey).await?;

    log::info!(
        "Admin wallet {}: {} tokens",
        admin_pubkey,
        balance / 1_000_000_000
    );

    Ok(())
}
