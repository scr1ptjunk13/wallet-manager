// funding/mixer/strategies/layered.rs, cross_chain.rs, relay_network.rs, custom.rs:
// Purpose: Logic for each mixing strategy (execute_layered_mixing, execute_cross_chain_obfuscation, etc.).
// Contents: Move each execute_* method from mixer.rs to its respective file, updating references to use FundMixer methods.


// src/funding/mixer/strategies/layered.rs
use crate::error::WalletError;
use crate::funding::mixer::types::*;
use crate::funding::mixer::fund_mixer::FundMixer;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use rand::Rng;

pub async fn execute_layered_mixing(
    mixer: &mut FundMixer,
    session_id: Uuid,
    request: MixingRequest,
) -> Result<(), WalletError> {
    mixer.update_mixing_status(session_id, MixingStatus::InProgress).await;
    let mut current_amount = request.amount;
    let destinations = &request.destination_addresses;

    let split_amounts = mixer.calculate_split_amounts(current_amount, destinations.len());

    for (i, &amount) in split_amounts.iter().enumerate() {
        mixer.add_mixing_step(session_id, MixingStep {
            step_type: MixingStepType::SplitTransfer,
            status: StepStatus::InProgress,
            transaction_hash: None,
            amount,
            timestamp: chrono::Utc::now(),
        }).await;

        let intermediate_wallet = mixer.get_intermediate_wallet(request.chain_id).await?;
        let tx_hash = mixer.execute_transfer(
            request.wallet_id,
            intermediate_wallet,
            amount,
            request.chain_id,
        ).await?;

        mixer.update_step_status(session_id, i, StepStatus::Completed, Some(tx_hash)).await;

        let delay = rand::thread_rng().gen_range(30..300);
        sleep(Duration::from_secs(delay)).await;
    }

    sleep(Duration::from_secs(mixer.config.min_mixing_delay)).await;

    for (i, destination) in destinations.iter().enumerate() {
        let amount = split_amounts[i];
        mixer.add_mixing_step(session_id, MixingStep {
            step_type: MixingStepType::ConsolidationTransfer,
            status: StepStatus::InProgress,
            transaction_hash: None,
            amount,
            timestamp: chrono::Utc::now(),
        }).await;

        let intermediate_wallet = mixer.get_intermediate_wallet(request.chain_id).await?;
        let tx_hash = mixer.execute_transfer(
            intermediate_wallet,
            destination.clone(),
            amount,
            request.chain_id,
        ).await?;

        let step_index = split_amounts.len() + i;
        mixer.update_step_status(session_id, step_index, StepStatus::Completed, Some(tx_hash)).await;

        let delay = rand::thread_rng().gen_range(60..600);
        sleep(Duration::from_secs(delay)).await;
    }

    mixer.complete_mixing_session(session_id, current_amount).await;
    Ok(())
}