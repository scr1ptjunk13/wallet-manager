// src/balance/mod.rs
pub mod manager;

pub use manager::BalanceManager;

use crate::types::*;
use crate::error::WalletError;
use std::collections::HashMap;
use uuid::Uuid;

/// Balance tracking service
#[derive(Debug, Clone)]
pub struct BalanceService {
    pub chain_id: u64,
    pub rpc_url: String,
    pub timeout_ms: u64,
    pub retry_count: u32,
}

impl BalanceService {
    pub fn new(chain_id: u64, rpc_url: String) -> Self {
        Self {
            chain_id,
            rpc_url,
            timeout_ms: 10000,
            retry_count: 3,
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_retry(mut self, retry_count: u32) -> Self {
        self.retry_count = retry_count;
        self
    }
}

/// Balance cache for storing wallet balances
#[derive(Debug, Clone)]
pub struct BalanceCache {
    cache: HashMap<String, CachedBalance>,
    ttl_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct CachedBalance {
    pub balance: Balance,
    pub cached_at: chrono::DateTime<chrono::Utc>,
}

impl BalanceCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: HashMap::new(),
            ttl_seconds,
        }
    }

    pub fn insert(&mut self, wallet_id: Uuid, chain_id: u64, balance: Balance) {
        let key = format!("{}:{}", wallet_id, chain_id);
        self.cache.insert(key, CachedBalance {
            balance,
            cached_at: chrono::Utc::now(),
        });
    }

    pub fn get(&self, wallet_id: Uuid, chain_id: u64) -> Option<&Balance> {
        let key = format!("{}:{}", wallet_id, chain_id);
        if let Some(cached) = self.cache.get(&key) {
            let age = chrono::Utc::now().signed_duration_since(cached.cached_at);
            if age.num_seconds() < self.ttl_seconds as i64 {
                return Some(&cached.balance);
            }
        }
        None
    }

    pub fn invalidate(&mut self, wallet_id: Uuid, chain_id: u64) {
        let key = format!("{}:{}", wallet_id, chain_id);
        self.cache.remove(&key);
    }

    pub fn clear_expired(&mut self) {
        let now = chrono::Utc::now();
        self.cache.retain(|_, cached| {
            let age = now.signed_duration_since(cached.cached_at);
            age.num_seconds() < self.ttl_seconds as i64
        });
    }

    pub fn clear_all(&mut self) {
        self.cache.clear();
    }

    pub fn size(&self) -> usize {
        self.cache.len()
    }
}

/// Balance query builder
#[derive(Debug, Clone)]
pub struct BalanceQuery {
    pub wallet_id: Uuid,
    pub chain_ids: Vec<u64>,
    pub include_tokens: bool,
    pub token_addresses: Vec<String>,
    pub force_refresh: bool,
}

impl BalanceQuery {
    pub fn new(wallet_id: Uuid) -> Self {
        Self {
            wallet_id,
            chain_ids: vec![],
            include_tokens: false,
            token_addresses: vec![],
            force_refresh: false,
        }
    }

    pub fn chain(mut self, chain_id: u64) -> Self {
        self.chain_ids.push(chain_id);
        self
    }

    pub fn chains(mut self, chain_ids: Vec<u64>) -> Self {
        self.chain_ids.extend(chain_ids);
        self
    }

    pub fn with_tokens(mut self) -> Self {
        self.include_tokens = true;
        self
    }

    pub fn token(mut self, token_address: String) -> Self {
        self.token_addresses.push(token_address);
        self
    }

    pub fn tokens(mut self, token_addresses: Vec<String>) -> Self {
        self.token_addresses.extend(token_addresses);
        self
    }

    pub fn force_refresh(mut self) -> Self {
        self.force_refresh = true;
        self
    }
}

/// Balance aggregator for portfolio view
#[derive(Debug, Clone)]
pub struct BalanceAggregator {
    pub total_usd_value: f64,
    pub balances_by_chain: HashMap<u64, Balance>,
    pub token_totals: HashMap<String, f64>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl BalanceAggregator {
    pub fn new() -> Self {
        Self {
            total_usd_value: 0.0,
            balances_by_chain: HashMap::new(),
            token_totals: HashMap::new(),
            last_updated: chrono::Utc::now(),
        }
    }

    pub fn add_balance(&mut self, chain_id: u64, balance: Balance) {
        self.balances_by_chain.insert(chain_id, balance.clone());

        // Add to token totals
        for (token, amount) in &balance.token_balances {
            *self.token_totals.entry(token.clone()).or_insert(0.0) += amount;
        }

        self.last_updated = chrono::Utc::now();
    }

    pub fn get_chain_balance(&self, chain_id: u64) -> Option<&Balance> {
        self.balances_by_chain.get(&chain_id)
    }

    pub fn get_token_total(&self, token: &str) -> f64 {
        self.token_totals.get(token).copied().unwrap_or(0.0)
    }

    pub fn supported_chains(&self) -> Vec<u64> {
        self.balances_by_chain.keys().copied().collect()
    }

    pub fn clear(&mut self) {
        self.total_usd_value = 0.0;
        self.balances_by_chain.clear();
        self.token_totals.clear();
        self.last_updated = chrono::Utc::now();
    }
}

/// Balance monitoring configuration
#[derive(Debug, Clone)]
pub struct BalanceMonitorConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub alert_threshold: f64,
    pub notification_webhook: Option<String>,
}

