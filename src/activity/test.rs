// src/activity/tests.rs
#[cfg(test)]
mod tests {
    use crate::ActivitySimulator;
    use crate::error::WalletError;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_onchain_activity() {
        let simulator = ActivitySimulator::new(
            "http://localhost:8545".to_string(), // Replace with Reth or Infura RPC
            "0x...private_key...".to_string(),   // Replace with test key
            None,
            None,
        ).unwrap();

        let wallet_id = Uuid::new_v4();
        let result = simulator.simulate_onchain_activity(wallet_id, 1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_offchain_activity() {
        let simulator = ActivitySimulator::new(
            "http://localhost:8545".to_string(),
            "0x...private_key...".to_string(),
            Some("discord_test_key".to_string()),
            Some("twitter_test_key".to_string()),
        ).unwrap();

        let wallet_id = Uuid::new_v4();
        let result = simulator.simulate_offchain_activity(wallet_id).await;
        assert!(result.is_ok());
    }
}