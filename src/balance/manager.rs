// src/balance/manager.rs
use crate::types::*;
use crate::error::WalletError;
use crate::balance::{BalanceService, BalanceCache, BalanceQuery, BalanceAggregator, BalanceEvent};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Balance manager for tracking wallet balances across chains
pub struct BalanceManager {
    services: HashMap<u64, BalanceService>,
    cache: Arc<RwLock<BalanceCache>>,
    supported_chains: Vec<u64>,
    rpc_endpoints: HashMap<u64, String>,
}

impl BalanceManager {
    /// Create new balance manager
    pub async fn new(supported_chains: &[u64]) -> Result<Self, WalletError> {
        let mut services = HashMap::new();
        let mut rpc_endpoints = HashMap::new();

        // Default RPC endpoints
        let default_endpoints = Self::get_default_rpc_endpoints();

        for &chain_id in supported_chains {
            if let Some(rpc_url) = default_endpoints.get(&chain_id) {
                services.insert(chain_id, BalanceService::new(chain_id, rpc_url.clone()));
                rpc_endpoints.insert(chain_id, rpc_url.clone());
            }
        }

        Ok(Self {
            services,
            cache: Arc::new(RwLock::new(BalanceCache::new(300))), // 5 min cache
            supported_chains: supported_chains.to_vec(),
            rpc_endpoints,
        })
    }

    /// Create with custom RPC endpoints
    pub async fn with_rpc_endpoints(
        chain_endpoints: HashMap<u64, String>
    ) -> Result<Self, WalletError> {
        let mut services = HashMap::new();
        let supported_chains: Vec<u64> = chain_endpoints.keys().copied().collect();

        for (chain_id, rpc_url) in &chain_endpoints {
            services.insert(*chain_id, BalanceService::new(*chain_id, rpc_url.clone()));
        }

        Ok(Self {
            services,
            cache: Arc::new(RwLock::new(BalanceCache::new(300))),
            supported_chains,
            rpc_endpoints: chain_endpoints,
        })
    }

    /// Update balance for a wallet
    pub async fn update_balance(&self, update: BalanceUpdate) -> Result<(), WalletError> {
        let mut cache = self.cache.write().await;

        // Create new balance or update existing
        let balance = Balance {
            chain_id: update.chain_id,
            native_balance: update.native_balance.unwrap_or(0.0),
            token_balances: update.token_updates.clone(),
            last_updated: chrono::Utc::now(),
        };

        cache.insert(update.wallet_id, update.chain_id, balance);
        Ok(())
    }

    /// Get balance for a wallet on specific chain
    pub async fn get_balance(
        &self,
        wallet_id: Uuid,
        chain_id: u64
    ) -> Result<Option<Balance>, WalletError> {
        let cache = self.cache.read().await;
        if let Some(balance) = cache.get(wallet_id, chain_id) {
            return Ok(Some(balance.clone()));
        }
        drop(cache);

        // Fetch from chain if not in cache
        self.fetch_balance(wallet_id, chain_id).await
    }

    /// Get balances for multiple chains
    pub async fn get_balances(
        &self,
        query: BalanceQuery
    ) -> Result<HashMap<u64, Balance>, WalletError> {
        let mut balances = HashMap::new();
        let chains = if query.chain_ids.is_empty() {
            self.supported_chains.clone()
        } else {
            query.chain_ids.clone()
        };

        for chain_id in chains {
            if let Some(balance) = self.get_balance(query.wallet_id, chain_id).await? {
                balances.insert(chain_id, balance);
            }
        }

        Ok(balances)
    }

    /// Get aggregated balance for portfolio view
    pub async fn get_aggregated_balance(
        &self,
        wallet_ids: Vec<Uuid>
    ) -> Result<BalanceAggregator, WalletError> {
        let mut aggregator = BalanceAggregator::new();

        for wallet_id in wallet_ids {
            let query = BalanceQuery::new(wallet_id).chains(self.supported_chains.clone());
            let balances = self.get_balances(query).await?;

            for (chain_id, balance) in balances {
                aggregator.add_balance(chain_id, balance);
            }
        }

        Ok(aggregator)
    }

