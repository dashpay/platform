use dpp::asset_lock::reduced_asset_lock_value::ReducedAssetLockValue;
use dpp::asset_lock::StoredAssetLockInfo;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::basic::identity::{IdentityAssetLockTransactionOutPointAlreadyConsumedError, IdentityAssetLockTransactionOutPointNotEnoughBalanceError};
use dpp::fee::Credits;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::platform_value::Bytes36;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::execution::validation::state_transition::common::asset_lock::proof::verify_is_not_spent::AssetLockProofVerifyIsNotSpent;
use dpp::asset_lock::reduced_asset_lock_value::ReducedAssetLockValueGettersV0;

// TODO: Versioning
impl AssetLockProofVerifyIsNotSpent for ChainAssetLockProof {
    fn verify_is_not_spent_and_has_enough_balance<C>(
        &self,
        platform_ref: &PlatformRef<C>,
        required_balance: Credits,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<ReducedAssetLockValue>, Error> {

        // Make sure that asset lock isn't spent yet

        let outpoint_bytes = self.out_point.try_into().map_err(|_e| {
            Error::Execution(ExecutionError::Conversion(String::from(
                "can't convert output to bytes",
            )))
        })?;

        let stored_asset_lock_info = platform_ref.drive.fetch_asset_lock_outpoint_info(
            &Bytes36::new(outpoint_bytes),
            transaction,
            &platform_version.drive,
        )?;

        match stored_asset_lock_info {
            StoredAssetLockInfo::Present => {
                // It was already entirely spent
                Ok(ConsensusValidationResult::new_with_error(IdentityAssetLockTransactionOutPointAlreadyConsumedError::new(
                    self.out_point.txid,
                    self.out_point.vout as usize,
                ).into()))
            }
            StoredAssetLockInfo::PresentWithInfo(reduced_asset_lock_value) => {
                if reduced_asset_lock_value.remaining_credit_value() < required_balance {
                    Ok(ConsensusValidationResult::new_with_error(IdentityAssetLockTransactionOutPointNotEnoughBalanceError::new(
                        self.out_point.txid,
                        self.out_point.vout as usize,
                        reduced_asset_lock_value.initial_credit_value(),
                        reduced_asset_lock_value.remaining_credit_value(),
                        required_balance,
                    ).into()))
                } else {
                    Ok(ConsensusValidationResult::new_with_data(reduced_asset_lock_value))
                }
            }
            StoredAssetLockInfo::NotPresent => {
                Ok(ConsensusValidationResult::new())
            }
        }
    }
}
