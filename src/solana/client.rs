use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::CommitmentConfig;
use solana_sdk::{
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
    pub fn fee_payer_pubkey(&self) -> Pubkey {
        self.fee_payer.pubkey()
    }

    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let rpc_url = env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

        let fee_payer_secret = env::var("BOT_WALLET_SECRET")
            .map_err(|_| "BOT_WALLET_SECRET environment variable not set")?;

        let secret_bytes: Vec<u8> = serde_json::from_str(&fee_payer_secret)
            .map_err(|_| "BOT_WALLET_SECRET must be a JSON array of bytes")?;

        let fee_payer = Keypair::try_from(secret_bytes.as_slice())
            .map_err(|_| "Invalid BOT_WALLET_SECRET keypair bytes")?;

        let token_mint_str = env::var("TOKEN_MINT_ADDRESS")
            .map_err(|_| "TOKEN_MINT_ADDRESS environment variable not set")?;

        let token_mint =
            Pubkey::from_str(&token_mint_str).map_err(|_| "Invalid TOKEN_MINT_ADDRESS pubkey")?;

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

    pub async fn get_token_balance(
        &self,
        owner: &Pubkey,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let ata = get_associated_token_address(owner, &self.token_mint);

        let account_data = match self.rpc.get_account_data(&ata).await {
            Ok(data) => data,
            Err(_) => return Ok(0),
        };

        let token_account = TokenAccount::unpack(&account_data)?;
        Ok(token_account.amount)
    }

    pub async fn create_ata_if_needed(
        &self,
        owner: &Pubkey,
    ) -> Result<Pubkey, Box<dyn std::error::Error + Send + Sync>> {
        let ata = get_associated_token_address(owner, &self.token_mint);

        if self.rpc.get_account_data(&ata).await.is_ok() {
            return Ok(ata);
        }

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

    pub async fn transfer_tokens(
        &self,
        from_keypair: &Keypair,
        to_owner: &Pubkey,
        amount: u64,
    ) -> Result<Signature, Box<dyn std::error::Error + Send + Sync>> {
        let from_pubkey = from_keypair.pubkey();
        let from_ata = get_associated_token_address(&from_pubkey, &self.token_mint);
        let to_ata = get_associated_token_address(to_owner, &self.token_mint);

        self.create_ata_if_needed(to_owner).await?;

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
}
