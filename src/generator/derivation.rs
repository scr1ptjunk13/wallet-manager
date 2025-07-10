// src/generator/derivation.rs
use crate::error::WalletError;
use std::str::FromStr;

/// Derivation path for HD wallets
#[derive(Debug, Clone, PartialEq)]
pub struct DerivationPath {
    pub purpose: u32,
    pub coin_type: u32,
    pub account: u32,
    pub change: u32,
    pub index: u32,
}

impl DerivationPath {
    /// Create a new derivation path
    pub fn new(purpose: u32, coin_type: u32, account: u32, change: u32, index: u32) -> Self {
        Self {
            purpose,
            coin_type,
            account,
            change,
            index,
        }
    }

    /// Create Ethereum derivation path (BIP44)
    pub fn ethereum(account: u32, index: u32) -> Self {
        Self::new(44, 60, account, 0, index)
    }

    /// Create Bitcoin derivation path (BIP44)
    pub fn bitcoin(account: u32, index: u32) -> Self {
        Self::new(44, 0, account, 0, index)
    }

    /// Create custom derivation path
    pub fn custom(coin_type: u32, account: u32, index: u32) -> Self {
        Self::new(44, coin_type, account, 0, index)
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        format!(
            "m/{}'/{}'/{}'/{}/{}",
            self.purpose, self.coin_type, self.account, self.change, self.index
        )
    }

    /// Get the next derivation path by incrementing index
    pub fn next(&self) -> Self {
        Self {
            purpose: self.purpose,
            coin_type: self.coin_type,
            account: self.account,
            change: self.change,
            index: self.index + 1,
        }
    }

    /// Get derivation path with specific index
    pub fn with_index(&self, index: u32) -> Self {
        Self {
            purpose: self.purpose,
            coin_type: self.coin_type,
            account: self.account,
            change: self.change,
            index,
        }
    }
}

impl FromStr for DerivationPath {
    type Err = WalletError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('/').collect();

        if parts.len() != 6 || parts[0] != "m" {
            return Err(WalletError::InvalidDerivationPath(s.to_string()));
        }

        let purpose = parts[1]
            .trim_end_matches('\'')
            .parse::<u32>()
            .map_err(|_| WalletError::InvalidDerivationPath(s.to_string()))?;

        let coin_type = parts[2]
            .trim_end_matches('\'')
            .parse::<u32>()
            .map_err(|_| WalletError::InvalidDerivationPath(s.to_string()))?;

        let account = parts[3]
            .trim_end_matches('\'')
            .parse::<u32>()
            .map_err(|_| WalletError::InvalidDerivationPath(s.to_string()))?;

        let change = parts[4]
            .parse::<u32>()
            .map_err(|_| WalletError::InvalidDerivationPath(s.to_string()))?;

        let index = parts[5]
            .parse::<u32>()
            .map_err(|_| WalletError::InvalidDerivationPath(s.to_string()))?;

        Ok(DerivationPath {
            purpose,
            coin_type,
            account,
            change,
            index,
        })
    }
}

/// Derivation manager for generating wallet addresses
pub struct DerivationManager {
    base_path: DerivationPath,
    current_index: u32,
}

impl DerivationManager {
    /// Create new derivation manager
    pub fn new(base_path: &str) -> Result<Self, WalletError> {
        let path = DerivationPath::from_str(base_path)?;
        Ok(Self {
            base_path: path,
            current_index: 0,
        })
    }

    /// Create Ethereum derivation manager
    pub fn ethereum(account: u32) -> Self {
        Self {
            base_path: DerivationPath::ethereum(account, 0),
            current_index: 0,
        }
    }

    /// Get next derivation path
    pub fn next_path(&mut self) -> DerivationPath {
        let path = self.base_path.with_index(self.current_index);
        self.current_index += 1;
        path
    }

    /// Get derivation path at specific index
    pub fn path_at_index(&self, index: u32) -> DerivationPath {
        self.base_path.with_index(index)
    }

    /// Get current index
    pub fn current_index(&self) -> u32 {
        self.current_index
    }

    /// Reset index to 0
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Set current index
    pub fn set_index(&mut self, index: u32) {
        self.current_index = index;
    }

    /// Generate batch of derivation paths
    pub fn generate_batch(&mut self, count: usize) -> Vec<DerivationPath> {
        let mut paths = Vec::with_capacity(count);
        for _ in 0..count {
            paths.push(self.next_path());
        }
        paths
    }

    /// Get derivation paths for specific range
    pub fn get_range(&self, start: u32, end: u32) -> Vec<DerivationPath> {
        (start..end)
            .map(|i| self.base_path.with_index(i))
            .collect()
    }
}

/// Utility functions for derivation
pub mod utils {
    use super::*;

    /// Validate derivation path string
    pub fn validate_path(path: &str) -> Result<(), WalletError> {
        DerivationPath::from_str(path)?;
        Ok(())
    }

    /// Generate standard Ethereum paths
    pub fn generate_ethereum_paths(account: u32, count: usize) -> Vec<DerivationPath> {
        (0..count as u32)
            .map(|i| DerivationPath::ethereum(account, i))
            .collect()
    }

    /// Generate paths for multiple accounts
    pub fn generate_multi_account_paths(
        coin_type: u32,
        accounts: &[u32],
        addresses_per_account: usize,
    ) -> Vec<DerivationPath> {
        let mut paths = Vec::new();

        for &account in accounts {
            for i in 0..addresses_per_account as u32 {
                paths.push(DerivationPath::custom(coin_type, account, i));
            }
        }

        paths
    }

    /// Parse derivation path components
    pub fn parse_components(path: &str) -> Result<(u32, u32, u32, u32, u32), WalletError> {
        let dp = DerivationPath::from_str(path)?;
        Ok((dp.purpose, dp.coin_type, dp.account, dp.change, dp.index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derivation_path_creation() {
        let path = DerivationPath::ethereum(0, 5);
        assert_eq!(path.to_string(), "m/44'/60'/0'/0/5");
    }

    #[test]
    fn test_derivation_path_parsing() {
        let path_str = "m/44'/60'/0'/0/5";
        let path = DerivationPath::from_str(path_str).unwrap();
        assert_eq!(path.purpose, 44);
        assert_eq!(path.coin_type, 60);
        assert_eq!(path.account, 0);
        assert_eq!(path.change, 0);
        assert_eq!(path.index, 5);
    }

    #[test]
    fn test_derivation_manager() {
        let mut manager = DerivationManager::ethereum(0);
        let path1 = manager.next_path();
        let path2 = manager.next_path();

        assert_eq!(path1.index, 0);
        assert_eq!(path2.index, 1);
        assert_eq!(manager.current_index(), 2);
    }

    #[test]
    fn test_batch_generation() {
        let mut manager = DerivationManager::ethereum(0);
        let paths = manager.generate_batch(3);

        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].index, 0);
        assert_eq!(paths[1].index, 1);
        assert_eq!(paths[2].index, 2);
    }

    #[test]
    fn test_path_validation() {
        assert!(utils::validate_path("m/44'/60'/0'/0/5").is_ok());
        assert!(utils::validate_path("invalid/path").is_err());
    }
}