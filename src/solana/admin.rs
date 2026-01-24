use crate::solana::{client::SolanaClient, storage::WalletCache};
use std::env;
use std::sync::Arc;

/// Initial token amount for admin (4000 tokens with 9 decimals)
const ADMIN_INITIAL_TOKENS: u64 = 4_000_000_000_000; // 4000 * 10^9

/// Check if admin has been initialized, and if not, log minting instructions
pub async fn initialize_admin_if_needed(
    solana_client: &Arc<SolanaClient>,
    wallet_cache: &Arc<WalletCache>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let admin_id_str = env::var("DISCORD_ADMIN_ID")
        .map_err(|_| "DISCORD_ADMIN_ID environment variable not set")?;
    
    let admin_id: u64 = admin_id_str.parse()
        .map_err(|_| "DISCORD_ADMIN_ID must be a valid u64")?;
    
    // Get or create admin wallet using the cache
    let (_, admin_pubkey, was_created) = wallet_cache.get_or_create_wallet(admin_id).await;
    
    if was_created {
        log::info!("Created new admin wallet: {}", admin_pubkey);
    }
    
    // Check current balance (async)
    let current_balance = solana_client.get_token_balance(&admin_pubkey).await?;
    
    if current_balance == 0 {
        log::info!("Admin wallet has 0 tokens, will need initial minting");
        log::info!("Admin wallet address: {}", admin_pubkey);
        log::info!("Please mint {} tokens to this address using the Solana CLI:", ADMIN_INITIAL_TOKENS / 1_000_000_000);
        log::info!("  spl-token mint <TOKEN_MINT> {} {}", ADMIN_INITIAL_TOKENS / 1_000_000_000, admin_pubkey);
    } else {
        log::info!("Admin wallet balance: {} tokens", current_balance / 1_000_000_000);
    }
    
    Ok(())
}
