use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Wallet not found : {0}")]
    WalletNotFound(Uuid),
    
    #[error("Encryption failed : {0}")]
    EncryptionFailed(String),
}