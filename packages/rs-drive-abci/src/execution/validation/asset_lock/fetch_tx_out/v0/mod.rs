use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionIsNotFoundError, IdentityAssetLockTransactionOutputNotFoundError,
    InvalidAssetLockProofCoreChainHeightError,
    InvalidIdentityAssetLockProofChainLockValidationError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{OutPoint, TxOut};
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use dpp::validation::ValidationResult;

/// A trait for fetching asset lock transaction output from a Core RPC-like instance.
pub trait FetchAssetLockProofTxOutV0 {
    /// Synchronously fetches the asset lock transaction output from the core
    /// based on either an `Instant` or a `Chain` asset lock proof.
    ///
    /// # Type Parameters
    ///
    /// * `C`: CoreRPCLike - a type that implements the CoreRPCLike trait.
    ///
    /// # Parameters
    ///
    /// * `core`: &C - a reference to a CoreRPC-like instance.
    ///
    /// # Returns
    ///
    /// * `Result<ValidationResult<TxOut, ConsensusError>, Error>`:
    ///   - Ok(ValidationResult::WithData) if the transaction output is found,
    ///   - Ok(ValidationResult::WithError) for any errors encountered during the process,
    ///   - Err(Error) if there's an execution error.
    fn fetch_asset_lock_transaction_output_sync_v0<C: CoreRPCLike>(
        &self,
        core: &C,
    ) -> Result<ValidationResult<TxOut, ConsensusError>, Error>;
}

impl FetchAssetLockProofTxOutV0 for AssetLockProof {
    /// This fetches the asset lock transaction output from core
    fn fetch_asset_lock_transaction_output_sync_v0<C: CoreRPCLike>(
        &self,
        core: &C,
    ) -> Result<ValidationResult<TxOut, ConsensusError>, Error> {
        match self {
            AssetLockProof::Instant(asset_lock_proof) => {
                if let Some(output) = asset_lock_proof.output() {
                    Ok(ValidationResult::new_with_data(output.clone()))
                } else {
                    Ok(ValidationResult::new_with_error(
                        BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                            IdentityAssetLockTransactionOutputNotFoundError::new(
                                asset_lock_proof.output_index(),
                            ),
                        )
                        .into(),
                    ))
                }
            }
            AssetLockProof::Chain(asset_lock_proof) => {
                let out_point = OutPoint::from(asset_lock_proof.out_point.to_buffer());

                let output_index = out_point.vout as usize;
                let transaction_hash = out_point.txid;

                let transaction_data = match core.get_transaction_extended_info(&transaction_hash) {
                    Ok(transaction) => transaction,
                    Err(_e) => {
                        //todo: deal with IO errors
                        return Ok(ValidationResult::new_with_error(
                            BasicError::IdentityAssetLockTransactionIsNotFoundError(
                                IdentityAssetLockTransactionIsNotFoundError::new(
                                    transaction_hash.to_byte_array(),
                                ),
                            )
                            .into(),
                        ));
                    }
                };

                if !transaction_data.chainlock {
                    let best_chain_lock = core.get_best_chain_lock()?; //todo: this could be a problem
                    if asset_lock_proof.core_chain_locked_height > best_chain_lock.core_block_height
                    {
                        // we received a chain lock height that is too new
                        //todo: there is a race condition here, the chain lock proof should contain more information
                        // so that in the event that core responds that the transaction is not chain locked we
                        // can verify the chain lock proof itself.
                        return Ok(ValidationResult::new_with_error(
                            BasicError::InvalidAssetLockProofCoreChainHeightError(
                                InvalidAssetLockProofCoreChainHeightError::new(
                                    asset_lock_proof.core_chain_locked_height,
                                    best_chain_lock.core_block_height,
                                ),
                            )
                            .into(),
                        ));
                    } else {
                        // it's possible that in the meantime the transaction was locked, lets try again
                        let transaction_data =
                            match core.get_transaction_extended_info(&transaction_hash) {
                                Ok(transaction) => transaction,
                                Err(_e) => {
                                    return Ok(ValidationResult::new_with_error(
                                        BasicError::IdentityAssetLockTransactionIsNotFoundError(
                                            IdentityAssetLockTransactionIsNotFoundError::new(
                                                transaction_hash.to_byte_array(),
                                            ),
                                        )
                                        .into(),
                                    ))
                                }
                            };

                        if !transaction_data.chainlock {
                            // Very weird
                            // We are getting back that the transaction is not locked, but the chain
                            // lock proof says that it should be, most likely the client is lying.
                            // it would seem that the chain lock is malformed or we are under attack
                            // todo: log this event
                            // todo: ban the ip sending this request
                            return Ok(ValidationResult::new_with_error(
                                BasicError::InvalidIdentityAssetLockProofChainLockValidationError(
                                    InvalidIdentityAssetLockProofChainLockValidationError::new(
                                        transaction_hash,
                                        asset_lock_proof.core_chain_locked_height,
                                    ),
                                )
                                .into(),
                            ));
                        }
                    }
                }

                let transaction = transaction_data.transaction().map_err(|e| {
                    Error::Execution(ExecutionError::DashCoreConsensusEncodeError(e))
                })?;
                if let Some(tx_out) = transaction.output.get(output_index) {
                    Ok(ValidationResult::new_with_data(tx_out.clone()))
                } else {
                    // Also seems to be a malformed asset lock
                    // todo: log this event
                    // todo: ban the ip sending this request
                    Ok(ValidationResult::new_with_error(
                        BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                            IdentityAssetLockTransactionOutputNotFoundError::new(output_index),
                        )
                        .into(),
                    ))
                }
            }
        }
    }
}
