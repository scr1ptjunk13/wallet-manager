pub mod encryption;

use crate::error::{WalletError, WalletResult};
use encryption::WalletEncryption;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Security manager for wallet operations
#[derive(Clone)]
pub struct SecurityManager {
    encryption: Arc<WalletEncryption>,
    config: SecurityConfig,
}

/// Security configuration
#[derive(Clone, Debug)]
pub struct SecurityConfig {
    pub encryption_key: [u8; 32],
    pub enable_key_rotation: bool,
    pub max_decrypt_attempts: u32,
    pub security_level: SecurityLevel,
}

/// Security levels for different operations
#[derive(Clone, Debug, PartialEq)]
pub enum SecurityLevel {
    /// Basic security for testing
    Basic,
    /// Standard security for production
    Standard,
    /// High security for sensitive operations
    High,
    /// Maximum security for critical operations
    Maximum,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(encryption_key: [u8; 32]) -> WalletResult<Self> {
        let config = SecurityConfig {
            encryption_key,
            enable_key_rotation: false,
            max_decrypt_attempts: 3,
            security_level: SecurityLevel::Standard,
        };

        let encryption = Arc::new(WalletEncryption::new(config.clone())?);

        Ok(Self {
            encryption,
            config,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: SecurityConfig) -> WalletResult<Self> {
        let encryption = Arc::new(WalletEncryption::new(config.clone())?);

        Ok(Self {
            encryption,
            config,
        })
    }

    /// Encrypt private key
    pub async fn encrypt_private_key(&self, private_key: &str) -> WalletResult<String> {
        self.encryption.encrypt_private_key(private_key).await
    }

    /// Decrypt private key
    pub async fn decrypt_private_key(&self, encrypted_private_key: &str) -> WalletResult<String> {
        self.encryption.decrypt_private_key(encrypted_private_key).await
    }

    /// Encrypt arbitrary data
    pub async fn encrypt_data(&self, data: &[u8]) -> WalletResult<Vec<u8>> {
        self.encryption.encrypt_data(data).await
    }

    /// Decrypt arbitrary data
    pub async fn decrypt_data(&self, encrypted_data: &[u8]) -> WalletResult<Vec<u8>> {
        self.encryption.decrypt_data(encrypted_data).await
    }

    /// Validate private key format
    pub fn validate_private_key(&self, private_key: &str) -> WalletResult<()> {
        // Remove 0x prefix if present
        let key = private_key.strip_prefix("0x").unwrap_or(private_key);

        // Check length (64 hex characters for 32 bytes)
        if key.len() != 64 {
            return Err(WalletError::InvalidPrivateKey);
        }

        // Check if all characters are valid hex
        if !key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(WalletError::InvalidPrivateKey);
        }

        Ok(())
    }

    /// Validate Ethereum address format
    pub fn validate_address(&self, address: &str) -> WalletResult<()> {
        // Remove 0x prefix if present
        let addr = address.strip_prefix("0x").unwrap_or(address);

        // Check length (40 hex characters for 20 bytes)
        if addr.len() != 40 {
            return Err(WalletError::InvalidAddress(address.to_string()));
        }

        // Check if all characters are valid hex
        if !addr.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(WalletError::InvalidAddress(address.to_string()));
        }

        Ok(())
    }

    /// Generate secure random bytes
    pub fn generate_random_bytes(&self, length: usize) -> WalletResult<Vec<u8>> {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut bytes = vec![0u8; length];
        rng.fill_bytes(&mut bytes);
        Ok(bytes)
    }

