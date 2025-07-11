// src/funding/mixer/fund_mixer.rs
use crate::error::WalletError;
use crate::funding::mixer::types::*;
use crate::types::MixerConfig;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use reqwest::Client;

use super::connectors::{
    TornadoCashConnector, AztecConnector, RailgunConnector, NoirConnector, PenumbraConnector,
};
use crate::network::ProxyManager;

//dynamic anonymity sets - add a method to calculate randomized anonymity sets in fund_mixer.rs to enhance privacy by varying the size of the mixer pool used for each
//transaction

// Action:
// Add calculate_optimal_anonymity_set to funding/mixer/fund_mixer.rs.
// Use it in execute_tornado_mixing and execute_relay_network_mixing to dynamically set anonymity sets.
// Query chain activity (e.g., pool size) via an external API or on-chain data using Alloy.

#[derive(Clone)]
pub struct FundMixer {
    config: MixerConfig,
    mixing_pools: HashMap<u64, MixingPool>,
    active_mixes: HashMap<Uuid, MixingSession>,
    tornado_connectors: HashMap<u64, Box<dyn TornadoConnector>>,
    mixing_history: Vec<MixingRecord>,
    client: Client,
}

impl FundMixer {
    pub async fn new(config: MixerConfig) -> Result<Self, WalletError> {
        let mut tornado_connectors = HashMap::new();
        let proxies = config.proxies.clone().unwrap_or_default(); // Get proxy list from config

        // Initialize connectors for supported chains
        for chain_id in &config.supported_chains {
            if config.tornado_enabled {
                let connector = TornadoCashConnector::new(
                    *chain_id,
                    config.tornado_relayer_url.clone(),
                    config.tornado_private_key.clone(),
                    proxies.clone(),
                )?;
                tornado_connectors.insert(*chain_id, Box::new(connector) as Box<dyn TornadoConnector>);
                log::info!("Initialized TornadoCashConnector for chain {}", chain_id);
            }
            if config.aztec_enabled {
                let connector = AztecConnector::new(
                    *chain_id,
                    config.aztec_relayer_url.clone(),
                    config.aztec_api_key.clone(),
                    proxies.clone(),
                )?;
                tornado_connectors.insert(*chain_id, Box::new(connector) as Box<dyn TornadoConnector>);
                log::info!("Initialized AztecConnector for chain {}", chain_id);
            }
            if config.railgun_enabled {
                let connector = RailgunConnector::new(
                    *chain_id,
                    config.railgun_relayer_url.clone(),
                    config.railgun_api_key.clone(),
                    proxies.clone(),
                )?;
                tornado_connectors.insert(*chain_id, Box::new(connector) as Box<dyn TornadoConnector>);
                log::info!("Initialized RailgunConnector for chain {}", chain_id);
            }
            if config.noir_enabled {
                let connector = NoirConnector::new(
                    *chain_id,
                    config.noir_relayer_url.clone(),
                    config.noir_api_key.clone(),
                    proxies.clone(),
                )?;
                tornado_connectors.insert(*chain_id, Box::new(connector) as Box<dyn TornadoConnector>);
                log::info!("Initialized NoirConnector for chain {}", chain_id);
            }
            if config.penumbra_enabled {
                let connector = PenumbraConnector::new(
                    *chain_id,
                    config.penumbra_relayer_url.clone(),
                    config.penumbra_api_key.clone(),
                    proxies.clone(),
                )?;
                tornado_connectors.insert(*chain_id, Box::new(connector) as Box<dyn TornadoConnector>);
                log::info!("Initialized PenumbraConnector for chain {}", chain_id);
            }
        }

        Ok(Self {
            config,
            mixing_pools: HashMap::new(),
            active_mixes: HashMap::new(),
            tornado_connectors,
            mixing_history: Vec::new(),
            client: Client::new(),
        })
    }

    pub async fn start_mixing(&mut self, request: MixingRequest) -> Result<MixingSession, WalletError> {
        self.validate_mixing_request(&request)?;
        let session_id = Uuid::new_v4();
        let session = MixingSession {
            id: session_id,
            wallet_id: request.wallet_id,
            chain_id: request.chain_id,
            amount: request.amount,
            strategy: request.strategy.clone(),
            status: MixingStatus::Pending,
            created_at: chrono::Utc::now(),
            estimated_completion: chrono::Utc::now() + chrono::Duration::minutes(
                self.get_estimated_mixing_time(&request.strategy)
            ),
            steps: Vec::new(),
            current_step: 0,
        };
        self.active_mixes.insert(session_id, session.clone());
        let mut mixer = self.clone();
        tokio::spawn(async move {
            if let Err(e) = mixer.execute_mixing_strategy(session_id, request).await {
                log::error!("Mixing failed for session {}: {}", session_id, e);
                mixer.mark_mixing_failed(session_id, e.to_string()).await;
            }
        });
        Ok(session)
    }

