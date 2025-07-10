use crate::error::{WalletError, WalletResult};
use super::SecurityConfig;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Encrypted data container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 12],
    pub salt: Option<Vec<u8>>,
    pub version: u8,
}

/// Encryption key with metadata
#[derive(Debug, Clone, ZeroizeOnDrop)]
pub struct EncryptionKey {
    #[zeroize(skip)]
    pub id: String,
    pub key: [u8; 32],
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub usage_count: u64,
}

/// Wallet encryption handler
pub struct WalletEncryption {
    config: SecurityConfig,
    cipher: Aes256Gcm,
    key_store: Arc<RwLock<Vec<EncryptionKey>>>,
    current_key_id: Arc<RwLock<String>>,
}

impl WalletEncryption {
    /// Create a new encryption handler
    pub fn new(config: SecurityConfig) -> WalletResult<Self> {
        let key = Key::<Aes256Gcm>::from_slice(&config.encryption_key);
        let cipher = Aes256Gcm::new(key);

        // Create initial key
        let initial_key = EncryptionKey {
            id: "default".to_string(),
            key: config.encryption_key,
            created_at: chrono::Utc::now(),
            usage_count: 0,
        };

        let key_store = Arc::new(RwLock::new(vec![initial_key]));
        let current_key_id = Arc::new(RwLock::new("default".to_string()));

        Ok(Self {
            config,
            cipher,
            key_store,
            current_key_id,
        })
    }

    /// Encrypt private key with additional metadata
    pub async fn encrypt_private_key(&self, private_key: &str) -> WalletResult<String> {
        let data = private_key.as_bytes();
        let encrypted = self.encrypt_data(data).await?;

        // Encode as base64 for storage
        let encoded = general_purpose::STANDARD.encode(serde_json::to_vec(&encrypted)
            .map_err(|e| WalletError::EncryptionError(e.to_string()))?);

        Ok(encoded)
    }

    /// Decrypt private key
    pub async fn decrypt_private_key(&self, encrypted_private_key: &str) -> WalletResult<String> {
        // Decode from base64
        let decoded = general_purpose::STANDARD.decode(encrypted_private_key)
            .map_err(|e| WalletError::DecryptionError(e.to_string()))?;

        let encrypted_data: EncryptedData = serde_json::from_slice(&decoded)
            .map_err(|e| WalletError::DecryptionError(e.to_string()))?;

        let decrypted = self.decrypt_data_internal(&encrypted_data).await?;

        String::from_utf8(decrypted)
            .map_err(|e| WalletError::DecryptionError(e.to_string()))
    }

    /// Encrypt arbitrary data
    pub async fn encrypt_data(&self, data: &[u8]) -> WalletResult<Vec<u8>> {
        let encrypted_data = self.encrypt_data_internal(data).await?;

        serde_json::to_vec(&encrypted_data)
            .map_err(|e| WalletError::EncryptionError(e.to_string()))
    }

    /// Decrypt arbitrary data
    pub async fn decrypt_data(&self, encrypted_data: &[u8]) -> WalletResult<Vec<u8>> {
        let encrypted_data: EncryptedData = serde_json::from_slice(encrypted_data)
            .map_err(|e| WalletError::DecryptionError(e.to_string()))?;

        self.decrypt_data_internal(&encrypted_data).await
    }

    /// Internal encryption implementation
    async fn encrypt_data_internal(&self, data: &[u8]) -> WalletResult<EncryptedData> {
        // Generate random nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt the data
        let ciphertext = self.cipher.encrypt(&nonce, data)
            .map_err(|e| WalletError::EncryptionError(e.to_string()))?;

        // Update key usage count
        self.increment_key_usage().await?;

        Ok(EncryptedData {
            ciphertext,
            nonce: nonce.into(),
            salt: None,
            version: 1,
        })
    }

