use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dashcore::hashes::Hash;
use dashcore::{OutPoint, TxOut};
use dpp::consensus::basic::identity::{
    IdentityAssetLockProofLockedTransactionMismatchError,
    IdentityAssetLockTransactionIsNotFoundError, IdentityAssetLockTransactionOutputNotFoundError,
    InvalidAssetLockProofCoreChainHeightError,
    InvalidIdentityAssetLockProofChainLockValidationError,
};
use dpp::consensus::ConsensusError;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use dpp::validation::ValidationResult;
use drive::drive::Drive;

pub trait FetchAssetLockProofTxOut {
    fn fetch_asset_lock_transaction_output_sync<C: CoreRPCLike>(
        &self,
        core: &C,
    ) -> Result<ValidationResult<TxOut, ConsensusError>, Error>;
}

impl FetchAssetLockProofTxOut for AssetLockProof {
    /// This fetches the asset lock transaction output from core
    fn fetch_asset_lock_transaction_output_sync<C: CoreRPCLike>(
        &self,
        core: &C,
    ) -> Result<ValidationResult<TxOut, ConsensusError>, Error> {
        match self {
            AssetLockProof::Instant(asset_lock_proof) => {
                if let Some(output) = asset_lock_proof.output() {
                    Ok(ValidationResult::new_with_data(output.clone()))
                } else {
                    Ok(ValidationResult::new_with_error(
                        ConsensusError::IdentityAssetLockTransactionOutputNotFoundError(
                            IdentityAssetLockTransactionOutputNotFoundError::new(
                                asset_lock_proof.output_index(),
                            ),
                        ),
                    ))
                }
            }
            AssetLockProof::Chain(asset_lock_proof) => {
                let out_point = OutPoint::from(asset_lock_proof.out_point.to_buffer());

                let output_index = out_point.vout as usize;
                let transaction_hash = out_point.txid;

                let transaction_data = match core.get_transaction_extended_info(&transaction_hash) {
                    Ok(transaction) => transaction,
                    Err(e) => {
                        //todo: deal with IO errors
                        return Ok(ValidationResult::new_with_error(
                            ConsensusError::IdentityAssetLockTransactionIsNotFoundError(
                                IdentityAssetLockTransactionIsNotFoundError::new(
                                    transaction_hash.as_hash().into_inner(),
                                ),
                            ),
                        ));
                    }
                };

                if transaction_data.chainlock == false {
                    let best_chain_lock = core.get_best_chain_lock()?;
                    if asset_lock_proof.core_chain_locked_height > best_chain_lock.core_block_height
                    {
                        // we received a chain lock height that is too new
                        //todo: there is a race condition here, the chain lock proof should contain more information
                        // so that in the event that core responds that the transaction is not chain locked we
                        // can verify the chain lock proof itself.
                        return Ok(ValidationResult::new_with_error(
                            ConsensusError::InvalidAssetLockProofCoreChainHeightError(
                                InvalidAssetLockProofCoreChainHeightError::new(
                                    asset_lock_proof.core_chain_locked_height,
                                    best_chain_lock.core_block_height,
                                ),
                            ),
                        ));
                    } else {
                        // it's possible that in the meantime the transaction was locked, lets try again
                        let transaction_data =
                            match core.get_transaction_extended_info(&transaction_hash) {
                                Ok(transaction) => transaction,
                                Err(e) => {
                                    return Ok(ValidationResult::new_with_error(
                                        ConsensusError::IdentityAssetLockTransactionIsNotFoundError(
                                            IdentityAssetLockTransactionIsNotFoundError::new(
                                                transaction_hash.as_hash().into_inner(),
                                            ),
                                        ),
                                    ))
                                }
                            };

                        if transaction_data.chainlock == false {
                            // Very weird
                            // We are getting back that the transaction is not locked, but the chain
                            // lock proof says that it should be, most likely the client is lying.
                            // it would seem that the chain lock is malformed or we are under attack
                            // todo: log this event
                            // todo: ban the ip sending this request
                            return Ok(ValidationResult::new_with_error(
                                ConsensusError::InvalidIdentityAssetLockProofChainLockValidationError(
                                    InvalidIdentityAssetLockProofChainLockValidationError::new(
                                        transaction_hash,
                                        asset_lock_proof.core_chain_locked_height,
                                    ),
                                ),
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
                        ConsensusError::IdentityAssetLockTransactionOutputNotFoundError(
                            IdentityAssetLockTransactionOutputNotFoundError::new(output_index),
                        ),
                    ))
                }
            }
        }
    }
}
