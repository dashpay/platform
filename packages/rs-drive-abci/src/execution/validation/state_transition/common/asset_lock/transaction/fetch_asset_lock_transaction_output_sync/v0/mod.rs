use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionIsNotFoundError, IdentityAssetLockTransactionOutputNotFoundError,
    InvalidAssetLockProofTransactionHeightError,
};
use dpp::dashcore::secp256k1::ThirtyTwoByteHash;
use dpp::dashcore::transaction::special_transaction::TransactionPayload;
use dpp::dashcore::TxOut;
use dpp::identity::state_transition::asset_lock_proof::validate_asset_lock_transaction_structure::validate_asset_lock_transaction_structure;
use dpp::prelude::{AssetLockProof, ConsensusValidationResult};
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

/// This fetches the asset lock transaction output from core
pub fn fetch_asset_lock_transaction_output_sync_v0<C: CoreRPCLike>(
    core_rpc: &C,
    asset_lock_proof: &AssetLockProof,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<TxOut>, Error> {
    match asset_lock_proof {
        AssetLockProof::Instant(proof) => {
            if let Some(output) = proof.output() {
                Ok(ValidationResult::new_with_data(output.clone()))
            } else {
                Ok(ValidationResult::new_with_error(
                    IdentityAssetLockTransactionOutputNotFoundError::new(
                        proof.output_index() as usize
                    )
                    .into(),
                ))
            }
        }
        AssetLockProof::Chain(proof) => {
            let output_index = proof.out_point.vout;
            let transaction_hash = proof.out_point.txid;

            // Fetch transaction

            let maybe_transaction_info = core_rpc
                .get_optional_transaction_extended_info(&transaction_hash)
                .map_err(|e| {
                    Error::Execution(ExecutionError::DashCoreBadResponseError(format!(
                        "can't fetch asset transaction for chain asset lock proof: {e}",
                    )))
                })?;

            let Some(transaction_info) = maybe_transaction_info else {
                // Transaction hash bytes needs to be reversed to match actual transaction hash
                let mut hash = transaction_hash.as_raw_hash().into_32();
                hash.reverse();

                return Ok(ValidationResult::new_with_error(
                    IdentityAssetLockTransactionIsNotFoundError::new(hash).into(),
                ));
            };

            // Make sure transaction is mined on the chain locked block or before

            let Some(transaction_height) = transaction_info.height else {
                return Ok(ConsensusValidationResult::new_with_error(
                    InvalidAssetLockProofTransactionHeightError::new(
                        proof.core_chain_locked_height,
                        None,
                    )
                    .into(),
                ));
            };

            let is_transaction_not_mined = transaction_height == -1;
            let transaction_height = transaction_height as u32;

            // Return an error if transaction is not mined
            // or if it is mined after the chain locked height
            if is_transaction_not_mined || transaction_height > proof.core_chain_locked_height {
                let reported_height = if is_transaction_not_mined {
                    None
                } else {
                    Some(transaction_height)
                };

                return Ok(ConsensusValidationResult::new_with_error(
                    InvalidAssetLockProofTransactionHeightError::new(
                        proof.core_chain_locked_height,
                        reported_height,
                    )
                    .into(),
                ));
            }

            let transaction = transaction_info
                .transaction()
                .map_err(|e| Error::Execution(ExecutionError::DashCoreConsensusEncodeError(e)))?;

            // Validate asset lock transaction

            let validate_asset_lock_transaction_result = validate_asset_lock_transaction_structure(
                &transaction,
                output_index,
                platform_version,
            )?;

            if !validate_asset_lock_transaction_result.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(
                    validate_asset_lock_transaction_result.errors,
                ));
            }

            // Extract outpoint from the payload
            if let Some(TransactionPayload::AssetLockPayloadType(mut payload)) =
                transaction.special_transaction_payload
            {
                // We are dealing with old Rust edition so we can't use optional remove
                if payload.credit_outputs.get(output_index as usize).is_some() {
                    let output = payload.credit_outputs.remove(output_index as usize);
                    return Ok(ValidationResult::new_with_data(output));
                }
            }

            Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "transaction should have outpoint",
            )))
        }
    }
}
