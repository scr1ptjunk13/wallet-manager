// src/types.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub id: Uuid,
    pub address: String,
    pub encrypted_private_key: String,
    pub derivation_path: String,
    pub funding_source: FundingSource,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub balances: HashMap<String, Balance>, // chain_id -> Balance
    pub metadata: WalletMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub chain_id: u64,
    pub native_balance: f64,
    pub token_balances: HashMap<String, f64>, // token_address -> balance
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FundingSource {
    Cex {
        exchange: String,
        withdrawal_address: String,
    },
    Mixer {
        service: String,
        mix_id: String,
    },
    CrossChain {
        source_chain: u64,
        bridge_used: String,
    },
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    pub alias: Option<String>,
    pub proxy_used: Option<String>,
    pub risk_score: f64,
    pub active: bool,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct WalletConfig {
    pub master_seed: String,
    pub derivation_base: String,
    pub encryption_key: [u8; 32],
    pub supported_chains: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRequest {
    pub wallet_id: Uuid,
    pub amount: f64,
    pub chain_id: u64,
    pub source: FundingSource,
    pub priority: FundingPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FundingPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct BalanceUpdate {
    pub wallet_id: Uuid,
    pub chain_id: u64,
    pub native_balance: Option<f64>,
    pub token_updates: HashMap<String, f64>,
}
