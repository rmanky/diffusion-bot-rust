use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

const WALLETS_FILE: &str = "wallets.json";

/// Represents a stored user wallet
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredWallet {
    discord_id: u64,
    secret_key: Vec<u8>,
}

/// In-memory wallet cache with RwLock for concurrent access
pub struct WalletCache {
    wallets: RwLock<HashMap<u64, Keypair>>,
}

impl WalletCache {
    /// Create a new wallet cache, loading from disk
    pub fn new() -> Self {
        let wallets = load_wallets_from_disk();
        log::info!("Loaded {} wallets from disk", wallets.len());
        Self {
            wallets: RwLock::new(wallets),
        }
    }

    /// Get a wallet's public key (read-only, no lock contention)
    pub async fn get_pubkey(&self, discord_id: u64) -> Option<solana_sdk::pubkey::Pubkey> {
        let wallets = self.wallets.read().await;
        wallets.get(&discord_id).map(|kp| kp.pubkey())
    }

    /// Check if a wallet exists
    pub async fn exists(&self, discord_id: u64) -> bool {
        let wallets = self.wallets.read().await;
        wallets.contains_key(&discord_id)
    }

    /// Get all wallet pubkeys with their discord IDs (for balance all)
    pub async fn get_all_pubkeys(&self) -> Vec<(u64, solana_sdk::pubkey::Pubkey)> {
        let wallets = self.wallets.read().await;
        wallets.iter().map(|(id, kp)| (*id, kp.pubkey())).collect()
    }

    /// Get or create a wallet, returning a cloned keypair
    /// Returns (keypair_bytes, pubkey, was_created)
    pub async fn get_or_create_wallet(&self, discord_id: u64) -> (Vec<u8>, solana_sdk::pubkey::Pubkey, bool) {
        // First try read lock
        {
            let wallets = self.wallets.read().await;
            if let Some(kp) = wallets.get(&discord_id) {
                return (kp.to_bytes().to_vec(), kp.pubkey(), false);
            }
        }

        // Need to create - acquire write lock
        let mut wallets = self.wallets.write().await;
        
        // Double-check (another task might have created it)
        if let Some(kp) = wallets.get(&discord_id) {
            return (kp.to_bytes().to_vec(), kp.pubkey(), false);
        }

        // Create new wallet
        let keypair = Keypair::new();
        let pubkey = keypair.pubkey();
        let bytes = keypair.to_bytes().to_vec();
        
        log::info!(
            "Created new wallet for Discord user {}: {}",
            discord_id,
            pubkey
        );
        
        wallets.insert(discord_id, keypair);
        save_wallets_to_disk(&wallets);
        
        (bytes, pubkey, true)
    }

    /// Get the keypair bytes for a user (for signing transactions)
    pub async fn get_keypair_bytes(&self, discord_id: u64) -> Option<Vec<u8>> {
        let wallets = self.wallets.read().await;
        wallets.get(&discord_id).map(|kp| kp.to_bytes().to_vec())
    }
}

/// Load wallets from disk (internal)
fn load_wallets_from_disk() -> HashMap<u64, Keypair> {
    let path = Path::new(WALLETS_FILE);
    if !path.exists() {
        return HashMap::new();
    }

    let data = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) => {
            log::error!("Failed to read wallets file: {}", e);
            return HashMap::new();
        }
    };

    let stored: Vec<StoredWallet> = match serde_json::from_str(&data) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse wallets file: {}", e);
            return HashMap::new();
        }
    };

    stored
        .into_iter()
        .filter_map(|w| {
            Keypair::try_from(w.secret_key.as_slice())
                .ok()
                .map(|kp| (w.discord_id, kp))
        })
        .collect()
}

/// Save wallets to disk (internal)
fn save_wallets_to_disk(wallets: &HashMap<u64, Keypair>) {
    let stored: Vec<StoredWallet> = wallets
        .iter()
        .map(|(discord_id, keypair)| StoredWallet {
            discord_id: *discord_id,
            secret_key: keypair.to_bytes().to_vec(),
        })
        .collect();

    let data = match serde_json::to_string_pretty(&stored) {
        Ok(d) => d,
        Err(e) => {
            log::error!("Failed to serialize wallets: {}", e);
            return;
        }
    };

    if let Err(e) = fs::write(WALLETS_FILE, data) {
        log::error!("Failed to write wallets file: {}", e);
    }
}

/// Create a shared wallet cache wrapped in Arc
pub fn create_wallet_cache() -> Arc<WalletCache> {
    Arc::new(WalletCache::new())
}
