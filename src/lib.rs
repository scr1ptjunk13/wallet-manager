// src/lib.rs
pub mod types;
pub mod error;
pub mod generator;
pub mod funding;
pub mod balance;
pub mod security;

use crate::types::*;
use crate::error::WalletError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Main wallet manager - your money machine
#[derive(Clone)]
pub struct WalletManager {
    wallets: Arc<RwLock<HashMap<Uuid, Wallet>>>,
    config: WalletConfig,
    generator: generator::WalletGenerator,
    funding: funding::FundingManager,
    balance: balance::BalanceManager,
    security: security::SecurityManager,
}

impl WalletManager {
    /// Create a new wallet manager
    pub async fn new(config: WalletConfig) -> Result<Self, WalletError> {
        let generator = generator::WalletGenerator::new(&config)?;
        let funding = funding::FundingManager::new().await?;
        let balance = balance::BalanceManager::new(&config.supported_chains).await?;
        let security = security::SecurityManager::new(config.encryption_key)?;

        Ok(Self {
            wallets: Arc::new(RwLock::new(HashMap::new())),
            config,
            generator,
            funding,
            balance,
            security,
        })
    }

    /// Generate new wallet
    pub async fn generate_wallet(&self, alias: Option<String>) -> Result<Uuid, WalletError> {
        let wallet = self.generator.generate_wallet(alias).await?;
        let wallet_id = wallet.id;

        let mut wallets = self.wallets.write().await;
        wallets.insert(wallet_id, wallet);

        Ok(wallet_id)
    }

    /// Generate multiple wallets at once
    pub async fn generate_wallets(&self, count: usize) -> Result<Vec<Uuid>, WalletError> {
        let mut wallet_ids = Vec::new();

        for i in 0..count {
            let alias = Some(format!("wallet_{}", i));
            let wallet_id = self.generate_wallet(alias).await?;
            wallet_ids.push(wallet_id);
        }

        Ok(wallet_ids)
    }

    /// Get wallet by ID
    pub async fn get_wallet(&self, wallet_id: Uuid) -> Result<Option<Wallet>, WalletError> {
        let wallets = self.wallets.read().await;
        Ok(wallets.get(&wallet_id).cloned())
    }

    /// Get all wallets
    pub async fn get_all_wallets(&self) -> Result<Vec<Wallet>, WalletError> {
        let wallets = self.wallets.read().await;
        Ok(wallets.values().cloned().collect())
    }

    /// Fund wallet
    pub async fn fund_wallet(&self, request: FundingRequest) -> Result<(), WalletError> {
        self.funding.fund_wallet(request).await
    }

    /// Update wallet balance
    pub async fn update_balance(&self, update: BalanceUpdate) -> Result<(), WalletError> {
        // Update balance tracker
        self.balance.update_balance(update.clone()).await?;

        // Update wallet in memory
        let mut wallets = self.wallets.write().await;
        if let Some(wallet) = wallets.get_mut(&update.wallet_id) {
            if let Some(balance) = wallet.balances.get_mut(&update.chain_id.to_string()) {
                if let Some(native) = update.native_balance {
                    balance.native_balance = native;
                }
                for (token, amount) in update.token_updates {
                    balance.token_balances.insert(token, amount);
                }
                balance.last_updated = chrono::Utc::now();
            }
        }

        Ok(())
    }

    /// Get wallet count
    pub async fn wallet_count(&self) -> usize {
        let wallets = self.wallets.read().await;
        wallets.len()
    }

    /// Get private key (decrypted)
    pub async fn get_private_key(&self, wallet_id: Uuid) -> Result<String, WalletError> {
        let wallets = self.wallets.read().await;
        if let Some(wallet) = wallets.get(&wallet_id) {
            self.security.decrypt_private_key(&wallet.encrypted_private_key).await
        } else {
            Err(WalletError::WalletNotFound(wallet_id))
        }
    }

    /// Health check
    pub async fn health_check(&self) -> Result<(), WalletError> {
        // Check all systems
        self.generator.health_check().await?;
        self.funding.health_check().await?;
        self.balance.health_check().await?;
        self.security.health_check().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wallet_generation() {
        let config = WalletConfig {
            master_seed: "test seed phrase".to_string(),
            derivation_base: "m/44'/60'/0'/0".to_string(),
            encryption_key: [0u8; 32],
            supported_chains: vec![1, 137, 42161],
        };

        let manager = WalletManager::new(config).await.unwrap();
        let wallet_id = manager.generate_wallet(Some("test".to_string())).await.unwrap();

        assert_eq!(manager.wallet_count().await, 1);

        let wallet = manager.get_wallet(wallet_id).await.unwrap();
        assert!(wallet.is_some());
    }
}