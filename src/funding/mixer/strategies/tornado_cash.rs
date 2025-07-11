// Purpose: Logic for the Tornado Cash mixing strategy.
// Contents :execute_tornado_mixing function.


// src/funding/mixer/strategies/tornado_cash.rs
use crate::error::WalletError;
use crate::funding::mixer::types::*;
use crate::funding::mixer::fund_mixer::FundMixer;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

pub async fn execute_tornado_mixing(
    mixer: &mut FundMixer,
    session_id: Uuid,
    request: MixingRequest,
) -> Result<(), WalletError> {
    let tornado = mixer.tornado_connectors.get(&request.chain_id)
        .ok_or_else(|| WalletError::MixingError("Tornado Cash not supported on this chain".to_string()))?;

    mixer.update_mixing_status(session_id, MixingStatus::InProgress).await;

    mixer.add_mixing_step(session_id, MixingStep {
        step_type: MixingStepType::TornadoDeposit,
        status: StepStatus::InProgress,
        transaction_hash: None,
        amount: request.amount,
        timestamp: chrono::Utc::now(),
    }).await;

    let deposit_result = tornado.deposit(request.amount, request.wallet_id).await
        .map_err(|e| WalletError::MixingError(format!("Tornado deposit failed: {}", e)))?;

    mixer.update_step_status(session_id, 0, StepStatus::Completed, Some(deposit_result.tx_hash.clone())).await;

    let wait_time = mixer.calculate_optimal_wait_time(request.amount, request.chain_id).await;
    sleep(Duration::from_secs(wait_time)).await;

    mixer.add_mixing_step(session_id, MixingStep {
        step_type: MixingStepType::TornadoWithdraw,
        status: StepStatus::InProgress,
        transaction_hash: None,
        amount: request.amount,
        timestamp: chrono::Utc::now(),
    }).await;

    let withdraw_result = tornado.withdraw(
        request.amount,
        request.destination_addresses[0].clone(),
        deposit_result.commitment,
        deposit_result.nullifier,
    ).await
        .map_err(|e| WalletError::MixingError(format!("Tornado withdraw failed: {}", e)))?;

    mixer.update_step_status(session_id, 1, StepStatus::Completed, Some(withdraw_result.tx_hash)).await;

    mixer.complete_mixing_session(session_id, withdraw_result.final_amount).await;

    Ok(())
}