use sha2::{Digest, Sha256};
use solana_sdk::signature::{keypair_from_seed, Keypair};
use solana_sdk::signer::Signer;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct WalletDeriver {
    seed: [u8; 32],
    cache: RwLock<HashMap<u64, Keypair>>,
}

impl WalletDeriver {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let seed_hex =
            std::env::var("WALLET_SEED").map_err(|_| "WALLET_SEED environment variable not set")?;

        let seed_bytes = hex::decode(&seed_hex).map_err(|_| "WALLET_SEED must be valid hex")?;

        if seed_bytes.len() != 32 {
            return Err("WALLET_SEED must be exactly 32 bytes (64 hex chars)".into());
        }

        let mut seed = [0u8; 32];
        seed.copy_from_slice(&seed_bytes);

        Ok(Self {
            seed,
            cache: RwLock::new(HashMap::new()),
        })
    }

    fn derive(&self, discord_id: u64) -> Keypair {
        let mut hasher = Sha256::new();
        hasher.update(self.seed);
        hasher.update(discord_id.to_le_bytes());
        let derived: [u8; 32] = hasher.finalize().into();
        keypair_from_seed(&derived).expect("valid 32-byte seed")
    }

    pub async fn get_keypair(&self, discord_id: u64) -> Keypair {
        {
            let cache = self.cache.read().await;
            if let Some(kp) = cache.get(&discord_id) {
                return Keypair::try_from(kp.to_bytes().as_slice()).expect("valid keypair");
            }
        }

        let keypair = self.derive(discord_id);
        let mut cache = self.cache.write().await;
        let result = Keypair::try_from(keypair.to_bytes().as_slice()).expect("valid keypair");
        cache.insert(discord_id, keypair);
        result
    }

    pub async fn get_pubkey(&self, discord_id: u64) -> solana_sdk::pubkey::Pubkey {
        self.get_keypair(discord_id).await.pubkey()
    }

    pub async fn get_all_cached(&self) -> Vec<(u64, solana_sdk::pubkey::Pubkey)> {
        let cache = self.cache.read().await;
        cache.iter().map(|(id, kp)| (*id, kp.pubkey())).collect()
    }
}

pub fn create_wallet_deriver(
) -> Result<Arc<WalletDeriver>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Arc::new(WalletDeriver::new()?))
}
