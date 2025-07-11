// src/funding/mixer/mixer_funding.rs
use crate::activity::ActivitySimulator;
use crate::error::WalletError;
use crate::types::{FundingRecord, FundingSource, MixerFundingRequest, MixerType};
use super::fund_mixer::FundMixer;
use super::types::*;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[derive(Clone)]
pub struct MixerFunding {
    mixer: FundMixer,
    activity_simulator: Option<ActivitySimulator>,
}

impl MixerFunding {
    pub async fn new(config: &MixerConfig) -> Result<Self, WalletError> {
        let activity_simulator = if config.tornado_enabled || config.aztec_enabled || config.railgun_enabled || config.noir_enabled || config.penumbra_enabled {
            // Initialize ActivitySimulator with same RPC and keys as mixer
            Some(ActivitySimulator::new(
                config.tornado_relayer_url.clone(), // Use same RPC or configure separately
                config.tornado_private_key.clone(), // Use same key or configure separately
                Some("discord_api_key".to_string()), // Replace with actual key
                Some("twitter_api_key".to_string()), // Replace with actual key
            )?)
        } else {
            None
        };

        Ok(Self {
            mixer: FundMixer::new(config.clone()).await?,
            activity_simulator,
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
                MixerType::Noir => MixingStrategy::Noir,
                MixerType::Penumbra => MixingStrategy::Penumbra,
            },
            destination_addresses: vec![request.wallet_id.to_string()],
            relay_preference: Some(match request.mixer_type {
                MixerType::Tornado => "tornado".to_string(),
                MixerType::Aztec => "aztec".to_string(),
                MixerType::Railgun => "railgun".to_string(),
                MixerType::Noir => "noir".to_string(),
                MixerType::Penumbra => "penumbra".to_string(),
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

                        // Trigger activity simulation if enabled
                        if request.post_funding_activity {
                            if let Some(simulator) = &self.activity_simulator {
                                simulator.simulate_activity(request.wallet_id, request.chain_id).await?;
                            }
                        }

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
        // Existing health check logic...
        if let Some(simulator) = &self.activity_simulator {
            // Check simulator health (e.g., API connectivity)
            let response = simulator.client.get("https://discord.com/api/v10/users/@me")
                .bearer_auth(simulator.discord_api_key.as_ref().unwrap_or(&"".to_string()))
                .send().await;
            if let Err(e) = response {
                log::warn!("Discord API health check failed: {}", e);
            }
        }
        Ok(())
    }
}