impl Default for BalanceMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: 300, // 5 minutes
            alert_threshold: 0.1,  // 10% change
            notification_webhook: None,
        }
    }
}

/// Balance event types
#[derive(Debug, Clone)]
pub enum BalanceEvent {
    Updated {
        wallet_id: Uuid,
        chain_id: u64,
        old_balance: f64,
        new_balance: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    LowBalance {
        wallet_id: Uuid,
        chain_id: u64,
        balance: f64,
        threshold: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    Error {
        wallet_id: Uuid,
        chain_id: u64,
        error: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

/// Balance utilities
pub mod utils {
    use super::*;

    /// Format balance for display
    pub fn format_balance(balance: f64, decimals: u8) -> String {
        format!("{:.prec$}", balance, prec = decimals as usize)
    }

    /// Convert wei to ether
    pub fn wei_to_ether(wei: u64) -> f64 {
        wei as f64 / 1e18
    }

    /// Convert ether to wei
    pub fn ether_to_wei(ether: f64) -> u64 {
        (ether * 1e18) as u64
    }

    /// Check if balance is considered "dust"
    pub fn is_dust(balance: f64, threshold: f64) -> bool {
        balance < threshold
    }

    /// Calculate percentage change
    pub fn calculate_change_percentage(old_balance: f64, new_balance: f64) -> f64 {
        if old_balance == 0.0 {
            return 0.0;
        }
        ((new_balance - old_balance) / old_balance) * 100.0
    }

    /// Get chain name by ID
    pub fn get_chain_name(chain_id: u64) -> &'static str {
        match chain_id {
            1 => "Ethereum",
            137 => "Polygon",
            42161 => "Arbitrum",
            10 => "Optimism",
            56 => "BSC",
            43114 => "Avalanche",
            250 => "Fantom",
            _ => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_cache() {
        let mut cache = BalanceCache::new(300); // 5 minutes
        let wallet_id = Uuid::new_v4();
        let chain_id = 1;

        let balance = Balance {
            chain_id,
            native_balance: 1.5,
            token_balances: HashMap::new(),
            last_updated: chrono::Utc::now(),
        };

        cache.insert(wallet_id, chain_id, balance.clone());

        let cached = cache.get(wallet_id, chain_id);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().chain_id, chain_id);
    }

    #[test]
    fn test_balance_query_builder() {
        let wallet_id = Uuid::new_v4();
        let query = BalanceQuery::new(wallet_id)
            .chain(1)
            .chain(137)
            .with_tokens()
            .token("0x123".to_string())
            .force_refresh();

        assert_eq!(query.wallet_id, wallet_id);
        assert_eq!(query.chain_ids, vec![1, 137]);
        assert!(query.include_tokens);
        assert_eq!(query.token_addresses, vec!["0x123"]);
        assert!(query.force_refresh);
    }

    #[test]
    fn test_balance_aggregator() {
        let mut aggregator = BalanceAggregator::new();

        let balance = Balance {
            chain_id: 1,
            native_balance: 1.5,
            token_balances: {
                let mut tokens = HashMap::new();
                tokens.insert("USDC".to_string(), 1000.0);
                tokens
            },
            last_updated: chrono::Utc::now(),
        };

        aggregator.add_balance(1, balance);

        assert_eq!(aggregator.supported_chains(), vec![1]);
        assert_eq!(aggregator.get_token_total("USDC"), 1000.0);
    }

    #[test]
    fn test_balance_utilities() {
        assert_eq!(utils::format_balance(1.23456, 2), "1.23");
        assert_eq!(utils::wei_to_ether(1_000_000_000_000_000_000), 1.0);
        assert_eq!(utils::ether_to_wei(1.0), 1_000_000_000_000_000_000);
        assert!(utils::is_dust(0.001, 0.01));
        assert_eq!(utils::calculate_change_percentage(100.0, 110.0), 10.0);
        assert_eq!(utils::get_chain_name(1), "Ethereum");
    }
}