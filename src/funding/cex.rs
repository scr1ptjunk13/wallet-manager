// src/funding/cex.rs
use crate::types::*;
use crate::error::WalletError;
use std::collections::HashMap;
use uuid::Uuid;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// CEX funding implementation for automated withdrawals
pub struct CexFunding {
    config: CexConfig,
    exchanges: HashMap<String, Box<dyn ExchangeConnector>>,
    withdrawal_history: Vec<WithdrawalRecord>,
}

impl CexFunding {
    /// Create new CEX funding manager
    pub async fn new(config: &CexConfig) -> Result<Self, WalletError> {
        let mut exchanges = HashMap::new();

        // Initialize exchange connectors
        if config.binance_enabled {
            exchanges.insert("binance".to_string(), Box::new(BinanceConnector::new(
                config.binance_api_key.clone(),
                config.binance_secret.clone(),
            )?));
        }

        if config.coinbase_enabled {
            exchanges.insert("coinbase".to_string(), Box::new(CoinbaseConnector::new(
                config.coinbase_api_key.clone(),
                config.coinbase_secret.clone(),
            )?));
        }

        if config.okx_enabled {
            exchanges.insert("okx".to_string(), Box::new(OkxConnector::new(
                config.okx_api_key.clone(),
                config.okx_secret.clone(),
                config.okx_passphrase.clone(),
            )?));
        }

        Ok(Self {
            config: config.clone(),
            exchanges,
            withdrawal_history: Vec::new(),
        })
    }

