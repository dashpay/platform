use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::GetTransactionResult;
use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionIsNotFoundError, IdentityAssetLockTransactionOutputNotFoundError,
    InvalidAssetLockProofTransactionHeightError, InvalidAssetLockTransactionOutputReturnSizeError,
    InvalidIdentityAssetLockTransactionOutputError,
};
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{OutPoint, Transaction, TxOut, Txid};
use dpp::prelude::{AssetLockProof, ConsensusValidationResult};
use dpp::validation::{SimpleConsensusValidationResult, ValidationResult};

/// This fetches the asset lock transaction output from core
pub fn fetch_asset_lock_transaction_output_sync<C: CoreRPCLike>(
    core_rpc: &C,
    asset_lock_proof: &AssetLockProof,
) -> Result<ConsensusValidationResult<TxOut>, Error> {
    match asset_lock_proof {
        AssetLockProof::Instant(proof) => {
            if let Some(output) = proof.output() {
                Ok(ValidationResult::new_with_data(output.clone()))
            } else {
                Ok(ValidationResult::new_with_error(
                    IdentityAssetLockTransactionOutputNotFoundError::new(proof.output_index()),
                ))
            }
        }
        AssetLockProof::Chain(proof) => {
            let out_point = OutPoint::from(proof.out_point.to_buffer());

            let output_index = out_point.vout as usize;
            let transaction_hash = out_point.txid;

            // Fetch transaction

            let Some(transaction_info) = fetch_transaction_info(core_rpc, &transaction_hash)? else {
                // Transaction hash bytes needs to be reversed to match actual transaction hash
                let mut hash = transaction_hash.as_hash().into_inner();
                hash.reverse();

                return Ok(ValidationResult::new_with_error(
                    IdentityAssetLockTransactionIsNotFoundError::new(
                        hash,
                    ),
                ));
            };

            // Make sure transaction is mined on the chain locked block or before

            let Some(transaction_height) = transaction_info.blockindex else {
                return Ok(ConsensusValidationResult::new_with_error(InvalidAssetLockProofTransactionHeightError::new(
                    proof.core_chain_locked_height,
                    None,
                )));
            };

            if transaction_height > proof.core_chain_locked_height {
                return Ok(ConsensusValidationResult::new_with_error(
                    InvalidAssetLockProofTransactionHeightError::new(
                        proof.core_chain_locked_height,
                        Some(transaction_height),
                    ),
                ));
            }

            let transaction = transaction_info
                .transaction()
                .map_err(|e| Error::Execution(ExecutionError::DashCoreConsensusEncodeError(e)))?;

            // Validate asset lock transaction

            let validate_asset_lock_transaction_result =
                validate_asset_lock_transaction_structure(&transaction, output_index)?;

            if !validate_asset_lock_transaction_result.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(
                    validate_asset_lock_transaction_result.errors,
                ));
            }

            let Some(tx_out) = transaction.output.get(output_index) else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution("should be validated above in validate_asset_lock_transaction_structure")))
            };

            Ok(ValidationResult::new_with_data(tx_out.clone()))
        }
    }
}

/// Validates asset lock transaction structure
pub fn validate_asset_lock_transaction_structure(
    transaction: &Transaction,
    output_index: usize,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut result = SimpleConsensusValidationResult::default();

    let Some(output) = transaction.output.get(output_index) else {
        result.add_error(IdentityAssetLockTransactionOutputNotFoundError::new(
            output_index,
        ));

        return Ok(result)
    };

    if !output.script_pubkey.is_op_return() {
        result.add_error(InvalidIdentityAssetLockTransactionOutputError::new(
            output_index,
        ));
        return Ok(result);
    }

    // Slicing from 1 bytes, which is OP_RETURN, to the end of the script
    let public_key_hash = &output.script_pubkey.as_bytes()[2..];
    // 20 bytes is the size of ripemd160, which should be stored after the OP_RETURN
    if public_key_hash.len() != 20 {
        result.add_error(InvalidAssetLockTransactionOutputReturnSizeError::new(
            output_index,
        ));
        return Ok(result);
    }

    Ok(result)
}

fn fetch_transaction_info<C: CoreRPCLike>(
    core_rpc: &C,
    transaction_id: &Txid,
) -> Result<Option<GetTransactionResult>, Error> {
    match core_rpc.get_transaction_extended_info(transaction_id) {
        Ok(transaction) => Ok(Some(transaction)),
        // Return None if transaction with specified tx id is not present
        Err(dashcore_rpc::Error::JsonRpc(dashcore_rpc::jsonrpc::error::Error::Rpc(
            dashcore_rpc::jsonrpc::error::RpcError {
                code: CORE_RPC_INVALID_ADDRESS_OR_KEY,
                ..
            },
        ))) => Ok(None),
        Err(e) => Err(Error::Execution(ExecutionError::DashCoreBadResponseError(
            format!(
                "can't fetch asset lock transaction for chain asset lock proof: {}",
                e.to_string()
            ),
        ))),
    }
}
