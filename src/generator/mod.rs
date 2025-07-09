// src/generator/mod.rs
pub mod derivation;

use crate::types::*;
use crate::error::WalletError;
use crate::security::SecurityManager;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use uuid::Uuid;

pub struct WalletGenerator {
    config: WalletConfig,
    security: SecurityManager,
    derivation_counter: Arc<AtomicU32>,
}

impl WalletGenerator {
    pub fn new(config: &WalletConfig) -> Result<Self, WalletError> {
        let security = SecurityManager::new(config.encryption_key)?;

        Ok(Self {
            config: config.clone(),
            security,
            derivation_counter: Arc::new(AtomicU32::new(0)),
        })
    }

    pub async fn generate_wallet(&self, alias: Option<String>) -> Result<Wallet, WalletError> {
        let wallet_id = Uuid::new_v4();
        let derivation_index = self.derivation_counter.fetch_add(1, Ordering::SeqCst);

        // Generate derivation path
        let derivation_path = format!("{}/{}", self.config.derivation_base, derivation_index);

        // Generate wallet from seed
        let (private_key, address) = self.derive_wallet(&derivation_path).await?;

        // Encrypt private key
        let encrypted_private_key = self.security.encrypt_private_key(&private_key).await?;

        // Create wallet
        let wallet = Wallet {
            id: wallet_id,
            address,
            encrypted_private_key,
            derivation_path,
            funding_source: FundingSource::Manual,
            created_at: chrono::Utc::now(),
            balances: self.create_initial_balances(),
            metadata: WalletMetadata {
                alias,
                proxy_used: None,
                risk_score: 0.0,
                active: true,
                last_activity: None,
            },
        };

        Ok(wallet)
    }

    async fn derive_wallet(&self, derivation_path: &str) -> Result<(String, String), WalletError> {
        use bip39::Mnemonic;
        use hdwallet::{DefaultKeyChain, ExtendedPrivKey, KeyChain};

        // Parse mnemonic
        let mnemonic = Mnemonic::parse(&self.config.master_seed)
            .map_err(|e| WalletError::KeyGeneration(e.to_string()))?;

        // Generate seed
        let seed = mnemonic.to_seed("");

        // Create master key
        let master_key = ExtendedPrivKey::with_seed(&seed)
            .map_err(|e| WalletError::KeyGeneration(e.to_string()))?;

        // Derive key at path
        let key_chain = DefaultKeyChain::new(master_key);
        let derived_key = key_chain.derive_private_key(derivation_path.parse().unwrap())
            .map_err(|e| WalletError::KeyGeneration(e.to_string()))?;

        // Get private key bytes
        let private_key_bytes = derived_key.private_key();
        let private_key_hex = hex::encode(private_key_bytes);

        // Generate address
        let address = self.private_key_to_address(&private_key_hex)?;

        Ok((private_key_hex, address))
    }

    fn private_key_to_address(&self, private_key_hex: &str) -> Result<String, WalletError> {
        use secp256k1::{PublicKey, SecretKey, Secp256k1};
        use tiny_keccak::{Hasher, Keccak};

        let secp = Secp256k1::new();

        // Parse private key
        let private_key_bytes = hex::decode(private_key_hex)
            .map_err(|e| WalletError::KeyGeneration(e.to_string()))?;

        let secret_key = SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| WalletError::KeyGeneration(e.to_string()))?;

        // Get public key
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize_uncompressed();

        // Generate address (last 20 bytes of keccak256 hash)
        let mut hasher = Keccak::v256();
        hasher.update(&public_key_bytes[1..]);
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);

        let address = format!("0x{}", hex::encode(&hash[12..]));
        Ok(address)
    }

    fn create_initial_balances(&self) -> std::collections::HashMap<String, Balance> {
        let mut balances = std::collections::HashMap::new();

        for &chain_id in &self.config.supported_chains {
            balances.insert(
                chain_id.to_string(),
                Balance {
                    chain_id,
                    native_balance: 0.0,
                    token_balances: std::collections::HashMap::new(),
                    last_updated: chrono::Utc::now(),
                },
            );
        }

        balances
    }

    pub async fn health_check(&self) -> Result<(), WalletError> {
        // Test wallet generation
        let test_wallet = self.generate_wallet(Some("health_check".to_string())).await?;

        // Verify wallet has valid address
        if !test_wallet.address.starts_with("0x") || test_wallet.address.len() != 42 {
            return Err(WalletError::HealthCheck("Invalid address format".to_string()));
        }

        Ok(())
    }
}