    /// Fund wallet through CEX withdrawal
    pub async fn fund_wallet(&mut self, request: CexFundingRequest) -> Result<FundingRecord, WalletError> {
        let exchange = self.exchanges.get(&request.exchange)
            .ok_or_else(|| WalletError::FundingError(format!("Exchange {} not configured", request.exchange)))?;

        let start_time = std::time::Instant::now();

        // Get wallet address (this would come from your wallet manager)
        let wallet_address = self.get_wallet_address(request.wallet_id).await?;

        // Prepare withdrawal request
        let withdrawal_request = WithdrawalRequest {
            currency: self.get_currency_for_chain(request.chain_id)?,
            amount: request.amount,
            address: wallet_address,
            network: self.get_network_name(request.chain_id)?,
            tag: None,
        };

        // Add delay if specified
        if request.delay_seconds > 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(request.delay_seconds)).await;
        }

        // Execute withdrawal
        let withdrawal_result = match request.withdraw_method {
            WithdrawMethod::Direct => {
                exchange.withdraw_direct(withdrawal_request).await
            }
            WithdrawMethod::Staged => {
                exchange.withdraw_staged(withdrawal_request).await
            }
            WithdrawMethod::Randomized => {
                exchange.withdraw_randomized(withdrawal_request).await
            }
        };

        let execution_time = start_time.elapsed().as_secs();

        let (success, transaction_hash, cost) = match withdrawal_result {
            Ok(result) => (true, Some(result.transaction_hash), result.fee),
            Err(e) => {
                return Err(WalletError::FundingError(format!("Withdrawal failed: {}", e)));
            }
        };

        // Create funding record
        let funding_record = FundingRecord {
            id: Uuid::new_v4(),
            wallet_id: request.wallet_id,
            amount: request.amount,
            chain_id: request.chain_id,
            funding_source: FundingSource::Cex(request.clone()),
            success,
            transaction_hash,
            timestamp: chrono::Utc::now(),
            cost,
            execution_time_seconds: execution_time,
        };

        // Store in history
        self.withdrawal_history.push(WithdrawalRecord {
            id: funding_record.id,
            exchange: request.exchange.clone(),
            wallet_id: request.wallet_id,
            amount: request.amount,
            chain_id: request.chain_id,
            status: if success { WithdrawalStatus::Completed } else { WithdrawalStatus::Failed },
            transaction_hash: transaction_hash.clone(),
            timestamp: funding_record.timestamp,
            fee: cost,
        });

        Ok(funding_record)
    }

    /// Get available balance on exchange
    pub async fn get_exchange_balance(&self, exchange: &str, currency: &str) -> Result<f64, WalletError> {
        let connector = self.exchanges.get(exchange)
            .ok_or_else(|| WalletError::FundingError(format!("Exchange {} not configured", exchange)))?;

        connector.get_balance(currency).await
    }

    /// Get withdrawal limits for exchange
    pub async fn get_withdrawal_limits(&self, exchange: &str, currency: &str) -> Result<WithdrawalLimits, WalletError> {
        let connector = self.exchanges.get(exchange)
            .ok_or_else(|| WalletError::FundingError(format!("Exchange {} not configured", exchange)))?;

        connector.get_withdrawal_limits(currency).await
    }

    /// Optimize withdrawal strategy
    pub async fn optimize_withdrawal(&self, amount: f64, chain_id: u64) -> Result<CexWithdrawalStrategy, WalletError> {
        let currency = self.get_currency_for_chain(chain_id)?;
        let mut strategies = Vec::new();

        // Check each exchange
        for (exchange_name, connector) in &self.exchanges {
            if let Ok(balance) = connector.get_balance(&currency).await {
                if balance >= amount {
                    if let Ok(limits) = connector.get_withdrawal_limits(&currency).await {
                        if amount >= limits.min_amount && amount <= limits.max_amount {
                            strategies.push(CexWithdrawalOption {
                                exchange: exchange_name.clone(),
                                available_balance: balance,
                                withdrawal_fee: limits.fee,
                                estimated_time_minutes: limits.processing_time_minutes,
                                daily_limit_remaining: limits.daily_limit - limits.daily_used,
                            });
                        }
                    }
                }
            }
        }

        // Sort by best option (lowest fee, highest balance, fastest time)
        strategies.sort_by(|a, b| {
            let score_a = a.withdrawal_fee + (a.estimated_time_minutes as f64 * 0.001);
            let score_b = b.withdrawal_fee + (b.estimated_time_minutes as f64 * 0.001);
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        if strategies.is_empty() {
            return Err(WalletError::FundingError("No suitable exchange found for withdrawal".to_string()));
        }

        Ok(CexWithdrawalStrategy {
            recommended_exchange: strategies[0].exchange.clone(),
            options: strategies,
            split_recommended: amount > 1.0, // Split large amounts
        })
    }

    /// Batch withdraw to multiple wallets
    pub async fn batch_withdraw(&mut self, requests: Vec<CexFundingRequest>) -> Result<Vec<FundingRecord>, WalletError> {
        let mut results = Vec::new();

        // Group by exchange for efficiency
        let mut grouped_requests: HashMap<String, Vec<CexFundingRequest>> = HashMap::new();
        for request in requests {
            grouped_requests.entry(request.exchange.clone()).or_insert_with(Vec::new).push(request);
        }

        for (exchange, exchange_requests) in grouped_requests {
            // Add delay between batches to avoid rate limiting
            if !results.is_empty() {
                tokio::time::sleep(tokio::time::Duration::from_secs(self.config.batch_delay_seconds)).await;
            }

            for request in exchange_requests {
                match self.fund_wallet(request).await {
                    Ok(record) => results.push(record),
                    Err(e) => {
                        // Log error but continue with other requests
                        eprintln!("Withdrawal failed for wallet {}: {}", request.wallet_id, e);
                    }
                }

                // Rate limiting between individual withdrawals
                tokio::time::sleep(tokio::time::Duration::from_secs(self.config.withdrawal_delay_seconds)).await;
            }
        }

        Ok(results)
    }

    /// Get withdrawal history
    pub fn get_withdrawal_history(&self) -> &Vec<WithdrawalRecord> {
        &self.withdrawal_history
    }

    /// Get withdrawal statistics
    pub fn get_withdrawal_stats(&self) -> WithdrawalStats {
        let total_withdrawals = self.withdrawal_history.len();
        let successful_withdrawals = self.withdrawal_history.iter()
            .filter(|r| r.status == WithdrawalStatus::Completed)
            .count();

        let total_amount = self.withdrawal_history.iter()
            .filter(|r| r.status == WithdrawalStatus::Completed)
            .map(|r| r.amount)
            .sum::<f64>();

        let total_fees = self.withdrawal_history.iter()
            .filter(|r| r.status == WithdrawalStatus::Completed)
            .map(|r| r.fee)
            .sum::<f64>();

        WithdrawalStats {
            total_withdrawals,
            successful_withdrawals,
            success_rate: if total_withdrawals > 0 {
                (successful_withdrawals as f64 / total_withdrawals as f64) * 100.0
            } else { 0.0 },
            total_amount,
            total_fees,
            average_amount: if successful_withdrawals > 0 {
                total_amount / successful_withdrawals as f64
            } else { 0.0 },
        }
    }

    /// Health check
    pub async fn health_check(&self) -> Result<(), WalletError> {
        for (exchange_name, connector) in &self.exchanges {
            connector.health_check().await
                .map_err(|e| WalletError::HealthCheck(format!("Exchange {} health check failed: {}", exchange_name, e)))?;
        }
        Ok(())
    }

    // Helper methods
    async fn get_wallet_address(&self, wallet_id: Uuid) -> Result<String, WalletError> {
        // This would integrate with your wallet manager
        // For now, return a mock address
        Ok(format!("0x{:x}", wallet_id.as_u128()))
    }

    fn get_currency_for_chain(&self, chain_id: u64) -> Result<String, WalletError> {
        let currency = match chain_id {
            1 => "ETH",      // Ethereum
            137 => "MATIC",  // Polygon
            42161 => "ETH",  // Arbitrum
            10 => "ETH",     // Optimism
            56 => "BNB",     // BSC
            43114 => "AVAX", // Avalanche
            _ => return Err(WalletError::FundingError(format!("Unsupported chain ID: {}", chain_id))),
        };
        Ok(currency.to_string())
    }

    fn get_network_name(&self, chain_id: u64) -> Result<String, WalletError> {
        let network = match chain_id {
            1 => "ERC20",
            137 => "MATIC",
            42161 => "ARBITRUM",
            10 => "OPTIMISM",
            56 => "BSC",
            43114 => "AVAX",
            _ => return Err(WalletError::FundingError(format!("Unsupported chain ID: {}", chain_id))),
        };
        Ok(network.to_string())
    }
}

/// Exchange connector trait
#[async_trait]
pub trait ExchangeConnector: Send + Sync {
    async fn withdraw_direct(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError>;
    async fn withdraw_staged(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError>;
    async fn withdraw_randomized(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError>;
    async fn get_balance(&self, currency: &str) -> Result<f64, WalletError>;
    async fn get_withdrawal_limits(&self, currency: &str) -> Result<WithdrawalLimits, WalletError>;
    async fn health_check(&self) -> Result<(), WalletError>;
}

/// Binance connector implementation
pub struct BinanceConnector {
    api_key: String,
    secret: String,
    client: reqwest::Client,
}

impl BinanceConnector {
    pub fn new(api_key: String, secret: String) -> Result<Self, WalletError> {
        Ok(Self {
            api_key,
            secret,
            client: reqwest::Client::new(),
        })
    }

    fn generate_signature(&self, query_string: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }
}

#[async_trait]
impl ExchangeConnector for BinanceConnector {
    async fn withdraw_direct(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let query_string = format!(
            "coin={}&address={}&amount={}&network={}&timestamp={}",
            request.currency, request.address, request.amount, request.network, timestamp
        );

        let signature = self.generate_signature(&query_string);
        let url = format!("https://api.binance.com/sapi/v1/capital/withdraw/apply?{}&signature={}", query_string, signature);

        let response = self.client
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| WalletError::FundingError(format!("Binance API error: {}", e)))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| WalletError::FundingError(format!("Failed to parse Binance response: {}", e)))?;

        if let Some(id) = result.get("id") {
            Ok(WithdrawalResult {
                transaction_hash: id.to_string(),
                fee: 0.001, // Default fee, should be fetched from response
            })
        } else {
            Err(WalletError::FundingError("Binance withdrawal failed".to_string()))
        }
    }

    async fn withdraw_staged(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError> {
        // Add random delay for staging
        let delay = fastrand::u64(30..300); // 30-300 seconds
        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
        self.withdraw_direct(request).await
    }

    async fn withdraw_randomized(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError> {
        // Add random delay and slightly randomize amount
        let delay = fastrand::u64(60..600); // 1-10 minutes
        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;

        let mut randomized_request = request;
        // Randomize amount by ±1%
        let variation = fastrand::f64() * 0.02 - 0.01; // -1% to +1%
        randomized_request.amount *= 1.0 + variation;

        self.withdraw_direct(randomized_request).await
    }

    async fn get_balance(&self, currency: &str) -> Result<f64, WalletError> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let query_string = format!("timestamp={}", timestamp);
        let signature = self.generate_signature(&query_string);
        let url = format!("https://api.binance.com/sapi/v1/capital/config/getall?{}&signature={}", query_string, signature);

        let response = self.client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| WalletError::FundingError(format!("Binance API error: {}", e)))?;

        let balances: serde_json::Value = response.json().await
            .map_err(|e| WalletError::FundingError(format!("Failed to parse Binance response: {}", e)))?;

        // Parse balance from response
        Ok(1.0) // Mock balance
    }

    async fn get_withdrawal_limits(&self, currency: &str) -> Result<WithdrawalLimits, WalletError> {
        Ok(WithdrawalLimits {
            min_amount: 0.001,
            max_amount: 1000.0,
            daily_limit: 100.0,
            daily_used: 0.0,
            fee: 0.001,
            processing_time_minutes: 5,
        })
    }

    async fn health_check(&self) -> Result<(), WalletError> {
        let url = "https://api.binance.com/api/v3/ping";
        let response = self.client.get(url).send().await
            .map_err(|e| WalletError::HealthCheck(format!("Binance ping failed: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(WalletError::HealthCheck("Binance API not responding".to_string()))
        }
    }
}

/// Coinbase connector implementation
pub struct CoinbaseConnector {
    api_key: String,
    secret: String,
    client: reqwest::Client,
}

impl CoinbaseConnector {
    pub fn new(api_key: String, secret: String) -> Result<Self, WalletError> {
        Ok(Self {
            api_key,
            secret,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl ExchangeConnector for CoinbaseConnector {
    async fn withdraw_direct(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError> {
        // Coinbase implementation
        Ok(WithdrawalResult {
            transaction_hash: "0x1234567890abcdef".to_string(),
            fee: 0.002,
        })
    }

    async fn withdraw_staged(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError> {
        let delay = fastrand::u64(30..300);
        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
        self.withdraw_direct(request).await
    }

    async fn withdraw_randomized(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError> {
        let delay = fastrand::u64(60..600);
        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
        self.withdraw_direct(request).await
    }

    async fn get_balance(&self, currency: &str) -> Result<f64, WalletError> {
        Ok(1.0) // Mock implementation
    }

    async fn get_withdrawal_limits(&self, currency: &str) -> Result<WithdrawalLimits, WalletError> {
        Ok(WithdrawalLimits {
            min_amount: 0.001,
            max_amount: 1000.0,
            daily_limit: 50.0,
            daily_used: 0.0,
            fee: 0.002,
            processing_time_minutes: 10,
        })
    }

    async fn health_check(&self) -> Result<(), WalletError> {
        Ok(())
    }
}

/// OKX connector implementation
pub struct OkxConnector {
    api_key: String,
    secret: String,
    passphrase: String,
    client: reqwest::Client,
}

impl OkxConnector {
    pub fn new(api_key: String, secret: String, passphrase: String) -> Result<Self, WalletError> {
        Ok(Self {
            api_key,
            secret,
            passphrase,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl ExchangeConnector for OkxConnector {
    async fn withdraw_direct(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError> {
        // OKX implementation
        Ok(WithdrawalResult {
            transaction_hash: "0xabcdef1234567890".to_string(),
            fee: 0.0015,
        })
    }

    async fn withdraw_staged(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError> {
        let delay = fastrand::u64(30..300);
        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
        self.withdraw_direct(request).await
    }

    async fn withdraw_randomized(&self, request: WithdrawalRequest) -> Result<WithdrawalResult, WalletError> {
        let delay = fastrand::u64(60..600);
        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
        self.withdraw_direct(request).await
    }

    async fn get_balance(&self, currency: &str) -> Result<f64, WalletError> {
        Ok(1.0) // Mock implementation
    }

    async fn get_withdrawal_limits(&self, currency: &str) -> Result<WithdrawalLimits, WalletError> {
        Ok(WithdrawalLimits {
            min_amount: 0.001,
            max_amount: 1000.0,
            daily_limit: 200.0,
            daily_used: 0.0,
            fee: 0.0015,
            processing_time_minutes: 3,
        })
    }

    async fn health_check(&self) -> Result<(), WalletError> {
        Ok(())
    }
}

/// CEX configuration
#[derive(Debug, Clone)]
pub struct CexConfig {
    pub binance_enabled: bool,
    pub binance_api_key: String,
    pub binance_secret: String,
    pub coinbase_enabled: bool,
    pub coinbase_api_key: String,
    pub coinbase_secret: String,
    pub okx_enabled: bool,
    pub okx_api_key: String,
    pub okx_secret: String,
    pub okx_passphrase: String,
    pub batch_delay_seconds: u64,
    pub withdrawal_delay_seconds: u64,
}

impl Default for CexConfig {
    fn default() -> Self {
        Self {
            binance_enabled: false,
            binance_api_key: String::new(),
            binance_secret: String::new(),
            coinbase_enabled: false,
            coinbase_api_key: String::new(),
            coinbase_secret: String::new(),
            okx_enabled: false,
            okx_api_key: String::new(),
            okx_secret: String::new(),
            okx_passphrase: String::new(),
            batch_delay_seconds: 10,
            withdrawal_delay_seconds: 5,
        }
    }
}

/// Additional types for CEX functionality
#[derive(Debug, Clone)]
pub struct WithdrawalRequest {
    pub currency: String,
    pub amount: f64,
    pub address: String,
    pub network: String,
    pub tag: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WithdrawalResult {
    pub transaction_hash: String,
    pub fee: f64,
}

#[derive(Debug, Clone)]
pub struct WithdrawalLimits {
    pub min_amount: f64,
    pub max_amount: f64,
    pub daily_limit: f64,
    pub daily_used: f64,
    pub fee: f64,
    pub processing_time_minutes: u32,
}

#[derive(Debug, Clone)]
pub struct WithdrawalRecord {
    pub id: Uuid,
    pub exchange: String,
    pub wallet_id: Uuid,
    pub amount: f64,
    pub chain_id: u64,
    pub status: WithdrawalStatus,
    pub transaction_hash: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub fee: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WithdrawalStatus {
    Pending,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct WithdrawalStats {
    pub total_withdrawals: usize,
    pub successful_withdrawals: usize,
    pub success_rate: f64,
    pub total_amount: f64,
    pub total_fees: f64,
    pub average_amount: f64,
}

#[derive(Debug, Clone)]
pub struct CexWithdrawalOption {
    pub exchange: String,
    pub available_balance: f64,
    pub withdrawal_fee: f64,
    pub estimated_time_minutes: u32,
    pub daily_limit_remaining: f64,
}

#[derive(Debug, Clone)]
pub struct CexWithdrawalStrategy {
    pub recommended_exchange: String,
    pub options: Vec<CexWithdrawalOption>,
    pub split_recommended: bool,
}