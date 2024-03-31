use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionOutPointAlreadyConsumedError,
    IdentityAssetLockTransactionOutPointNotEnoughBalanceError,
};
use dpp::dashcore::OutPoint;
use dpp::fee::Credits;
use dpp::platform_value::Bytes36;
use dpp::prelude::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

/// Both proofs share the same verification logic
#[inline(always)]
pub(super) fn verify_asset_lock_is_not_spent_and_has_enough_balance_v0<C>(
    platform_ref: &PlatformRef<C>,
    out_point: OutPoint,
    required_balance: Credits,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<AssetLockValue>, Error> {
    // Make sure that asset lock isn't spent yet

    let stored_asset_lock_info = platform_ref.drive.fetch_asset_lock_outpoint_info(
        &Bytes36::new(out_point.into()),
        transaction,
        &platform_version.drive,
    )?;

    match stored_asset_lock_info {
        StoredAssetLockInfo::FullyConsumed => {
            // It was already entirely spent
            Ok(ConsensusValidationResult::new_with_error(
                IdentityAssetLockTransactionOutPointAlreadyConsumedError::new(
                    out_point.txid,
                    out_point.vout as usize,
                )
                .into(),
            ))
        }
        StoredAssetLockInfo::PartiallyConsumed(reduced_asset_lock_value) => {
            if reduced_asset_lock_value.remaining_credit_value() == 0 {
                Ok(ConsensusValidationResult::new_with_error(
                    IdentityAssetLockTransactionOutPointAlreadyConsumedError::new(
                        out_point.txid,
                        out_point.vout as usize,
                    )
                    .into(),
                ))
            } else if reduced_asset_lock_value.remaining_credit_value() < required_balance {
                Ok(ConsensusValidationResult::new_with_error(
                    IdentityAssetLockTransactionOutPointNotEnoughBalanceError::new(
                        out_point.txid,
                        out_point.vout as usize,
                        reduced_asset_lock_value.initial_credit_value(),
                        reduced_asset_lock_value.remaining_credit_value(),
                        required_balance,
                    )
                    .into(),
                ))
            } else {
                Ok(ConsensusValidationResult::new_with_data(
                    reduced_asset_lock_value,
                ))
            }
        }
        StoredAssetLockInfo::NotPresent => Ok(ConsensusValidationResult::new()),
    }
}
