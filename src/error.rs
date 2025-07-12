use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum WalletError {
    // Generation errors
    #[error("Failed to generate wallet: {0}")]
    GenerationError(String),

    #[error("Invalid derivation path: {0}")]
    InvalidDerivationPath(String),

    #[error("Seed phrase error: {0}")]
    SeedPhraseError(String),

    // Wallet management errors
    #[error("Wallet not found: {0}")]
    WalletNotFound(Uuid),

    #[error("Wallet already exists: {0}")]
    WalletAlreadyExists(Uuid),

    #[error("Invalid wallet ID: {0}")]
    InvalidWalletId(String),

    // Security errors
    #[error("Encryption failed: {0}")]
    EncryptionError(String),

    #[error("Decryption failed: {0}")]
    DecryptionError(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivationError(String),

    #[error("Invalid encryption key")]
    InvalidEncryptionKey,

    #[error("Security check failed: {0}")]
    SecurityCheckFailed(String),

    // Funding errors
    #[error("Funding failed: {0}")]
    FundingError(String),

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Invalid funding amount: {0}")]
    InvalidFundingAmount(String),

    #[error("Funding source unavailable: {0}")]
    FundingSourceUnavailable(String),

    #[error("Transaction failed: {0}")]
    TransactionError(String),

    // Balance errors
    #[error("Balance fetch failed: {0}")]
    BalanceFetchError(String),

    #[error("Balance update failed: {0}")]
    BalanceUpdateError(String),

    #[error("Invalid balance amount: {0}")]
    InvalidBalanceAmount(String),

    #[error("Unsupported chain: {0}")]
    UnsupportedChain(u64),

    // Network errors
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Connection timeout")]
    ConnectionTimeout,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    // Configuration errors
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Missing configuration key: {0}")]
    MissingConfigurationKey(String),

    #[error("Configuration load failed: {0}")]
    ConfigurationLoadError(String),

    // Database/Storage errors
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    // Validation errors
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Invalid private key")]
    InvalidPrivateKey,

    #[error("Invalid public key")]
    InvalidPublicKey,

    #[error("Validation failed: {0}")]
    ValidationError(String),

    // System errors
    #[error("System error: {0}")]
    SystemError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    // Airdrop specific errors
    #[error("Airdrop eligibility check failed: {0}")]
    AirdropEligibilityError(String),

    #[error("Airdrop claim failed: {0}")]
    AirdropClaimError(String),

    #[error("Airdrop not available: {0}")]
    AirdropNotAvailable(String),

    #[error("Airdrop already claimed: {0}")]
    AirdropAlreadyClaimed(String),

    #[error("Mixing error: {0}")]
    MixingError(String), // Added here

    // Generic errors
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Unknown error: {0}")]
    UnknownError(String),
}

impl WalletError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            WalletError::NetworkError(_)
            | WalletError::RpcError(_)
            | WalletError::ConnectionTimeout
            | WalletError::RateLimitExceeded
            | WalletError::TimeoutError(_)
            | WalletError::MixingError(_) => true, // Add MixingError as retryable
            _ => false,
        }
    }

    /// Check if error is critical (should stop all operations)
    pub fn is_critical(&self) -> bool {
        match self {
            WalletError::InvalidEncryptionKey
            | WalletError::SecurityCheckFailed(_)
            | WalletError::KeyDerivationError(_)
            | WalletError::InvalidConfiguration(_) => true,
            _ => false,
        }
    }

    /// Get error category for logging/metrics
    pub fn category(&self) -> &'static str {
        match self {
            WalletError::GenerationError(_)
            | WalletError::InvalidDerivationPath(_)
            | WalletError::SeedPhraseError(_) => "generation",

            WalletError::WalletNotFound(_)
            | WalletError::WalletAlreadyExists(_)
            | WalletError::InvalidWalletId(_) => "wallet_management",

            WalletError::EncryptionError(_)
            | WalletError::DecryptionError(_)
            | WalletError::KeyDerivationError(_)
            | WalletError::InvalidEncryptionKey
            | WalletError::SecurityCheckFailed(_) => "security",

            WalletError::FundingError(_)
            | WalletError::InsufficientFunds
            | WalletError::InvalidFundingAmount(_)
            | WalletError::FundingSourceUnavailable(_)
            | WalletError::TransactionError(_) => "funding",

            WalletError::BalanceFetchError(_)
            | WalletError::BalanceUpdateError(_)
            | WalletError::InvalidBalanceAmount(_)
            | WalletError::UnsupportedChain(_) => "balance",

            WalletError::NetworkError(_)
            | WalletError::RpcError(_)
            | WalletError::ConnectionTimeout
            | WalletError::RateLimitExceeded => "network",

            WalletError::InvalidConfiguration(_)
            | WalletError::MissingConfigurationKey(_)
            | WalletError::ConfigurationLoadError(_) => "configuration",

            WalletError::DatabaseError(_)
            | WalletError::StorageError(_)
            | WalletError::SerializationError(_)
            | WalletError::DeserializationError(_) => "storage",

            WalletError::InvalidAddress(_)
            | WalletError::InvalidPrivateKey
            | WalletError::InvalidPublicKey
            | WalletError::ValidationError(_) => "validation",

            WalletError::AirdropEligibilityError(_)
            | WalletError::AirdropClaimError(_)
            | WalletError::AirdropNotAvailable(_)
            | WalletError::AirdropAlreadyClaimed(_)
            | WalletError::MixingError(_) => "airdrop", // Add MixingError to airdrop category

            _ => "system",
        }
    }
}

// Result type alias for convenience
pub type WalletResult<T> = Result<T, WalletError>;