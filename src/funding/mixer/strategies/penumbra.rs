// src/funding/mixer/strategies/penumbra.rs
use crate::error::WalletError;
use crate::funding::mixer::types::*;
use crate::funding::mixer::fund_mixer::FundMixer;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

pub async fn execute_penumbra_mixing(
    mixer: &mut FundMixer,
    session_id: Uuid,
    request: MixingRequest,
) -> Result<(), WalletError> {
    let relay = mixer.relay_networks.get("penumbra")
        .ok_or_else(|| WalletError::MixingError("Penumbra relay not available".to_string()))?;

    mixer.update_mixing_status(session_id, MixingStatus::InProgress).await;

    mixer.add_mixing_step(session_id, MixingStep {
        step_type: MixingStepType::Shield,
        status: StepStatus::InProgress,
        transaction_hash: None,
        amount: request.amount,
        timestamp: chrono::Utc::now(),
    }).await;

    let shield_result = relay.shield_funds(request.wallet_id, request.amount).await
        .map_err(|e| WalletError::MixingError(format!("Penumbra shield failed: {}", e)))?;

    mixer.update_step_status(session_id, 0, StepStatus::Completed, Some(shield_result.tx_hash.clone())).await;

    let wait_time = mixer.calculate_optimal_wait_time(request.amount, request.chain_id).await;
    sleep(Duration::from_secs(wait_time)).await;

    mixer.add_mixing_step(session_id, MixingStep {
        step_type: MixingStepType::PrivateTransfer,
        status: StepStatus::InProgress,
        transaction_hash: None,
        amount: request.amount,
        timestamp: chrono::Utc::now(),
    }).await;

    let transfer_result = relay.private_transfer(
        shield_result.commitment,
        request.destination_addresses[0].clone(),
        request.amount,
    ).await
        .map_err(|e| WalletError::MixingError(format!("Penumbra private transfer failed: {}", e)))?;

    mixer.update_step_status(session_id, 1, StepStatus::Completed, Some(transfer_result.tx_hash)).await;

    mixer.complete_mixing_session(session_id, request.amount).await;
    Ok(())
}