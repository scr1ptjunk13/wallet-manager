// src/activity/simulator.rs
use crate::error::WalletError;
use crate::network::ProxyManager;
use alloy_provider::{Provider, Http};
use alloy_primitives::{Address, U256};
use alloy_signer::LocalWallet;
use alloy_contract::Contract;
use reqwest::Client;
use rand::Rng;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use std::str::FromStr;

// Uniswap V3 Swap ABI (simplified, replace with actual ABI)
const UNISWAP_ABI: &str = r#"
[
    {"inputs":[{"internalType":"address","name":"recipient","type":"address"},{"internalType":"bool","name":"zeroForOne","type":"bool"},{"internalType":"int256","name":"amountSpecified","type":"int256"},{"internalType":"uint160","name":"sqrtPriceLimitX96","type":"uint160"}],"name":"swap","stateMutability":"nonpayable","type":"function"}
]
"#;

#[derive(Debug, Clone, serde::Serialize)]
struct DiscordMessage {
    content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TwitterPost {
    text: String,
}

pub struct ActivitySimulator {
    proxy_manager: ProxyManager,
    provider: Http<reqwest::Client>,
    wallet: LocalWallet,
    uniswap_contract: Contract,
    discord_api_key: Option<String>,
    twitter_api_key: Option<String>,
}

impl ActivitySimulator {
    pub fn new(
        rpc_url: String,
        private_key: String,
        discord_api_key: Option<String>,
        twitter_api_key: Option<String>,
        proxies: Vec<String>,
    ) -> Result<Self, WalletError> {
        // Initialize proxy manager with 50 proxies
        let proxy_manager = ProxyManager::new(proxies)?;

        // Get a client with a proxy for provider
        let client = proxy_manager.get_client().await?;
        let provider = Http::new_with_client(
            reqwest::Url::parse(&rpc_url)
                .map_err(|e| WalletError::MixingError(format!("Invalid RPC URL: {}", e)))?,
            client,
        );

        // Initialize wallet
        let wallet = LocalWallet::from_str(&private_key)
            .map_err(|e| WalletError::MixingError(format!("Invalid private key: {}", e)))?;

        // Initialize Uniswap V3 contract
        let uniswap_address = Address::from_str("0x...UniswapV3PoolAddress...") // Replace with actual address
            .map_err(|e| WalletError::MixingError(format!("Invalid Uniswap address: {}", e)))?;
        let uniswap_contract = Contract::new(uniswap_address, UNISWAP_ABI.parse().unwrap(), provider.clone());

        Ok(Self {
            proxy_manager,
            provider,
            wallet,
            uniswap_contract,
            discord_api_key,
            twitter_api_key,
        })
    }

    pub async fn simulate_onchain_activity(&self, wallet_id: Uuid, chain_id: u64) -> Result<(), WalletError> {
        let tx_count = rand::thread_rng().gen_range(2..6); // 2-5 transactions
        for i in 0..tx_count {
            // Rotate proxy for each transaction
            let client = self.proxy_manager.get_client().await?;
            let provider = Http::new_with_client(
                self.provider.url().clone(),
                client,
            );

            let amount = rand::thread_rng().gen_range(0.001..0.01);
            let amount_wei = U256::from((amount * 1e18) as u64);

            // Simulate Uniswap V3 swap
            let call = self.uniswap_contract
                .method("swap", (
                    self.wallet.address(),
                    true,
                    amount_wei,
                    U256::from(0),
                ))
                .map_err(|e| WalletError::MixingError(format!("Failed to prepare swap: {}", e)))?;

            let tx = call.send().await
                .map_err(|e| WalletError::MixingError(format!("Swap failed: {}", e)))?;

            log::info!("Simulated swap {} for wallet {}: tx_hash={}", i + 1, wallet_id, format!("0x{:x}", tx.tx_hash()));

            let delay = rand::thread_rng().gen_range(86400..604800);
            sleep(Duration::from_secs(delay)).await;
        }
        Ok(())
    }

    pub async fn simulate_offchain_activity(&self, wallet_id: Uuid) -> Result<(), WalletError> {
        let activity_count = rand::thread_rng().gen_range(1..4);
        for i in 0..activity_count {
            let client = self.proxy_manager.get_client().await?;
            let choice = rand::thread_rng().gen::<f32>();
            if choice < 0.5 && self.discord_api_key.is_some() {
                let message = DiscordMessage {
                    content: format!("Excited about the airdrop! #crypto {}", rand::thread_rng().gen::<u32>()),
                };
                let response = client
                    .post("https://discord.com/api/v10/channels/.../messages") // Replace with actual channel ID
                    .bearer_auth(self.discord_api_key.as_ref().unwrap())
                    .json(&message)
                    .send()
                    .await
                    .map_err(|e| WalletError::MixingError(format!("Discord API failed: {}", e)))?;
                log::info!("Simulated Discord message {} for wallet {}: status={}", i + 1, wallet_id, response.status());
            } else if self.twitter_api_key.is_some() {
                let post = TwitterPost {
                    text: format!("Just joined the latest #airdrop! 🚀 {}", rand::thread_rng().gen::<u32>()),
                };
                let response = client
                    .post("https://api.twitter.com/2/tweets")
                    .bearer_auth(self.twitter_api_key.as_ref().unwrap())
                    .json(&post)
                    .send()
                    .await
                    .map_err(|e| WalletError::MixingError(format!("Twitter API failed: {}", e)))?;
                log::info!("Simulated Twitter post {} for wallet {}: status={}", i + 1, wallet_id, response.status());
            }

            let delay = rand::thread_rng().gen_range(3600..86400);
            sleep(Duration::from_secs(delay)).await;
        }
        Ok(())
    }

    pub async fn simulate_activity(&self, wallet_id: Uuid, chain_id: u64) -> Result<(), WalletError> {
        tokio::try_join!(
            self.simulate_onchain_activity(wallet_id, chain_id),
            self.simulate_offchain_activity(wallet_id)
        )?;
        Ok(())
    }
}

