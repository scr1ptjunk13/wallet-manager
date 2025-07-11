// src/funding/mod.rs
pub mod cex;
pub mod mixer;
pub mod cross_chain;

pub use cex::CexFunding;
pub use mixer::{MixerFunding, MixingStrategy, MixingStepType, CustomMixingPattern, CustomMixingStep};
pub use cross_chain::CrossChainFunding;

use crate::types::*;
use crate::error::WalletError;
use std::collections::HashMap;
use uuid::Uuid;


/// Main funding manager that coordinates all funding sources
pub struct FundingManager {
    cex_funding: CexFunding,
    mixer_funding: MixerFunding,
    cross_chain_funding: CrossChainFunding,
    funding_history: HashMap<Uuid, Vec<FundingRecord>>,
    config: FundingConfig,
}

impl FundingManager {
    /// Create a new funding manager
    pub async fn new() -> Result<Self, WalletError> {
        let config = FundingConfig::default();

        Ok(Self {
            cex_funding: CexFunding::new(&config.cex_config).await?,
            mixer_funding: MixerFunding::new(&config.mixer_config).await?,
            cross_chain_funding: CrossChainFunding::new(&config.cross_chain_config).await?,
            funding_history: HashMap::new(),
            config,
        })
    }

    /// Create with custom config
    pub async fn with_config(config: FundingConfig) -> Result<Self, WalletError> {
        Ok(Self {
            cex_funding: CexFunding::new(&config.cex_config).await?,
            mixer_funding: MixerFunding::new(&config.mixer_config).await?,
            cross_chain_funding: CrossChainFunding::new(&config.cross_chain_config).await?,
            funding_history: HashMap::new(),
            config,
        })
    }

    /// Fund a wallet using the specified method
    pub async fn fund_wallet(&mut self, request: FundingRequest) -> Result<(), WalletError> {
        let funding_record = match request.funding_source {
            FundingSource::Cex(ref cex_request) => {
                self.cex_funding.fund_wallet(cex_request.clone()).await?
            }
            FundingSource::Mixer(ref mixer_request) => {
                self.mixer_funding.fund_wallet(mixer_request.clone()).await?
            }
            FundingSource::CrossChain(ref cross_chain_request) => {
                self.cross_chain_funding.fund_wallet(cross_chain_request.clone()).await?
            }
            FundingSource::Manual => {
                return Err(WalletError::FundingError("Manual funding not supported".to_string()));
            }
        };

        // Store funding record
        self.funding_history
            .entry(request.wallet_id)
            .or_insert_with(Vec::new)
            .push(funding_record);

        Ok(())
    }

    /// Fund multiple wallets in batch
    pub async fn fund_wallets_batch(&mut self, requests: Vec<FundingRequest>) -> Result<Vec<FundingResult>, WalletError> {
        let mut results = Vec::new();

        for request in requests {
            let wallet_id = request.wallet_id;
            match self.fund_wallet(request).await {
                Ok(()) => results.push(FundingResult {
                    wallet_id,
                    success: true,
                    error: None,
                    transaction_hash: None,
                }),
                Err(e) => results.push(FundingResult {
                    wallet_id,
                    success: false,
                    error: Some(e.to_string()),
                    transaction_hash: None,
                }),
            }
        }

        Ok(results)
    }

    /// Get funding history for a wallet
    pub fn get_funding_history(&self, wallet_id: Uuid) -> Option<&Vec<FundingRecord>> {
        self.funding_history.get(&wallet_id)
    }

    /// Get total funded amount for a wallet
    pub fn get_total_funded(&self, wallet_id: Uuid) -> f64 {
        self.funding_history
            .get(&wallet_id)
            .map(|records| records.iter().map(|r| r.amount).sum())
            .unwrap_or(0.0)
    }