    /// Internal decryption implementation
    async fn decrypt_data_internal(&self, encrypted_data: &EncryptedData) -> WalletResult<Vec<u8>> {
        // Check version compatibility
        if encrypted_data.version != 1 {
            return Err(WalletError::DecryptionError(
                format!("Unsupported encryption version: {}", encrypted_data.version)
            ));
        }

        let nonce = Nonce::from_slice(&encrypted_data.nonce);

        // Decrypt the data
        let plaintext = self.cipher.decrypt(nonce, encrypted_data.ciphertext.as_ref())
            .map_err(|e| WalletError::DecryptionError(e.to_string()))?;

        Ok(plaintext)
    }

    /// Encrypt with password-based key derivation
    pub async fn encrypt_with_password(&self, data: &[u8], password: &str) -> WalletResult<EncryptedData> {
        // Generate salt
        let salt = SaltString::generate(&mut OsRng);

        // Derive key from password
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| WalletError::KeyDerivationError(e.to_string()))?;

        // Extract key material
        let derived_key = password_hash.hash.ok_or_else(||
            WalletError::KeyDerivationError("Failed to derive key".to_string()))?;

        // Use first 32 bytes as encryption key
        let mut key_bytes = [0u8; 32];
        let key_data = derived_key.as_bytes();
        let copy_len = std::cmp::min(32, key_data.len());
        key_bytes[..copy_len].copy_from_slice(&key_data[..copy_len]);

        // Create temporary cipher
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let temp_cipher = Aes256Gcm::new(key);

        // Generate nonce and encrypt
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = temp_cipher.encrypt(&nonce, data)
            .map_err(|e| WalletError::EncryptionError(e.to_string()))?;

        // Clean up key material
        let mut key_bytes = key_bytes;
        key_bytes.zeroize();

