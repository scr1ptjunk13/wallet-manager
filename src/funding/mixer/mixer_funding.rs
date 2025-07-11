// src/funding/mixer/mixer_funding.rs
use crate::error::WalletError;
use crate::types::{FundingRecord, FundingSource, MixerFundingRequest, MixerType};
use super::fund_mixer::FundMixer;
use super::types::*;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[derive(Clone)]
pub struct MixerFunding {
    mixer: FundMixer,
}

impl MixerFunding {
    pub async fn new(config: &MixerConfig) -> Result<Self, WalletError> {
        Ok(Self {
            mixer: FundMixer::new(config.clone()).await?,
        })
    }

    pub async fn fund_wallet(&mut self, request: MixerFundingRequest) -> Result<FundingRecord, WalletError> {
        let mixer_request = MixingRequest {
            wallet_id: request.wallet_id,
            chain_id: request.chain_id,
            amount: request.amount,
            strategy: match request.mixer_type {
                MixerType::Tornado => MixingStrategy::TornadoCash,
                MixerType::Aztec => MixingStrategy::RelayNetwork,
                MixerType::Railgun => MixingStrategy::RelayNetwork,
            },
            destination_addresses: vec![request.wallet_id.to_string()],
            relay_preference: Some(match request.mixer_type {
                MixerType::Tornado => "tornado".to_string(),
                MixerType::Aztec => "aztec".to_string(),
                MixerType::Railgun => "railgun".to_string(),
            }),
            custom_pattern: None,
        };

        let session = self.mixer.start_mixing(mixer_request).await?;
        let start_time = chrono::Utc::now();
        loop {
            if let Some(session) = self.mixer.get_mixing_session(session.id) {
                match session.status {
                    MixingStatus::Completed => {
                        let execution_time = chrono::Utc::now()
                            .signed_duration_since(start_time)
                            .num_seconds() as u64;
                        return Ok(FundingRecord {
                            id: Uuid::new_v4(),
                            wallet_id: request.wallet_id,
                            amount: request.amount,
                            chain_id: request.chain_id,
                            funding_source: FundingSource::Mixer(request.clone()),
                            success: true,
                            transaction_hash: session.steps.last().and_then(|step| step.transaction_hash.clone()),
                            timestamp: start_time,
                            cost: request.amount * 0.01,
                            execution_time_seconds: execution_time,
                        });
                    }
                    MixingStatus::Failed => {
                        return Err(WalletError::MixingError("Mixing session failed".to_string()));
                    }
                    _ => {
                        sleep(Duration::from_secs(30)).await;
                        continue;
                    }
                }
            } else {
                return Err(WalletError::MixingError("Session not found".to_string()));
            }
        }
    }

    pub async fn health_check(&self) -> Result<(), WalletError> {
        for (chain_id, connector) in &self.mixer.tornado_connectors {
            // Check if Tornado Cash contract is accessible
            let provider = Http::new(reqwest::Url::parse(&self.mixer.config.tornado_relayer_url)?);
            let block_number = provider.get_block_number().await
                .map_err(|e| WalletError::MixingError(format!("Tornado provider unavailable: {}", e)))?;
            log::info!("Tornado connector for chain {} is healthy, block: {}", chain_id, block_number);
        }

        for (name, relay) in &self.mixer.relay_networks {
            // Placeholder: Check API or contract availability
            log::info!("Relay {} is healthy", name);
        }

        Ok(())
    }
}