    async fn execute_mixing_strategy(&mut self, session_id: Uuid, request: MixingRequest) -> Result<(), WalletError> {
        match request.strategy {
            MixingStrategy::TornadoCash => super::strategies::tornado_cash::execute_tornado_mixing(self, session_id, request).await,
            MixingStrategy::LayeredMixing => super::strategies::layered::execute_layered_mixing(self, session_id, request).await,
            MixingStrategy::CrossChainObfuscation => super::strategies::cross_chain::execute_cross_chain_obfuscation(self, session_id, request).await,
            MixingStrategy::RelayNetwork => super::strategies::relay_network::execute_relay_network_mixing(self, session_id, request).await,
            MixingStrategy::CustomPattern => super::strategies::custom::execute_custom_pattern_mixing(self, session_id, request).await,
            MixingStrategy::Noir => super::strategies::noir::execute_noir_mixing(self, session_id, request).await,
            MixingStrategy::Penumbra => super::strategies::penumbra::execute_penumbra_mixing(self, session_id, request).await,
        }
    }

    /// Calculate optimal anonymity set based on amount and chain activity
    pub async fn calculate_optimal_anonymity_set(&self, amount: f64, chain_id: u64) -> u32 {
        let base_set = 100;
        let multiplier = (amount / 0.1).log10().max(1.0) as u32;
        let random_variation = rand::thread_rng().gen_range(-20..20);

        let pool_size = self.query_pool_size(chain_id).await.unwrap_or(100);
        let adjusted_set = (base_set * multiplier + random_variation).clamp(50, pool_size as u32);
        adjusted_set
    }

    /// Query pool size from an external API or on-chain data
    async fn query_pool_size(&self, chain_id: u64) -> Result<usize, WalletError> {
        let url = format!("https://api.tornadocash.eth/pool_size?chain_id={}", chain_id);
        let response = self.client.get(&url).send().await
            .map_err(|e| WalletError::MixingError(format!("Failed to query pool size: {}", e)))?;
        let size: usize = response.json().await
            .map_err(|e| WalletError::MixingError(format!("Failed to parse pool size: {}", e)))?;
        Ok(size)
    }

    /// Example execute_tornado_mixing using dynamic anonymity set
    pub async fn execute_tornado_mixing(&mut self, session_id: Uuid, request: MixingRequest) -> Result<(), WalletError> {
        let tornado = self.tornado_connectors.get(&request.chain_id)
            .ok_or_else(|| WalletError::MixingError("Tornado Cash not supported on this chain".to_string()))?;

        self.update_mixing_status(session_id, MixingStatus::InProgress).await;

        let anonymity_set = self.calculate_optimal_anonymity_set(request.amount, request.chain_id).await;
        self.add_mixing_step(session_id, MixingStep {
            step_type: MixingStepType::TornadoDeposit,
            status: StepStatus::InProgress,
            transaction_hash: None,
            amount: request.amount,
            timestamp: chrono::Utc::now(),
        }).await;

        let deposit_result = tornado.deposit(request.amount, request.wallet_id, anonymity_set).await
            .map_err(|e| WalletError::MixingError(format!("Tornado deposit failed: {}", e)))?;

        self.update_step_status(session_id, 0, StepStatus::Completed, Some(deposit_result.tx_hash.clone())).await;

        let wait_time = self.calculate_optimal_wait_time(request.amount, request.chain_id).await;
        sleep(Duration::from_secs(wait_time)).await;
        Ok(())
    }

    fn calculate_split_amounts(&self, total: f64, count: usize) -> Vec<f64> {
        let mut amounts = Vec::new();
        let mut remaining = total;
        for i in 0..count {
            if i == count - 1 {
                amounts.push(remaining);
            } else {
                let ratio = rand::thread_rng().gen_range(0.1..0.5);
                let noise = rand::thread_rng().gen_range(-0.01..0.01);
                let amount = (remaining * ratio * (1.0 + noise)).max(0.0);
                amounts.push(amount);
                remaining -= amount;
            }
        }
        amounts
    }

    // Placeholder methods (implement as needed)
    async fn update_mixing_status(&mut self, _session_id: Uuid, _status: MixingStatus) {
        // Implement status update logic
    }

    async fn add_mixing_step(&mut self, _session_id: Uuid, _step: MixingStep) {
        // Implement step addition logic
    }

    async fn update_step_status(&mut self, _session_id: Uuid, _step_index: usize, _status: StepStatus, _tx_hash: Option<String>) {
        // Implement step status update logic
    }

    async fn mark_mixing_failed(&mut self, _session_id: Uuid, _error: String) {
        // Implement failure handling
    }

    fn get_estimated_mixing_time(&self, _strategy: &MixingStrategy) -> i64 {
        60 // Placeholder: 60 minutes
    }

    fn validate_mixing_request(&self, _request: &MixingRequest) -> Result<(), WalletError> {
        Ok(()) // Placeholder validation
    }

    async fn calculate_optimal_wait_time(&self, _amount: f64, _chain_id: u64) -> u64 {
        3600 // Placeholder: 1 hour
    }
}

// ... (Other methods: validate_mixing_request, get_estimated_mixing_time, etc.)