        Ok(EncryptedData {
            ciphertext,
            nonce: nonce.into(),
            salt: Some(salt.as_bytes().to_vec()),
            version: 1,
        })
    }

    /// Decrypt with password-based key derivation
    pub async fn decrypt_with_password(&self, encrypted_data: &EncryptedData, password: &str) -> WalletResult<Vec<u8>> {
        let salt_bytes = encrypted_data.salt.as_ref()
            .ok_or_else(|| WalletError::DecryptionError("Missing salt for password-based decryption".to_string()))?;

        // Recreate salt
        let salt = SaltString::from_b64(
            &general_purpose::STANDARD.encode(salt_bytes)
        ).map_err(|e| WalletError::DecryptionError(e.to_string()))?;

        // Derive key from password
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| WalletError::KeyDerivationError(e.to_string()))?;

        // Extract key material
        let derived_key = password_hash.hash.ok_or_else(||
            WalletError::KeyDerivationError("Failed to derive key".to_string()))?;

        // Use first 32 bytes as encryption key
        let mut key_bytes = [0u8; 32];
        let key_data = derived_key.as_bytes();
        let copy_len = std::cmp::min(32, key_data.len());
        key_bytes[..copy_len].copy_from_slice(&key_data[..copy_len]);

        // Create temporary cipher
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let temp_cipher = Aes256Gcm::new(key);

        // Decrypt
        let nonce = Nonce::from_slice(&encrypted_data.nonce);
        let plaintext = temp_cipher.decrypt(nonce, encrypted_data.ciphertext.as_ref())
            .map_err(|e| WalletError::DecryptionError(e.to_string()))?;

        // Clean up key material
        let mut key_bytes = key_bytes;
        key_bytes.zeroize();

        Ok(plaintext)
    }

    /// Rotate encryption key
    pub async fn rotate_key(&self) -> WalletResult<String> {
        if !self.config.enable_key_rotation {
            return Err(WalletError::SecurityCheckFailed(
                "Key rotation is disabled".to_string()
            ));
        }

        // Generate new key
        let mut new_key = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut new_key);

        let new_key_id = format!("key_{}", chrono::Utc::now().timestamp());

        // Create new encryption key
        let encryption_key = EncryptionKey {
            id: new_key_id.clone(),
            key: new_key,
            created_at: chrono::Utc::now(),
            usage_count: 0,
        };

        // Add to key store
        let mut key_store = self.key_store.write().await;
        key_store.push(encryption_key);

        // Update current key
        let mut current_key_id = self.current_key_id.write().await;
        *current_key_id = new_key_id.clone();

        // Clean up old key material
        let mut new_key = new_key;
        new_key.zeroize();

        Ok(new_key_id)
    }

    /// Get current key ID
    pub async fn get_current_key_id(&self) -> String {
        let current_key_id = self.current_key_id.read().await;
        current_key_id.clone()
    }

    /// Get key usage statistics
    pub async fn get_key_stats(&self) -> WalletResult<Vec<KeyStats>> {
        let key_store = self.key_store.read().await;
        let stats = key_store.iter().map(|key| KeyStats {
            id: key.id.clone(),
            created_at: key.created_at,
            usage_count: key.usage_count,
        }).collect();

        Ok(stats)
    }

    /// Increment key usage count
    async fn increment_key_usage(&self) -> WalletResult<()> {
        let current_key_id = self.current_key_id.read().await;
        let mut key_store = self.key_store.write().await;

        if let Some(key) = key_store.iter_mut().find(|k| k.id == *current_key_id) {
            key.usage_count += 1;
        }

        Ok(())
    }

    /// Clean up old keys (keep only recent ones)
    pub async fn cleanup_old_keys(&self, keep_count: usize) -> WalletResult<usize> {
        let mut key_store = self.key_store.write().await;
        let current_key_id = self.current_key_id.read().await;

        // Sort by creation time (newest first)
        key_store.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Keep current key and most recent keys
        let mut keys_to_keep = Vec::new();
        let mut removed_count = 0;

        for (i, key) in key_store.iter().enumerate() {
            if i < keep_count || key.id == *current_key_id {
                keys_to_keep.push(key.clone());
            } else {
                removed_count += 1;
            }
        }

        *key_store = keys_to_keep;
        Ok(removed_count)
    }

    /// Backup encryption keys (encrypted with master password)
    pub async fn backup_keys(&self, master_password: &str) -> WalletResult<String> {
        let key_store = self.key_store.read().await;
        let backup_data = KeyBackup {
            keys: key_store.clone(),
            created_at: chrono::Utc::now(),
            version: 1,
        };

        let serialized = serde_json::to_vec(&backup_data)
            .map_err(|e| WalletError::SerializationError(e.to_string()))?;

        let encrypted = self.encrypt_with_password(&serialized, master_password).await?;
        let encoded = general_purpose::STANDARD.encode(serde_json::to_vec(&encrypted)
            .map_err(|e| WalletError::EncryptionError(e.to_string()))?);

        Ok(encoded)
    }

    /// Restore encryption keys from backup
    pub async fn restore_keys(&self, backup_data: &str, master_password: &str) -> WalletResult<()> {
        let decoded = general_purpose::STANDARD.decode(backup_data)
            .map_err(|e| WalletError::DecryptionError(e.to_string()))?;

        let encrypted_data: EncryptedData = serde_json::from_slice(&decoded)
            .map_err(|e| WalletError::DecryptionError(e.to_string()))?;

        let decrypted = self.decrypt_with_password(&encrypted_data, master_password).await?;

        let backup: KeyBackup = serde_json::from_slice(&decrypted)
            .map_err(|e| WalletError::DeserializationError(e.to_string()))?;

        // Validate backup version
        if backup.version != 1 {
            return Err(WalletError::DecryptionError(
                format!("Unsupported backup version: {}", backup.version)
            ));
        }

        // Restore keys
        let mut key_store = self.key_store.write().await;
        *key_store = backup.keys;

        Ok(())
    }

    /// Verify encryption integrity
    pub async fn verify_integrity(&self) -> WalletResult<bool> {
        let test_data = b"integrity_test_data";
        let encrypted = self.encrypt_data_internal(test_data).await?;
        let decrypted = self.decrypt_data_internal(&encrypted).await?;

        Ok(test_data == decrypted.as_slice())
    }

    /// Get encryption metadata
    pub async fn get_metadata(&self) -> EncryptionMetadata {
        let key_store = self.key_store.read().await;
        let current_key_id = self.current_key_id.read().await;

        EncryptionMetadata {
            current_key_id: current_key_id.clone(),
            total_keys: key_store.len(),
            encryption_algorithm: "AES-256-GCM".to_string(),
            key_derivation: "Argon2".to_string(),
            version: 1,
        }
    }
}