    /// Fetch balance from blockchain
    async fn fetch_balance(
        &self,
        wallet_id: Uuid,
        chain_id: u64
    ) -> Result<Option<Balance>, WalletError> {
        // This is a mock implementation - replace with actual RPC calls
        // In a real implementation, you would:
        // 1. Get wallet address
        // 2. Make RPC call to get native balance
        // 3. Make RPC calls for token balances
        // 4. Parse and return balance

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let balance = Balance {
            chain_id,
            native_balance: 0.0, // Mock value
            token_balances: HashMap::new(),
            last_updated: chrono::Utc::now(),
        };

        // Cache the result
        let mut cache = self.cache.write().await;
        cache.insert(wallet_id, chain_id, balance.clone());

        Ok(Some(balance))
    }

    /// Fetch native balance via RPC
    async fn fetch_native_balance(
        &self,
        address: &str,
        chain_id: u64
    ) -> Result<f64, WalletError> {
        // Mock implementation - replace with actual RPC call
        // Example: eth_getBalance RPC call
        Ok(0.0)
    }

    /// Fetch token balance via RPC
    async fn fetch_token_balance(
        &self,
        address: &str,
        token_address: &str,
        chain_id: u64
    ) -> Result<f64, WalletError> {
        // Mock implementation - replace with actual contract call
        // Example: ERC20 balanceOf function call
        Ok(0.0)
    }

    /// Refresh all balances for a wallet
    pub async fn refresh_wallet_balances(
        &self,
        wallet_id: Uuid
    ) -> Result<HashMap<u64, Balance>, WalletError> {
        // Invalidate cache
        {
            let mut cache = self.cache.write().await;
            for &chain_id in &self.supported_chains {
                cache.invalidate(wallet_id, chain_id);
            }
        }

        // Fetch fresh balances
        let query = BalanceQuery::new(wallet_id)
            .chains(self.supported_chains.clone())
            .force_refresh();

        self.get_balances(query).await
    }

    /// Batch update balances
    pub async fn batch_update_balances(
        &self,
        updates: Vec<BalanceUpdate>
    ) -> Result<(), WalletError> {
        let mut cache = self.cache.write().await;

        for update in updates {
            let balance = Balance {
                chain_id: update.chain_id,
                native_balance: update.native_balance.unwrap_or(0.0),
                token_balances: update.token_updates.clone(),
                last_updated: chrono::Utc::now(),
            };

            cache.insert(update.wallet_id, update.chain_id, balance);
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, Vec<u64>) {
        let cache = self.cache.read().await;
        (cache.size(), self.supported_chains.clone())
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear_all();
    }

    /// Clear expired cache entries
    pub async fn clear_expired_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear_expired();
    }

    /// Monitor balance changes
    pub async fn monitor_balance_changes(
        &self,
        wallet_id: Uuid,
        chain_id: u64,
        threshold: f64,
    ) -> Result<Vec<BalanceEvent>, WalletError> {
        // Get current balance
        let current = self.get_balance(wallet_id, chain_id).await?;

        // This would typically compare with historical data
        // For now, return empty vec as mock
        Ok(vec![])
    }

    /// Get supported chains
    pub fn get_supported_chains(&self) -> &[u64] {
        &self.supported_chains
    }

    /// Add new chain support
    pub async fn add_chain_support(
        &mut self,
        chain_id: u64,
        rpc_url: String,
    ) -> Result<(), WalletError> {
        if !self.supported_chains.contains(&chain_id) {
            self.supported_chains.push(chain_id);
            self.services.insert(chain_id, BalanceService::new(chain_id, rpc_url.clone()));
            self.rpc_endpoints.insert(chain_id, rpc_url);
        }
        Ok(())
    }

