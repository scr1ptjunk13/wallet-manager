// src/types.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use crate::funding::mixer::types::{MixingStrategy, MixingStepType, CustomMixingPattern, CustomMixingStep};


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

// Funding configuration for all sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingConfig {
    pub cex_config: CexConfig, // From cex.rs
    pub mixer_config: MixerConfig,
    pub cross_chain_config: CrossChainConfig, // From cross_chain.rs
    pub default_privacy_level: PrivacyLevel,
    pub max_retry_attempts: u32,
    pub retry_delay_seconds: u64,
}

impl Default for FundingConfig {
    fn default() -> Self {
        Self {
            cex_config: CexConfig::default(),
            mixer_config: MixerConfig::default(),
            cross_chain_config: CrossChainConfig::default(),
            default_privacy_level: PrivacyLevel::Medium,
            max_retry_attempts: 3,
            retry_delay_seconds: 60,
        }
    }
}

//Mixer Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerConfig {
    pub tornado_enabled: bool,
    pub tornado_relayer_url: String,
    pub tornado_private_key: String,
    pub aztec_enabled: bool,
    pub aztec_api_key: String,
    pub railgun_enabled: bool,
    pub railgun_api_key: String,
    pub noir_enabled: bool,
    pub noir_api_key: String,
    pub penumbra_enabled: bool,
    pub penumbra_api_key: String,
    pub min_mixing_delay: u64,
    pub cross_chain_hops: usize,
}

impl Default for MixerConfig {
    fn default() -> Self {
        Self {
            tornado_enabled: false,
            tornado_relayer_url: String::new(),
            tornado_private_key: String::new(),
            aztec_enabled: false,
            aztec_api_key: String::new(),
            railgun_enabled: false,
            railgun_api_key: String::new(),
            noir_enabled: false,
            noir_api_key: String::new(),
            penumbra_enabled: false,
            penumbra_api_key: String::new(),
            min_mixing_delay: 300,
            cross_chain_hops: 3,
        }
    }
}




// Mixing record for history tracking
#[derive(Debug, Clone)]
pub struct MixingRecord {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub chain_id: u64,
    pub amount: f64,
    pub final_amount: f64,
    pub strategy: MixingStrategy,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
    pub steps_count: usize,
    pub success: bool,
}

// Funding record for history tracking
#[derive(Debug, Clone)]
pub struct FundingRecord {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub amount: f64,
    pub chain_id: u64,
    pub funding_source: FundingSource,
    pub success: bool,
    pub transaction_hash: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cost: f64,
    pub execution_time_seconds: u64,
}

// Funding request
#[derive(Debug, Clone)]
pub struct FundingRequest {
    pub wallet_id: Uuid,
    pub amount: f64,
    pub chain_id: u64,
    pub funding_source: FundingSource,
    pub priority: FundingPriority,
    pub max_wait_time: u64,
    pub privacy_requirements: PrivacyLevel,
}

// Mixer funding request
#[derive(Debug, Clone)]
pub struct MixerFundingRequest {
    pub wallet_id: Uuid,
    pub amount: f64,
    pub chain_id: u64,
    pub mixer_type: MixerType,
    pub anonymity_set: u32,
    pub delay_hours: u32,
}

// CEX funding request (placeholder, defined in cex.rs)
#[derive(Debug, Clone)]
pub struct CexFundingRequest {
    pub wallet_id: Uuid,
    pub amount: f64,
    pub chain_id: u64,
    pub exchange: String,
    pub withdraw_method: WithdrawMethod,
    pub delay_seconds: u64,
}

// Cross-chain funding request (placeholder, defined in cross_chain.rs)
#[derive(Debug, Clone)]
pub struct CrossChainFundingRequest {
    pub wallet_id: Uuid,
    pub amount: f64,
    pub source_chain: u64,
    pub target_chain: u64,
    pub bridge: String,
    pub slippage_tolerance: f64,
}

// Funding source types
#[derive(Debug, Clone)]
pub enum FundingSource {
    Cex(CexFundingRequest),
    Mixer(MixerFundingRequest),
    CrossChain(CrossChainFundingRequest),
    Manual,
}

// Funding source types for strategy
#[derive(Debug, Clone, PartialEq)]
pub enum FundingSourceType {
    Cex,
    Mixer,
    CrossChain,
}

// Privacy levels
#[derive(Debug, Clone, PartialEq)]
pub enum PrivacyLevel {
    Low,
    Medium,
    High,
}

// Funding priority
#[derive(Debug, Clone)]
pub enum FundingPriority {
    Low,
    Normal,
    High,
}

// Funding statistics
#[derive(Debug, Clone)]
pub struct FundingStats {
    pub total_wallets_funded: usize,
    pub total_amount_funded: f64,
    pub funding_by_source: HashMap<String, f64>,
    pub success_rate: f64,
    pub average_amount: f64,
}

// Funding strategy recommendation
#[derive(Debug, Clone)]
pub struct FundingStrategy {
    pub primary_source: FundingSourceType,
    pub backup_source: Option<FundingSourceType>,
    pub split_funding: bool,
    pub privacy_level: PrivacyLevel,
    pub estimated_time_minutes: u32,
    pub estimated_cost: f64,
}

// Funding recommendation
#[derive(Debug, Clone)]
pub struct FundingRecommendation {
    pub source: FundingSourceType,
    pub estimated_cost: f64,
    pub estimated_time_minutes: u32,
    pub privacy_score: u32,
    pub reliability_score: u32,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
}

// Funding result
#[derive(Debug, Clone)]
pub struct FundingResult {
    pub wallet_id: Uuid,
    pub success: bool,
    pub error: Option<String>,
    pub transaction_hash: Option<String>,
}

// Mixer types
#[derive(Debug, Clone)]
pub enum MixerType {
    Tornado,
    Aztec,
    Railgun,
}

// Placeholder for CEX config (defined in cex.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CexConfig {
    // Define fields as needed
}

impl Default for CexConfig {
    fn default() -> Self {
        Self {}
    }
}

// Placeholder for CrossChain config (defined in cross_chain.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainConfig {
    // Define fields as needed
}

impl Default for CrossChainConfig {
    fn default() -> Self {
        Self {}
    }
}

// Placeholder for withdraw method (defined in cex.rs)
#[derive(Debug, Clone)]
pub enum WithdrawMethod {
    Direct,
    // Add other methods as needed
}