// examples/basic_usage.rs
use wallet_manager::{WalletManager, WalletConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize configuration
    let config = WalletConfig {
        master_seed: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string(),
        derivation_base: "m/44'/60'/0'/0".to_string(),
        encryption_key: [42u8; 32], // In real use, generate this securely
        supported_chains: vec![1, 137, 42161], // Ethereum, Polygon, Arbitrum
    };

    // Create wallet manager
    let manager = WalletManager::new(config).await?;

    // Generate 10 wallets
    println!("🔧 Generating 10 wallets...");
    let wallet_ids = manager.generate_wallets(10).await?;

    println!("✅ Generated {} wallets", wallet_ids.len());

    // Display wallet info
    for wallet_id in &wallet_ids {
        let wallet = manager.get_wallet(*wallet_id).await?.unwrap();
        println!("💳 Wallet {}: {}", wallet_id, wallet.address);
    }

    // Health check
    println!("🏥 Running health check...");
    manager.health_check().await?;
    println!("✅ Health check passed");

    // Get total count
    println!("📊 Total wallets: {}", manager.wallet_count().await);

    Ok(())
}