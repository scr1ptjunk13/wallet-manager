// src/funding/mixer/connectors/aztec.rs
use crate::error::WalletError;
use super::super::types::{RelayNetwork, ShieldResult, PrivateTransferResult};
use async_trait::async_trait;
use alloy_provider::Http;
use alloy_primitives::{Address, U256};
use alloy_signer::LocalWallet;
use alloy_contract::Contract;
use uuid::Uuid;
use std::str::FromStr;

// Aztec ABI (simplified, replace with actual ABI)
const AZTEC_ABI: &str = r#"
[
    {"inputs":[{"internalType":"uint256","name":"amount","type":"uint256"}],"name":"shield","stateMutability":"payable","type":"function"},
    {"inputs":[{"internalType":"bytes32","name":"commitment","type":"bytes32"},{"internalType":"address","name":"recipient","type":"address"},{"internalType":"uint256","name":"amount","type":"uint256"}],"name":"privateTransfer","stateMutability":"nonpayable","type":"function"}
]
"#;

pub struct AztecRelay {
    api_key: String,
    client: reqwest::Client,
    provider: Http<reqwest::Client>,
    contract: Contract,
    wallet: LocalWallet,
}

impl AztecRelay {
    pub fn new(api_key: String) -> Result<Self, WalletError> {
        let provider = Http::new(reqwest::Url::parse("https://aztec-rpc-url") // Replace with actual RPC
            .map_err(|e| WalletError::MixingError(format!("Invalid RPC URL: {}", e)))?);
        let wallet = LocalWallet::from_str(&api_key)
            .map_err(|e| WalletError::MixingError(format!("Invalid private key: {}", e)))?;
        let contract_address = Address::from_str("0x...AztecContractAddress...") // Replace with actual address
            .map_err(|e| WalletError::MixingError(format!("Invalid contract address: {}", e)))?;
        let contract = Contract::new(contract_address, AZTEC_ABI.parse().unwrap(), provider.clone());

        Ok(Self {
            api_key,
            client: reqwest::Client::new(),
            provider,
            contract,
            wallet,
        })
    }
}

#[async_trait]
impl RelayNetwork for AztecRelay {
    async fn shield_funds(&self, wallet_id: Uuid, amount: f64) -> Result<ShieldResult, Box<dyn std::error::Error + Send + Sync>> {
        let amount_wei = U256::from((amount * 1e18) as u64);
        let commitment = format!("0x{:x}", rand::thread_rng().gen::<u128>());

        let call = self.contract
            .method("shield", (amount_wei,))
            .map_err(|e| WalletError::MixingError(format!("Failed to prepare shield: {}", e)))?
            .value(amount_wei);

        let tx = call.send().await
            .map_err(|e| WalletError::MixingError(format!("Shield failed: {}", e)))?;

        let tx_hash = format!("0x{:x}", tx.tx_hash());

        Ok(ShieldResult {
            tx_hash,
            commitment,
        })
    }

    async fn private_transfer(&self, commitment: String, recipient: String, amount: f64) -> Result<PrivateTransferResult, Box<dyn std::error::Error + Send + Sync>> {
        let recipient_addr = Address::from_str(&recipient)
            .map_err(|e| WalletError::MixingError(format!("Invalid recipient address: {}", e)))?;
        let amount_wei = U256::from((amount * 1e18) as u64);

        let call = self.contract
            .method("privateTransfer", (commitment.parse::<[u8; 32]>().unwrap(), recipient_addr, amount_wei))
            .map_err(|e| WalletError::MixingError(format!("Failed to prepare private transfer: {}", e)))?;

        let tx = call.send().await
            .map_err(|e| WalletError::MixingError(format!("Private transfer failed: {}", e)))?;

        let tx_hash = format!("0x{:x}", tx.tx_hash());

        Ok(PrivateTransferResult {
            tx_hash,
        })
    }
}