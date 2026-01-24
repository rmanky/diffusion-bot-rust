use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::state::Account as TokenAccount;
use std::env;
use std::str::FromStr;

pub struct SolanaClient {
    rpc: RpcClient,
    fee_payer: Keypair,
    pub token_mint: Pubkey,
}

impl SolanaClient {
    /// Create a new Solana client from environment variables
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let rpc_url = env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

        let fee_payer_secret = env::var("BOT_WALLET_SECRET")
            .map_err(|_| "BOT_WALLET_SECRET environment variable not set")?;

        // Parse as base58 (e.g., from Cake Wallet or other wallet exports)
        let secret_bytes = bs58::decode(&fee_payer_secret)
            .into_vec()
            .map_err(|_| "BOT_WALLET_SECRET must be a valid base58 string")?;

        let fee_payer = Keypair::try_from(secret_bytes.as_slice())
            .map_err(|_| "Invalid BOT_WALLET_SECRET keypair bytes")?;

        let token_mint_str = env::var("TOKEN_MINT_ADDRESS")
            .map_err(|_| "TOKEN_MINT_ADDRESS environment variable not set")?;

        let token_mint = Pubkey::from_str(&token_mint_str)
            .map_err(|_| "Invalid TOKEN_MINT_ADDRESS pubkey")?;

        let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

        log::info!("Solana client initialized:");
        log::info!("  Fee payer: {}", fee_payer.pubkey());
        log::info!("  Token mint: {}", token_mint);

        Ok(Self {
            rpc,
            fee_payer,
            token_mint,
        })
    }

    /// Get the token balance for a wallet
    pub async fn get_token_balance(&self, owner: &Pubkey) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let ata = get_associated_token_address(owner, &self.token_mint);

        // Check if the ATA exists
        let account_data = match self.rpc.get_account_data(&ata).await {
            Ok(data) => data,
            Err(_) => {
                // ATA doesn't exist, balance is 0
                return Ok(0);
            }
        };

        let token_account = TokenAccount::unpack(&account_data)?;
        Ok(token_account.amount)
    }

    /// Create an associated token account if it doesn't exist
    pub async fn create_ata_if_needed(
        &self,
        owner: &Pubkey,
    ) -> Result<Pubkey, Box<dyn std::error::Error + Send + Sync>> {
        let ata = get_associated_token_address(owner, &self.token_mint);

        // Check if ATA already exists
        if self.rpc.get_account_data(&ata).await.is_ok() {
            return Ok(ata);
        }

        // Create the ATA
        let create_ata_ix = create_associated_token_account(
            &self.fee_payer.pubkey(),
            owner,
            &self.token_mint,
            &spl_token::id(),
        );

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_ata_ix],
            Some(&self.fee_payer.pubkey()),
            &[&self.fee_payer],
            recent_blockhash,
        );

        self.rpc.send_and_confirm_transaction(&tx).await?;
        log::info!("Created ATA {} for owner {}", ata, owner);

        Ok(ata)
    }

    /// Transfer tokens from one wallet to another
    pub async fn transfer_tokens(
        &self,
        from_keypair: &Keypair,
        to_owner: &Pubkey,
        amount: u64,
    ) -> Result<Signature, Box<dyn std::error::Error + Send + Sync>> {
        let from_pubkey = from_keypair.pubkey();
        let from_ata = get_associated_token_address(&from_pubkey, &self.token_mint);
        let to_ata = get_associated_token_address(to_owner, &self.token_mint);

        // Ensure destination ATA exists
        self.create_ata_if_needed(to_owner).await?;

        // Create transfer instruction
        let transfer_ix = spl_token::instruction::transfer(
            &spl_token::id(),
            &from_ata,
            &to_ata,
            &from_pubkey,
            &[],
            amount,
        )?;

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[transfer_ix],
            Some(&self.fee_payer.pubkey()),
            &[&self.fee_payer, from_keypair],
            recent_blockhash,
        );

        let signature = self.rpc.send_and_confirm_transaction(&tx).await?;
        log::info!(
            "Transferred {} tokens from {} to {}: {}",
            amount,
            from_pubkey,
            to_owner,
            signature
        );

        Ok(signature)
    }

    /// Mint tokens to an address (requires mint authority)
    pub async fn mint_tokens(
        &self,
        mint_authority: &Keypair,
        to_owner: &Pubkey,
        amount: u64,
    ) -> Result<Signature, Box<dyn std::error::Error + Send + Sync>> {
        // Ensure destination ATA exists
        let to_ata = self.create_ata_if_needed(to_owner).await?;

        // Create mint instruction
        let mint_ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            &self.token_mint,
            &to_ata,
            &mint_authority.pubkey(),
            &[],
            amount,
        )?;

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[mint_ix],
            Some(&self.fee_payer.pubkey()),
            &[&self.fee_payer, mint_authority],
            recent_blockhash,
        );

        let signature = self.rpc.send_and_confirm_transaction(&tx).await?;
        log::info!(
            "Minted {} tokens to {}: {}",
            amount,
            to_owner,
            signature
        );

        Ok(signature)
    }

    /// Get the fee payer's SOL balance
    pub async fn get_fee_payer_balance(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.rpc.get_balance(&self.fee_payer.pubkey()).await?)
    }
}