    /// Remove chain support
    pub async fn remove_chain_support(&mut self, chain_id: u64) -> Result<(), WalletError> {
        self.supported_chains.retain(|&x| x != chain_id);
        self.services.remove(&chain_id);
        self.rpc_endpoints.remove(&chain_id);

        // Clear cache for this chain
        // Note: This is a simplified implementation
        // In practice, you'd want to clear specific entries
        self.clear_cache().await;

        Ok(())
    }

    /// Update RPC endpoint for a chain
    pub async fn update_rpc_endpoint(
        &mut self,
        chain_id: u64,
        new_rpc_url: String,
    ) -> Result<(), WalletError> {
        if let Some(service) = self.services.get_mut(&chain_id) {
            *service = BalanceService::new(chain_id, new_rpc_url.clone());
            self.rpc_endpoints.insert(chain_id, new_rpc_url);
        }
        Ok(())
    }

    /// Get balance history (mock implementation)
    pub async fn get_balance_history(
        &self,
        wallet_id: Uuid,
        chain_id: u64,
        days: u32,
    ) -> Result<Vec<(chrono::DateTime<chrono::Utc>, f64)>, WalletError> {
        // Mock implementation - would typically query a database
        // Return empty history for now
        Ok(vec![])
    }

    /// Calculate portfolio value in USD (mock)
    pub async fn calculate_portfolio_value(
        &self,
        wallet_ids: Vec<Uuid>,
    ) -> Result<f64, WalletError> {
        // Mock implementation - would typically:
        // 1. Get all balances
        // 2. Fetch current prices
        // 3. Calculate total USD value
        Ok(0.0)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<(), WalletError> {
        // Check if services are working
        for (chain_id, _service) in &self.services {
            // Mock health check - would typically ping RPC endpoint
            if !self.rpc_endpoints.contains_key(chain_id) {
                return Err(WalletError::HealthCheck(
                    format!("No RPC endpoint for chain {}", chain_id)
                ));
            }
        }

        // Check cache
        let cache = self.cache.read().await;
        if cache.size() > 10000 {
            return Err(WalletError::HealthCheck("Cache too large".to_string()));
        }

        Ok(())
    }

    /// Get default RPC endpoints
    fn get_default_rpc_endpoints() -> HashMap<u64, String> {
        let mut endpoints = HashMap::new();

        // Ethereum Mainnet
        endpoints.insert(1, "https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY".to_string());

        // Polygon
        endpoints.insert(137, "https://polygon-mainnet.g.alchemy.com/v2/YOUR_API_KEY".to_string());

        // Arbitrum
        endpoints.insert(42161, "https://arb-mainnet.g.alchemy.com/v2/YOUR_API_KEY".to_string());

        // Optimism
        endpoints.insert(10, "https://opt-mainnet.g.alchemy.com/v2/YOUR_API_KEY".to_string());

        // BSC
        endpoints.insert(56, "https://bsc-dataseed.binance.org".to_string());

        // Avalanche
        endpoints.insert(43114, "https://api.avax.network/ext/bc/C/rpc".to_string());

        // Fantom
        endpoints.insert(250, "https://rpc.ftm.tools".to_string());

        endpoints
    }

    /// Batch fetch balances for multiple wallets
    pub async fn batch_fetch_balances(
        &self,
        wallet_ids: Vec<Uuid>,
        chain_ids: Vec<u64>,
    ) -> Result<HashMap<Uuid, HashMap<u64, Balance>>, WalletError> {
        let mut results = HashMap::new();

        for wallet_id in wallet_ids {
            let query = BalanceQuery::new(wallet_id).chains(chain_ids.clone());
            let balances = self.get_balances(query).await?;
            results.insert(wallet_id, balances);
        }

        Ok(results)
    }

    /// Get low balance wallets
    pub async fn get_low_balance_wallets(
        &self,
        wallet_ids: Vec<Uuid>,
        threshold: f64,
    ) -> Result<Vec<(Uuid, u64, f64)>, WalletError> {
        let mut low_balance_wallets = Vec::new();

        for wallet_id in wallet_ids {
            for &chain_id in &self.supported_chains {
                if let Some(balance) = self.get_balance(wallet_id, chain_id).await? {
                    if balance.native_balance < threshold {
                        low_balance_wallets.push((wallet_id, chain_id, balance.native_balance));
                    }
                }
            }
        }

        Ok(low_balance_wallets)
    }

    /// Export balances to CSV format
    pub async fn export_balances_csv(
        &self,
        wallet_ids: Vec<Uuid>,
    ) -> Result<String, WalletError> {
        let mut csv_content = String::from("wallet_id,chain_id,native_balance,tokens\n");

        for wallet_id in wallet_ids {
            for &chain_id in &self.supported_chains {
                if let Some(balance) = self.get_balance(wallet_id, chain_id).await? {
                    let tokens = balance.token_balances
                        .iter()
                        .map(|(token, amount)| format!("{}:{}", token, amount))
                        .collect::<Vec<_>>()
                        .join(";");

                    csv_content.push_str(&format!(
                        "{},{},{},{}\n",
                        wallet_id,
                        chain_id,
                        balance.native_balance,
                        tokens
                    ));
                }
            }
        }

        Ok(csv_content)
    }
}

impl Clone for BalanceManager {
    fn clone(&self) -> Self {
        Self {
            services: self.services.clone(),
            cache: Arc::clone(&self.cache),
            supported_chains: self.supported_chains.clone(),
            rpc_endpoints: self.rpc_endpoints.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_balance_manager_creation() {
        let chains = vec![1, 137, 42161];
        let manager = BalanceManager::new(&chains).await.unwrap();

        assert_eq!(manager.get_supported_chains(), &chains);
        assert_eq!(manager.services.len(), 3);
    }

    #[tokio::test]
    async fn test_balance_update() {
        let chains = vec![1];
        let manager = BalanceManager::new(&chains).await.unwrap();

        let wallet_id = Uuid::new_v4();
        let update = BalanceUpdate {
            wallet_id,
            chain_id: 1,
            native_balance: Some(1.5),
            token_updates: HashMap::new(),
        };

        manager.update_balance(update).await.unwrap();

        let balance = manager.get_balance(wallet_id, 1).await.unwrap();
        assert!(balance.is_some());
        assert_eq!(balance.unwrap().native_balance, 1.5);
    }

    #[tokio::test]
    async fn test_balance_query() {
        let chains = vec![1, 137];
        let manager = BalanceManager::new(&chains).await.unwrap();

        let wallet_id = Uuid::new_v4();
        let query = BalanceQuery::new(wallet_id).chains(vec![1, 137]);

        let balances = manager.get_balances(query).await.unwrap();
        assert_eq!(balances.len(), 0); // No balances initially
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let chains = vec![1];
        let manager = BalanceManager::new(&chains).await.unwrap();

        let (size, supported) = manager.get_cache_stats().await;
        assert_eq!(size, 0);
        assert_eq!(supported, vec![1]);

        manager.clear_cache().await;
        manager.clear_expired_cache().await;
    }

    #[tokio::test]
    async fn test_chain_management() {
        let chains = vec![1];
        let mut manager = BalanceManager::new(&chains).await.unwrap();

        // Add new chain
        manager.add_chain_support(137, "https://polygon-rpc.com".to_string()).await.unwrap();
        assert!(manager.get_supported_chains().contains(&137));

        // Remove chain
        manager.remove_chain_support(137).await.unwrap();
        assert!(!manager.get_supported_chains().contains(&137));
    }

    #[tokio::test]
    async fn test_health_check() {
        let chains = vec![1];
        let manager = BalanceManager::new(&chains).await.unwrap();

        let result = manager.health_check().await;
        assert!(result.is_ok());
    }
}