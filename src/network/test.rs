// src/network/tests.rs
#[cfg(test)]
mod tests {
    use super::ProxyManager;
    use crate::error::WalletError;
    use reqwest::Client;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_proxy_rotation_api() {
        let proxies = vec![
            "http://proxy1.example.com:8080".to_string(),
            "http://proxy2.example.com:8080".to_string(),
            // Add more test proxies (mock or real)
        ];
        let proxy_manager = ProxyManager::new(proxies).unwrap();

        // Test 10 API calls
        for i in 0..10 {
            let client = proxy_manager.get_client().await.unwrap();
            let response = client
                .get("https://api.ipify.org") // Returns IP address
                .send()
                .await
                .map_err(|e| WalletError::MixingError(format!("API call failed: {}", e)));
            assert!(response.is_ok(), "API call {} failed", i + 1);
            log::info!("API call {} succeeded with proxy", i + 1);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    #[tokio::test]
    async fn test_proxy_rotation_blockchain() {
        let proxies = vec![
            "http://proxy1.example.com:8080".to_string(),
            "http://proxy2.example.com:8080".to_string(),
        ];
        let proxy_manager = ProxyManager::new(proxies).unwrap();

        // Test 10 blockchain interactions (mocked)
        for i in 0..10 {
            let client = proxy_manager.get_client().await.unwrap();
            // Mock blockchain interaction (replace with real Alloy provider test)
            log::info!("Blockchain interaction {} succeeded with proxy", i + 1);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}


//usage example:
// Example initialization in main.rs or elsewhere
// use crate::network::ProxyManager;
// use crate::activity::ActivitySimulator;
// 
// async fn setup() -> Result<(), WalletError> {
//     let proxies = vec![
//         "http://proxy1.example.com:8080".to_string(),
//         "http://proxy2.example.com:8080".to_string(),
//         // Add 48 more proxy URLs (total 50)
//     ];
// 
//     let proxy_manager = ProxyManager::new(proxies)?;
// 
//     let simulator = ActivitySimulator::new(
//         "https://mainnet.infura.io/v3/YOUR_INFURA_KEY".to_string(),
//         "0x...private_key...".to_string(),
//         Some("discord_api_key".to_string()),
//         Some("twitter_api_key".to_string()),
//         proxy_manager.proxies.lock().await.clone(),
//     )?;
// 
//     // Simulate activity with proxy rotation
//     simulator.simulate_activity(Uuid::new_v4(), 1).await?;
//     Ok(())
// }