/// Key statistics
#[derive(Debug, Clone)]
pub struct KeyStats {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub usage_count: u64,
}

/// Key backup structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyBackup {
    keys: Vec<EncryptionKey>,
    created_at: chrono::DateTime<chrono::Utc>,
    version: u8,
}

/// Encryption metadata
#[derive(Debug, Clone)]
pub struct EncryptionMetadata {
    pub current_key_id: String,
    pub total_keys: usize,
    pub encryption_algorithm: String,
    pub key_derivation: String,
    pub version: u8,
}

/// Secure string for handling sensitive data
#[derive(Debug, Clone, ZeroizeOnDrop)]
pub struct SecureString {
    #[zeroize(skip)]
    inner: String,
}

impl SecureString {
    pub fn new(s: String) -> Self {
        Self { inner: s }
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }

    pub fn into_string(self) -> String {
        self.inner
    }
}

impl From<String> for SecureString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SecureString {
    fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::SecurityConfig;

    fn create_test_config() -> SecurityConfig {
        SecurityConfig {
            encryption_key: [1u8; 32],
            enable_key_rotation: true,
            max_decrypt_attempts: 3,
            security_level: crate::security::SecurityLevel::Standard,
        }
    }

    #[tokio::test]
    async fn test_encryption_decryption() {
        let config = create_test_config();
        let encryption = WalletEncryption::new(config).unwrap();

        let test_data = b"test_private_key_data";
        let encrypted = encryption.encrypt_data(test_data).await.unwrap();
        let decrypted = encryption.decrypt_data(&encrypted).await.unwrap();

        assert_eq!(test_data, decrypted.as_slice());
    }

    #[tokio::test]
    async fn test_private_key_encryption() {
        let config = create_test_config();
        let encryption = WalletEncryption::new(config).unwrap();

        let private_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let encrypted = encryption.encrypt_private_key(private_key).await.unwrap();
        let decrypted = encryption.decrypt_private_key(&encrypted).await.unwrap();

        assert_eq!(private_key, decrypted);
    }

    #[tokio::test]
    async fn test_password_based_encryption() {
        let config = create_test_config();
        let encryption = WalletEncryption::new(config).unwrap();

        let test_data = b"sensitive_data";
        let password = "strong_password_123";

        let encrypted = encryption.encrypt_with_password(test_data, password).await.unwrap();
        let decrypted = encryption.decrypt_with_password(&encrypted, password).await.unwrap();

        assert_eq!(test_data, decrypted.as_slice());
    }

    #[tokio::test]
    async fn test_key_rotation() {
        let config = create_test_config();
        let encryption = WalletEncryption::new(config).unwrap();

        let original_key_id = encryption.get_current_key_id().await;
        let new_key_id = encryption.rotate_key().await.unwrap();

        assert_ne!(original_key_id, new_key_id);
        assert_eq!(new_key_id, encryption.get_current_key_id().await);
    }

    #[tokio::test]
    async fn test_integrity_verification() {
        let config = create_test_config();
        let encryption = WalletEncryption::new(config).unwrap();

        let integrity_check = encryption.verify_integrity().await.unwrap();
        assert!(integrity_check);
    }

    #[tokio::test]
    async fn test_key_backup_restore() {
        let config = create_test_config();
        let encryption = WalletEncryption::new(config).unwrap();

        let master_password = "master_password_123";
        let backup = encryption.backup_keys(master_password).await.unwrap();

        // Rotate key to change state
        encryption.rotate_key().await.unwrap();

        // Restore from backup
        encryption.restore_keys(&backup, master_password).await.unwrap();

        // Verify restore worked
        assert!(encryption.verify_integrity().await.unwrap());
    }
}