    /// Get funding statistics
    pub fn get_funding_stats(&self) -> FundingStats {
        let mut stats = FundingStats {
            total_wallets_funded: self.funding_history.len(),
            total_amount_funded: 0.0,
            funding_by_source: HashMap::new(),
            success_rate: 0.0,
            average_amount: 0.0,
        };

        let mut total_records = 0;
        let mut successful_records = 0;

        for records in self.funding_history.values() {
            for record in records {
                total_records += 1;
                stats.total_amount_funded += record.amount;

                if record.success {
                    successful_records += 1;
                }

                let source_name = match record.funding_source {
                    FundingSource::Cex(_) => "CEX",
                    FundingSource::Mixer(_) => "Mixer",
                    FundingSource::CrossChain(_) => "CrossChain",
                    FundingSource::Manual => "Manual",
                };

                *stats.funding_by_source.entry(source_name.to_string()).or_insert(0.0) += record.amount;
            }
        }

        if total_records > 0 {
            stats.success_rate = (successful_records as f64 / total_records as f64) * 100.0;
            stats.average_amount = stats.total_amount_funded / total_records as f64;
        }

        stats
    }

    /// Optimize funding strategy based on amount and requirements
    pub fn optimize_funding_strategy(&self, request: &FundingRequest) -> FundingStrategy {
        let amount = request.amount;
        let chain_id = request.chain_id;

        // Strategy based on amount
        if amount < 0.1 {
            // Small amounts - use CEX for efficiency
            FundingStrategy {
                primary_source: FundingSourceType::Cex,
                backup_source: Some(FundingSourceType::CrossChain),
                split_funding: false,
                privacy_level: PrivacyLevel::Low,
                estimated_time_minutes: 5,
                estimated_cost: 0.001,
            }
        } else if amount < 1.0 {
            // Medium amounts - balance between cost and privacy
            FundingStrategy {
                primary_source: FundingSourceType::CrossChain,
                backup_source: Some(FundingSourceType::Cex),
                split_funding: false,
                privacy_level: PrivacyLevel::Medium,
                estimated_time_minutes: 15,
                estimated_cost: 0.01,
            }
        } else {
            // Large amounts - prioritize privacy
            FundingStrategy {
                primary_source: FundingSourceType::Mixer,
                backup_source: Some(FundingSourceType::CrossChain),
                split_funding: true,
                privacy_level: PrivacyLevel::High,
                estimated_time_minutes: 45,
                estimated_cost: 0.05,
            }
        }
    }