    /// Hash data using SHA-256
    pub fn hash_data(&self, data: &[u8]) -> WalletResult<[u8; 32]> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        Ok(hasher.finalize().into())
    }

    /// Securely wipe sensitive data from memory
    pub fn secure_wipe(&self, data: &mut [u8]) {
        use zeroize::Zeroize;
        data.zeroize();
    }

    /// Check if operation is allowed based on security level
    pub fn check_security_level(&self, required_level: SecurityLevel) -> WalletResult<()> {
        let current_level_value = match self.config.security_level {
            SecurityLevel::Basic => 1,
            SecurityLevel::Standard => 2,
            SecurityLevel::High => 3,
            SecurityLevel::Maximum => 4,
        };

        let required_level_value = match required_level {
            SecurityLevel::Basic => 1,
            SecurityLevel::Standard => 2,
            SecurityLevel::High => 3,
            SecurityLevel::Maximum => 4,
        };

        if current_level_value < required_level_value {
            return Err(WalletError::SecurityCheckFailed(
                format!("Required security level {:?}, current level {:?}",
                        required_level, self.config.security_level)
            ));
        }

        Ok(())
    }

    /// Perform security audit
    pub async fn security_audit(&self) -> WalletResult<SecurityAuditReport> {
        let mut report = SecurityAuditReport::default();

        // Check encryption key strength
        if self.config.encryption_key.iter().all(|&b| b == 0) {
            report.vulnerabilities.push("Weak encryption key detected".to_string());
        }

        // Check security level
        if self.config.security_level == SecurityLevel::Basic {
            report.warnings.push("Basic security level in use".to_string());
        }

        // Check key rotation
        if !self.config.enable_key_rotation {
            report.recommendations.push("Consider enabling key rotation".to_string());
        }

        report.passed = report.vulnerabilities.is_empty();
        Ok(report)
    }

    /// Health check for security systems
    pub async fn health_check(&self) -> WalletResult<()> {
        // Test encryption/decryption
        let test_data = "security_health_check";
        let encrypted = self.encrypt_private_key(test_data).await?;
        let decrypted = self.decrypt_private_key(&encrypted).await?;

        if decrypted != test_data {
            return Err(WalletError::SecurityCheckFailed(
                "Encryption/decryption test failed".to_string()
            ));
        }

        // Check security configuration
        self.check_security_level(SecurityLevel::Basic)?;

        Ok(())
    }

    /// Get security configuration
    pub fn get_config(&self) -> &SecurityConfig {
        &self.config
    }

    /// Update security configuration
    pub async fn update_config(&mut self, new_config: SecurityConfig) -> WalletResult<()> {
        // Validate new configuration
        if new_config.encryption_key.iter().all(|&b| b == 0) {
            return Err(WalletError::InvalidConfiguration(
                "Invalid encryption key".to_string()
            ));
        }

        // Update encryption with new key if changed
        if new_config.encryption_key != self.config.encryption_key {
            self.encryption = Arc::new(WalletEncryption::new(new_config.clone())?);
        }

        self.config = new_config;
        Ok(())
    }
}

/// Security audit report
#[derive(Debug, Default)]
pub struct SecurityAuditReport {
    pub passed: bool,
    pub vulnerabilities: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

impl SecurityAuditReport {
    /// Check if audit passed without any issues
    pub fn is_secure(&self) -> bool {
        self.passed && self.vulnerabilities.is_empty()
    }

    /// Get severity score (0-10, higher is worse)
    pub fn severity_score(&self) -> u8 {
        let vuln_score = self.vulnerabilities.len() as u8 * 3;
        let warn_score = self.warnings.len() as u8 * 1;
        std::cmp::min(vuln_score + warn_score, 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_manager_creation() {
        let key = [1u8; 32];
        let manager = SecurityManager::new(key).unwrap();
        assert_eq!(manager.config.encryption_key, key);
    }

    #[tokio::test]
    async fn test_private_key_validation() {
        let key = [1u8; 32];
        let manager = SecurityManager::new(key).unwrap();

        // Valid private key
        assert!(manager.validate_private_key("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").is_ok());

        // Invalid private key (too short)
        assert!(manager.validate_private_key("0123456789abcdef").is_err());

        // Invalid private key (non-hex)
        assert!(manager.validate_private_key("gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg").is_err());
    }

    #[tokio::test]
    async fn test_address_validation() {
        let key = [1u8; 32];
        let manager = SecurityManager::new(key).unwrap();

        // Valid address
        assert!(manager.validate_address("0x742d35Cc6634C0532925a3b8d4C9db4CA4b4c73f").is_ok());

        // Invalid address (too short)
        assert!(manager.validate_address("0x742d35Cc").is_err());

        // Invalid address (non-hex)
        assert!(manager.validate_address("0xgggggggggggggggggggggggggggggggggggggggg").is_err());
    }

    #[tokio::test]
    async fn test_encryption_decryption() {
        let key = [1u8; 32];
        let manager = SecurityManager::new(key).unwrap();

        let test_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let encrypted = manager.encrypt_private_key(test_key).await.unwrap();
        let decrypted = manager.decrypt_private_key(&encrypted).await.unwrap();

        assert_eq!(test_key, decrypted);
    }
}