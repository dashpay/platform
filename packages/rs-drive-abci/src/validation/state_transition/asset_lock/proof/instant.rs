use crate::error::execution::ExecutionError;
use crate::error::execution::ExecutionError::CorruptedCodeExecution;
use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::{
    CoreRPCLike, CORE_RPC_INVALID_ADDRESS_OR_KEY, CORE_RPC_INVALID_PARAMETER, CORE_RPC_PARSE_ERROR,
};
use crate::validation::state_transition::asset_lock::transaction::validate_asset_lock_transaction_structure;
use dpp::consensus::basic::identity::{
    IdentityAssetLockProofLockedTransactionMismatchError,
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    InvalidInstantAssetLockProofSignatureError,
};
use dpp::dashcore::{InstantLock, OutPoint};
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::platform_value::Bytes36;
use dpp::prelude::ConsensusValidationResult;
use dpp::validation::SimpleConsensusValidationResult;
use drive::grovedb::TransactionArg;

/// Validate the structure of the instant asset lock proof
pub fn validate_structure(
    instant_asset_lock_proof: &InstantAssetLockProof,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut result = SimpleConsensusValidationResult::default();

    let transaction_id = instant_asset_lock_proof.transaction().txid();
    if instant_asset_lock_proof.instant_lock().txid != transaction_id {
        result.add_error(IdentityAssetLockProofLockedTransactionMismatchError::new(
            instant_asset_lock_proof.instant_lock().txid,
            transaction_id,
        ));

        return Ok(result);
    }

    let validate_transaction_result = validate_asset_lock_transaction_structure(
        instant_asset_lock_proof.transaction(),
        instant_asset_lock_proof.output_index(),
    )?;

    if !validate_transaction_result.is_valid() {
        result.merge(validate_transaction_result);
    }

    Ok(result)
}

/// Validate the state of the instant asset lock proof
pub fn validate_state<C: CoreRPCLike>(
    instant_asset_lock_proof: &InstantAssetLockProof,
    platform_ref: &PlatformRef<C>,
    transaction: TransactionArg,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut result = ConsensusValidationResult::default();

    // Make sure that asset lock isn't spent yet

    let Some(asset_lock_outpoint) = instant_asset_lock_proof.out_point() else {
        return Err(Error::Execution(CorruptedCodeExecution("asset lock outpoint must be present")));
    };

    let is_already_spent = platform_ref
        .drive
        .has_asset_lock_outpoint(&Bytes36(asset_lock_outpoint), transaction)?;

    if is_already_spent {
        let outpoint = OutPoint::from(asset_lock_outpoint);

        result.add_error(IdentityAssetLockTransactionOutPointAlreadyExistsError::new(
            outpoint.txid,
            outpoint.vout as usize,
        ))
    }

    // Verify instant lock signature with Core

    let is_instant_lock_signature_valid = verify_instant_lock(
        platform_ref.core_rpc,
        instant_asset_lock_proof.instant_lock(),
        platform_ref.block_info.core_height,
    )?;

    if !is_instant_lock_signature_valid {
        result.add_error(InvalidInstantAssetLockProofSignatureError::new());

        return Ok(result);
    }

    Ok(result)
}

fn verify_instant_lock<C: CoreRPCLike>(
    core_rpc: &C,
    instant_lock: &InstantLock,
    core_chain_locked_height: u32,
) -> Result<bool, Error> {
    match core_rpc.verify_instant_lock(instant_lock, Some(core_chain_locked_height)) {
        Ok(result) => Ok(result),
        // Consider signature is invalid in case if instant lock data format is wrong for some reason
        Err(dashcore_rpc::Error::JsonRpc(dashcore_rpc::jsonrpc::error::Error::Rpc(
            dashcore_rpc::jsonrpc::error::RpcError {
                code:
                    CORE_RPC_PARSE_ERROR | CORE_RPC_INVALID_ADDRESS_OR_KEY | CORE_RPC_INVALID_PARAMETER,
                ..
            },
        ))) => Ok(false),
        Err(e) => Err(Error::Execution(ExecutionError::DashCoreBadResponseError(
            format!(
                "can't verify instant asset lock proof signature with core: {}",
                e.to_string()
            ),
        ))),
    }
}