    /// Auto-fund wallet with optimized strategy
    pub async fn auto_fund_wallet(&mut self, wallet_id: Uuid, amount: f64, chain_id: u64) -> Result<(), WalletError> {
        let request = FundingRequest {
            wallet_id,
            amount,
            chain_id,
            funding_source: FundingSource::Manual, // Will be replaced
            priority: FundingPriority::Normal,
            max_wait_time: 3600, // 1 hour
            privacy_requirements: PrivacyLevel::Medium,
        };

        let strategy = self.optimize_funding_strategy(&request);

        // Try primary source first
        let mut funding_request = request.clone();
        funding_request.funding_source = match strategy.primary_source {
            FundingSourceType::Cex => FundingSource::Cex(CexFundingRequest {
                wallet_id,
                amount,
                chain_id,
                exchange: "binance".to_string(),
                withdraw_method: WithdrawMethod::Direct,
                delay_seconds: 0,
            }),
            FundingSourceType::Mixer => FundingSource::Mixer(MixerFundingRequest {
                wallet_id,
                amount,
                chain_id,
                mixer_type: MixerType::Tornado,
                anonymity_set: 100,
                delay_hours: 1,
            }),
            FundingSourceType::CrossChain => FundingSource::CrossChain(CrossChainFundingRequest {
                wallet_id,
                amount,
                source_chain: 1, // Ethereum
                target_chain: chain_id,
                bridge: "across".to_string(),
                slippage_tolerance: 0.005,
            }),
        };

        // Attempt funding
        match self.fund_wallet(funding_request).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // Try backup source if available
                if let Some(backup_source) = strategy.backup_source {
                    let mut backup_request = request.clone();
                    backup_request.funding_source = match backup_source {
                        FundingSourceType::Cex => FundingSource::Cex(CexFundingRequest {
                            wallet_id,
                            amount,
                            chain_id,
                            exchange: "coinbase".to_string(),
                            withdraw_method: WithdrawMethod::Direct,
                            delay_seconds: 0,
                        }),
                        FundingSourceType::CrossChain => FundingSource::CrossChain(CrossChainFundingRequest {
                            wallet_id,
                            amount,
                            source_chain: 137, // Polygon
                            target_chain: chain_id,
                            bridge: "hop".to_string(),
                            slippage_tolerance: 0.01,
                        }),
                        FundingSourceType::Mixer => FundingSource::Mixer(MixerFundingRequest {
                            wallet_id,
                            amount,
                            chain_id,
                            mixer_type: MixerType::Aztec,
                            anonymity_set: 50,
                            delay_hours: 2,
                        }),
                    };

                    self.fund_wallet(backup_request).await
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Schedule funding for later execution
    pub fn schedule_funding(&mut self, request: FundingRequest, execute_at: chrono::DateTime<chrono::Utc>) -> Result<Uuid, WalletError> {
        let schedule_id = Uuid::new_v4();

        // In a real implementation, you would store this in a database
        // and have a background task that executes scheduled fundings

        Ok(schedule_id)
    }

    /// Cancel scheduled funding
    pub fn cancel_scheduled_funding(&mut self, schedule_id: Uuid) -> Result<(), WalletError> {
        // Implementation would remove from database
        Ok(())
    }

    /// Health check for all funding sources
    pub async fn health_check(&self) -> Result<(), WalletError> {
        // Check CEX funding
        self.cex_funding.health_check().await
            .map_err(|e| WalletError::HealthCheck(format!("CEX funding error: {}", e)))?;

        // Check mixer funding
        self.mixer_funding.health_check().await
            .map_err(|e| WalletError::HealthCheck(format!("Mixer funding error: {}", e)))?;

        // Check cross-chain funding
        self.cross_chain_funding.health_check().await
            .map_err(|e| WalletError::HealthCheck(format!("Cross-chain funding error: {}", e)))?;

        Ok(())
    }

    /// Get funding recommendations based on current market conditions
    pub async fn get_funding_recommendations(&self, amount: f64, chain_id: u64) -> Result<Vec<FundingRecommendation>, WalletError> {
        let mut recommendations = Vec::new();

        // CEX recommendation
        recommendations.push(FundingRecommendation {
            source: FundingSourceType::Cex,
            estimated_cost: amount * 0.001, // 0.1% fee
            estimated_time_minutes: 5,
            privacy_score: 2,
            reliability_score: 9,
            pros: vec![
                "Low fees".to_string(),
                "Fast execution".to_string(),
                "High reliability".to_string(),
            ],
            cons: vec![
                "Low privacy".to_string(),
                "KYC required".to_string(),
            ],
        });

        // Cross-chain recommendation
        recommendations.push(FundingRecommendation {
            source: FundingSourceType::CrossChain,
            estimated_cost: amount * 0.005, // 0.5% fee
            estimated_time_minutes: 15,
            privacy_score: 6,
            reliability_score: 7,
            pros: vec![
                "Good privacy".to_string(),
                "Decentralized".to_string(),
                "Multiple bridge options".to_string(),
            ],
            cons: vec![
                "Higher fees".to_string(),
                "Longer execution time".to_string(),
                "Bridge risks".to_string(),
            ],
        });

        // Mixer recommendation
        recommendations.push(FundingRecommendation {
            source: FundingSourceType::Mixer,
            estimated_cost: amount * 0.01, // 1% fee
            estimated_time_minutes: 60,
            privacy_score: 9,
            reliability_score: 6,
            pros: vec![
                "High privacy".to_string(),
                "Breaks transaction links".to_string(),
            ],
            cons: vec![
                "Highest fees".to_string(),
                "Longest execution time".to_string(),
                "Regulatory risks".to_string(),
            ],
        });

        // Sort by overall score (weighted average of factors)
        recommendations.sort_by(|a, b| {
            let score_a = (a.reliability_score * 0.4 + a.privacy_score * 0.3 + (10.0 - a.estimated_cost / amount * 100.0) * 0.3) as i32;
            let score_b = (b.reliability_score * 0.4 + b.privacy_score * 0.3 + (10.0 - b.estimated_cost / amount * 100.0) * 0.3) as i32;
            score_b.cmp(&score_a)
        });

        Ok(recommendations)
    }
}

/// Funding configuration
#[derive(Debug, Clone)]
pub struct FundingConfig {
    pub cex_config: cex::CexConfig,
    pub mixer_config: mixer::MixerConfig,
    pub cross_chain_config: cross_chain::CrossChainConfig,
    pub default_privacy_level: PrivacyLevel,
    pub max_retry_attempts: u32,
    pub retry_delay_seconds: u64,
}

impl Default for FundingConfig {
    fn default() -> Self {
        Self {
            cex_config: cex::CexConfig::default(),
            mixer_config: mixer::MixerConfig::default(),
            cross_chain_config: cross_chain::CrossChainConfig::default(),
            default_privacy_level: PrivacyLevel::Medium,
            max_retry_attempts: 3,
            retry_delay_seconds: 60,
        }
    }
}

/// Funding statistics
#[derive(Debug, Clone)]
pub struct FundingStats {
    pub total_wallets_funded: usize,
    pub total_amount_funded: f64,
    pub funding_by_source: HashMap<String, f64>,
    pub success_rate: f64,
    pub average_amount: f64,
}

/// Funding strategy recommendation
#[derive(Debug, Clone)]
pub struct FundingStrategy {
    pub primary_source: FundingSourceType,
    pub backup_source: Option<FundingSourceType>,
    pub split_funding: bool,
    pub privacy_level: PrivacyLevel,
    pub estimated_time_minutes: u32,
    pub estimated_cost: f64,
}

/// Funding source types
#[derive(Debug, Clone, PartialEq)]
pub enum FundingSourceType {
    Cex,
    Mixer,
    CrossChain,
}

/// Privacy levels
#[derive(Debug, Clone, PartialEq)]
pub enum PrivacyLevel {
    Low,
    Medium,
    High,
}

/// Funding recommendation
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

/// Funding result
#[derive(Debug, Clone)]
pub struct FundingResult {
    pub wallet_id: Uuid,
    pub success: bool,
    pub error: Option<String>,
    pub transaction_hash: Option<String>,
}

/// Funding record for history tracking
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_funding_manager_creation() {
        let manager = FundingManager::new().await;
        assert!(manager.is_ok());
    }

    #[test]
    fn test_funding_strategy_optimization() {
        let manager = FundingManager::new().await.unwrap();
        let request = FundingRequest {
            wallet_id: Uuid::new_v4(),
            amount: 0.05,
            chain_id: 1,
            funding_source: FundingSource::Manual,
            priority: FundingPriority::Normal,
            max_wait_time: 3600,
            privacy_requirements: PrivacyLevel::Medium,
        };

        let strategy = manager.optimize_funding_strategy(&request);
        assert_eq!(strategy.primary_source, FundingSourceType::Cex);
    }

    #[test]
    fn test_funding_stats() {
        let manager = FundingManager::new().await.unwrap();
        let stats = manager.get_funding_stats();
        assert_eq!(stats.total_wallets_funded, 0);
        assert_eq!(stats.total_amount_funded, 0.0);
    }

    #[tokio::test]
    async fn test_funding_recommendations() {
        let manager = FundingManager::new().await.unwrap();
        let recommendations = manager.get_funding_recommendations(1.0, 1).await.unwrap();
        assert_eq!(recommendations.len(), 3);
    }
}