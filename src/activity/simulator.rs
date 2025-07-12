// src/activity/simulator.rs
use crate::error::WalletError;
use crate::network::ProxyManager;
use alloy_provider::{ProviderBuilder};
use alloy_primitives::{Address, U256};
//use alloy_signer::signer::{Signer, SignerSync};
use alloy_signer_local::PrivateKeySigner;
use alloy_contract::{ContractInstance, Interface};
use alloy_rpc_types::TransactionRequest;
use alloy_sol_macro::sol;
use reqwest::Client;
use rand::Rng;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use std::str::FromStr;

// Define the Uniswap V3 contract using the sol! macro
sol! {
    #[sol(rpc)]
    contract UniswapV3Pool {
        function swap(
            address recipient,
            bool zeroForOne,
            int256 amountSpecified,
            uint160 sqrtPriceLimitX96,
            bytes calldata data
        ) external returns (int256 amount0, int256 amount1);
    }
}

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
    rpc_url: String,
    wallet: PrivateKeySigner,
    uniswap_address: Address,
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
        // Initialize proxy manager
        let proxy_manager = ProxyManager::new(proxies)?;

        // Initialize wallet with PrivateKeySigner
        let wallet = private_key.parse::<PrivateKeySigner>()
            .map_err(|e| WalletError::MixingError(format!("Invalid private key: {}", e)))?;

        // Uniswap V3 router address (replace with actual address for your network)
        let uniswap_address = Address::from_str("0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45") // Uniswap V3 Router
            .map_err(|e| WalletError::MixingError(format!("Invalid Uniswap address: {}", e)))?;

        Ok(Self {
            proxy_manager,
            rpc_url,
            wallet,
            uniswap_address,
            discord_api_key,
            twitter_api_key,
        })
    }

    async fn get_provider(&self) -> Result<ReqwestProvider, WalletError> {
        let client = self.proxy_manager.get_client().await?;
        let provider = ProviderBuilder::new()
            .with_client(client)
            .on_http(self.rpc_url.parse()
                .map_err(|e| WalletError::MixingError(format!("Invalid RPC URL: {}", e)))?);
        Ok(provider)
    }

    pub async fn simulate_onchain_activity(&self, wallet_id: Uuid, chain_id: u64) -> Result<(), WalletError> {
        let tx_count = rand::thread_rng().gen_range(2..6); // 2-5 transactions

        for i in 0..tx_count {
            // Get provider with rotated proxy
            let provider = self.get_provider().await?;

            let amount = rand::thread_rng().gen_range(0.001..0.01);
            let amount_wei = U256::from((amount * 1e18) as u64);

            // Create a simple transfer transaction
            let tx = TransactionRequest::default()
                .with_to(self.uniswap_address)
                .with_value(amount_wei)
                .with_gas_limit(21000)
                .with_chain_id(chain_id);

            // Sign and send transaction
            match provider.send_transaction(tx).await {
                Ok(pending_tx) => {
                    log::info!("Simulated transaction {} for wallet {}: tx_hash={:?}",
                              i + 1, wallet_id, pending_tx.tx_hash());
                },
                Err(e) => {
                    log::warn!("Failed to send transaction {} for wallet {}: {}",
                              i + 1, wallet_id, e);
                    // Continue with next transaction instead of failing completely
                }
            }

            // Random delay between 1-7 days
            let delay = rand::thread_rng().gen_range(86400..604800);
            sleep(Duration::from_secs(delay)).await;
        }
        Ok(())
    }

    pub async fn simulate_uniswap_activity(&self, wallet_id: Uuid, chain_id: u64) -> Result<(), WalletError> {
        let provider = self.get_provider().await?;

        // Create contract instance
        let contract = UniswapV3Pool::new(self.uniswap_address, &provider);

        let amount = rand::thread_rng().gen_range(0.001..0.01);
        let amount_wei = U256::from((amount * 1e18) as u64);

        // Build the swap call
        let call_builder = contract.swap(
            self.wallet.address(),
            true, // zeroForOne
            amount_wei.try_into().unwrap_or(1000000), // amountSpecified as int256
            U256::from(0), // sqrtPriceLimitX96
            vec![].into(), // empty calldata
        );

        // Send the transaction
        match call_builder.send().await {
            Ok(pending_tx) => {
                log::info!("Simulated Uniswap swap for wallet {}: tx_hash={:?}",
                          wallet_id, pending_tx.tx_hash());
            },
            Err(e) => {
                log::warn!("Failed to execute Uniswap swap for wallet {}: {}",
                          wallet_id, e);
            }
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

                // Note: You need to replace with actual channel ID
                let response = client
                    .post("https://discord.com/api/v10/channels/YOUR_CHANNEL_ID/messages")
                    .bearer_auth(self.discord_api_key.as_ref().unwrap())
                    .json(&message)
                    .send()
                    .await
                    .map_err(|e| WalletError::MixingError(format!("Discord API failed: {}", e)))?;

                log::info!("Simulated Discord message {} for wallet {}: status={}",
                          i + 1, wallet_id, response.status());

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

                log::info!("Simulated Twitter post {} for wallet {}: status={}",
                          i + 1, wallet_id, response.status());
            }

            // Random delay between 1 hour and 1 day
            let delay = rand::thread_rng().gen_range(3600..86400);
            sleep(Duration::from_secs(delay)).await;
        }
        Ok(())
    }

    pub async fn simulate_activity(&self, wallet_id: Uuid, chain_id: u64) -> Result<(), WalletError> {
        // Run both activities concurrently
        let onchain_result = self.simulate_onchain_activity(wallet_id, chain_id);
        let offchain_result = self.simulate_offchain_activity(wallet_id);

        // Wait for both to complete
        tokio::try_join!(onchain_result, offchain_result)?;
        Ok(())
    }

    pub async fn simulate_comprehensive_activity(&self, wallet_id: Uuid, chain_id: u64) -> Result<(), WalletError> {
        // Run all three types of activities
        let onchain_result = self.simulate_onchain_activity(wallet_id, chain_id);
        let uniswap_result = self.simulate_uniswap_activity(wallet_id, chain_id);
        let offchain_result = self.simulate_offchain_activity(wallet_id);

        // Wait for all to complete
        tokio::try_join!(onchain_result, uniswap_result, offchain_result)?;
        Ok(())
    }
}