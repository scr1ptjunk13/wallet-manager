// src/funding/mixer/tests.rs
#[cfg(test)]
mod tests {
    use super::super::{fund_mixer::FundMixer, mixer_funding::MixerFunding, types::*};
    use crate::types::{MixerConfig, MixerFundingRequest, MixerType};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_fund_mixer_creation() {
        let config = MixerConfig::default();
        let mixer = FundMixer::new(config).await;
        assert!(mixer.is_ok());
    }

    #[tokio::test]
    async fn test_mixer_funding() {
        let config = MixerConfig {
            tornado_enabled: true,
            tornado_relayer_url: "http://localhost".to_string(),
            tornado_private_key: "test_key".to_string(),
            ..Default::default()
        };
        let mut mixer_funding = MixerFunding::new(&config).await.unwrap();
        let request = MixerFundingRequest {
            wallet_id: Uuid::new_v4(),
            amount: 1.0,
            chain_id: 1,
            mixer_type: MixerType::Tornado,
            anonymity_set: 100,
            delay_hours: 1,
        };
        let result = mixer_funding.fund_wallet(request).await;
        assert!(result.is_ok());
    }
    #[tokio::test]
    async fn test_noir_mixing() {
        let config = MixerConfig {
            noir_enabled: true,
            noir_api_key: "test_key".to_string(),
            ..Default::default()
        };
        let mut mixer_funding = MixerFunding::new(&config).await.unwrap();
        let request = MixerFundingRequest {
            wallet_id: Uuid::new_v4(),
            amount: 1.0,
            chain_id: 1,
            mixer_type: MixerType::Noir, // Add Noir to MixerType in types.rs
            anonymity_set: 100,
            delay_hours: 1,
        };
        let result = mixer_funding.fund_wallet(request).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_mixing_stats() {
        let config = MixerConfig::default();
        let mixer = FundMixer::new(config).await.unwrap();
        let stats = mixer.get_mixing_stats();
        assert_eq!(stats.total_mixes, 0);
        assert_eq!(stats.total_volume, 0.0);
    }
}