// src/funding/mixer/connectors/penumbra.rs
use crate::error::WalletError;
use crate::network::ProxyManager;
use crate::funding::mixer::types::{TornadoConnector, TornadoDepositResult, TornadoWithdrawResult};
use async_trait::async_trait;
use alloy_provider::{Provider, Http};
use alloy_primitives::{Address, U256};
use alloy_signer::LocalWallet;
use alloy_contract::Contract;
use uuid::Uuid;
use std::str::FromStr;

// Simplified Penumbra ABI (replace with actual ABI)
const PENUMBRA_ABI: &str = r#"
[
    {"inputs":[{"internalType":"bytes32","name":"_commitment","type":"bytes32"}],"name":"deposit","stateMutability":"payable","type":"function"},
    {"inputs":[{"internalType":"bytes32","name":"_nullifierHash","type":"bytes32"},{"internalType":"address","name":"_recipient","type":"address"}],"name":"withdraw","stateMutability":"nonpayable","type":"function"}
]
"#;

pub struct PenumbraConnector {
    chain_id: u64,
    relayer_url: String,
    private_key: String,
    proxy_manager: ProxyManager,
    provider: Http<reqwest::Client>,
    contract: Contract,
    wallet: LocalWallet,
}

impl PenumbraConnector {
    pub fn new(chain_id: u64, relayer_url: String, private_key: String, proxies: Vec<String>) -> Result<Self, WalletError> {
        let proxy_manager = ProxyManager::new(proxies)?;
        let client = proxy_manager.get_client().block_on()
            .map_err(|e| WalletError::MixingError(format!("Failed to get client: {}", e)))?;
        let provider = Http::new_with_client(
            reqwest::Url::parse(&relayer_url)
                .map_err(|e| WalletError::MixingError(format!("Invalid relayer URL: {}", e)))?,
            client,
        );
        let wallet = LocalWallet::from_str(&private_key)
            .map_err(|e| WalletError::MixingError(format!("Invalid private key: {}", e)))?;
        let contract_address = Address::from_str("0x...PenumbraContractAddress...") // Replace with actual address
            .map_err(|e| WalletError::MixingError(format!("Invalid contract address: {}", e)))?;
        let contract = Contract::new(contract_address, PENUMBRA_ABI.parse().unwrap(), provider.clone());

        Ok(Self {
            chain_id,
            relayer_url,
            private_key,
            proxy_manager,
            provider,
            contract,
            wallet,
        })
    }
}

#[async_trait]
impl TornadoConnector for PenumbraConnector {
    async fn deposit(&self, amount: f64, wallet_id: Uuid, anonymity_set: u32) -> Result<TornadoDepositResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.proxy_manager.get_client().await?;
        let provider = Http::new_with_client(
            reqwest::Url::parse(&self.relayer_url)
                .map_err(|e| WalletError::MixingError(format!("Invalid relayer URL: {}", e)))?,
            client,
        );

        let amount_wei = U256::from((amount * 1e18) as u64);
        let commitment = format!("0x{:x}", rand::thread_rng().gen::<u128>());
        let nullifier = format!("0x{:x}", rand::thread_rng().gen::<u128>());

        let call = self.contract
            .method("deposit", (commitment.parse::<[u8; 32]>().unwrap(),))
            .map_err(|e| WalletError::MixingError(format!("Failed to prepare deposit: {}", e)))?
            .value(amount_wei);

        let tx = call.send().await
            .map_err(|e| WalletError::MixingError(format!("Deposit failed: {}", e)))?;

        log::info!("Penumbra deposit for wallet {}: tx_hash={}", wallet_id, format!("0x{:x}", tx.tx_hash()));

        Ok(TornadoDepositResult {
            tx_hash: format!("0x{:x}", tx.tx_hash()),
            commitment,
            nullifier,
        })
    }

    async fn withdraw(&self, amount: f64, recipient: String, commitment: String, nullifier: String) -> Result<TornadoWithdrawResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.proxy_manager.get_client().await?;
        let provider = Http::new_with_client(
            reqwest::Url::parse(&self.relayer_url)
                .map_err(|e| WalletError::MixingError(format!("Invalid relayer URL: {}", e)))?,
            client,
        );

        let recipient_addr = Address::from_str(&recipient)
            .map_err(|e| WalletError::MixingError(format!("Invalid recipient address: {}", e)))?;

        let call = self.contract
            .method("withdraw", (nullifier.parse::<[u8; 32]>().unwrap(), recipient_addr))
            .map_err(|e| WalletError::MixingError(format!("Failed to prepare withdraw: {}", e)))?;

        let tx = call.send().await
            .map_err(|e| WalletError::MixingError(format!("Withdraw failed: {}", e)))?;

        log::info!("Penumbra withdraw to {}: tx_hash={}", recipient, format!("0x{:x}", tx.tx_hash()));

        Ok(TornadoWithdrawResult {
            tx_hash: format!("0x{:x}", tx.tx_hash()),
            final_amount: amount * 0.99,
        })
    }
}