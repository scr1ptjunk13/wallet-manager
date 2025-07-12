// Airdrop provider simulation traits/modules

// config.rs: Defines an AirdropConfig struct to hold provider-specific data (e.g., contract address, chain ID, required actions). 
// Use serde to parse from a JSON/YAML file.Example:rust
// 
// use serde::{Deserialize, Serialize};
// use alloy_primitives::Address;
// 
// #[derive(Serialize, Deserialize, Debug)]
// pub struct AirdropConfig {
//     pub contract_address: Address,
//     pub chain_id: u64,
//     pub required_actions: Vec<String>,
//     pub social_keywords: Vec<String>,
// }
// 
