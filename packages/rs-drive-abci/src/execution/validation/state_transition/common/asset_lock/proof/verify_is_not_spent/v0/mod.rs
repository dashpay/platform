use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::consensus::basic::identity::{
    IdentityAssetLockStateTransitionReplayError,
    IdentityAssetLockTransactionOutPointAlreadyConsumedError,
    IdentityAssetLockTransactionOutPointNotEnoughBalanceError,
};
use dpp::dashcore::OutPoint;
use dpp::fee::Credits;
use dpp::platform_value::Bytes36;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

/// Both proofs share the same verification logic
#[inline(always)]
pub(super) fn verify_asset_lock_is_not_spent_and_has_enough_balance_v0<C>(
    platform_ref: &PlatformRef<C>,
    signable_bytes_hasher: &mut SignableBytesHasher,
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
            } else if signable_bytes_hasher
                .hash_bytes_and_check_if_vec_contains(reduced_asset_lock_value.used_tags_ref())
            {
                // Here we check that the transaction was not already tried to be execution
                // This is the replay attack prevention
                Ok(ConsensusValidationResult::new_with_error(
                    IdentityAssetLockStateTransitionReplayError::new(
                        out_point.txid,
                        out_point.vout as usize,
                        signable_bytes_hasher.to_hashed_bytes(),
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

#[cfg(test)]
mod tests {
    // NOTE on `PlatformVersion::latest()` vs `::first()`:
    // `TestPlatformBuilder::build_with_mock_rpc()` initializes the underlying
    // drive / state structure at `PlatformVersion::latest()` unless an explicit
    // `initial_protocol_version` is supplied. Because the function under test
    // delegates to `drive.fetch_asset_lock_outpoint_info(&..., &platform_version.drive)`,
    // the version passed here must match the drive version the platform was
    // built with — otherwise a DriveError::UnknownVersionMismatch fires on
    // every call. These tests therefore intentionally use `latest()` to stay
    // aligned with the builder. If v0-specific logic is ever asserted here
    // (beyond the _v0 function dispatch, which is already direct), rebuild the
    // platform via `.with_initial_protocol_version(PlatformVersion::first().protocol_version)`.
    use super::*;
    use crate::platform_types::platform::PlatformRef;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::Txid;
    use drive::util::batch::DriveOperation::SystemOperation;
    use drive::util::batch::SystemOperationType;

    fn build_out_point(seed: u8) -> OutPoint {
        let txid = Txid::from_raw_hash(dpp::dashcore::hashes::sha256d::Hash::from_byte_array(
            [seed; 32],
        ));
        OutPoint { txid, vout: 0 }
    }

    fn store_asset_lock_value(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        out_point: OutPoint,
        value: AssetLockValue,
        platform_version: &PlatformVersion,
    ) {
        let op = SystemOperation(SystemOperationType::AddUsedAssetLock {
            asset_lock_outpoint: Bytes36::new(out_point.into()),
            asset_lock_value: value,
        });
        platform
            .drive
            .apply_drive_operations(
                vec![op],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }

    #[test]
    fn should_return_valid_empty_result_when_not_present() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };

        let out_point = build_out_point(0xAA);
        let mut hasher = SignableBytesHasher::Bytes(b"not-yet-tried".to_vec());

        let result = verify_asset_lock_is_not_spent_and_has_enough_balance_v0(
            &platform_ref,
            &mut hasher,
            out_point,
            1_000,
            None,
            platform_version,
        )
        .expect("should not error");

        // NotPresent -> returns an empty, still-valid ConsensusValidationResult with no data set
        assert!(result.is_valid(), "not present should yield valid result");
        assert!(
            result.data.is_none(),
            "no AssetLockValue should be returned for NotPresent"
        );
    }

    #[test]
    fn should_return_already_consumed_error_when_fully_consumed() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let out_point = build_out_point(0xBB);

        // A 0-remaining-credit value is stored with an empty item, representing FullyConsumed.
        let zero_remaining =
            AssetLockValue::new(10_000, vec![0x76, 0xa9, 0x14], 0, vec![], platform_version)
                .expect("should build asset lock value");

        store_asset_lock_value(&platform, out_point, zero_remaining, platform_version);

        let platform_state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };

        let mut hasher = SignableBytesHasher::Bytes(b"sig-bytes".to_vec());

        let result = verify_asset_lock_is_not_spent_and_has_enough_balance_v0(
            &platform_ref,
            &mut hasher,
            out_point,
            1_000,
            None,
            platform_version,
        )
        .expect("should not error");

        assert!(!result.is_valid(), "fully consumed should be invalid");
        assert!(
            result.errors.iter().any(|e| matches!(
                e,
                ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionOutPointAlreadyConsumedError(_)
                )
            )),
            "expected AlreadyConsumed error, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_return_not_enough_balance_when_remaining_below_required() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let out_point = build_out_point(0xCC);

        // Partially consumed: remaining = 500 < required 1000
        let partial = AssetLockValue::new(10_000, vec![0x76, 0xa9], 500, vec![], platform_version)
            .expect("should build asset lock value");

        store_asset_lock_value(&platform, out_point, partial, platform_version);

        let platform_state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };

        let mut hasher = SignableBytesHasher::Bytes(b"sig-not-yet-used".to_vec());

        let result = verify_asset_lock_is_not_spent_and_has_enough_balance_v0(
            &platform_ref,
            &mut hasher,
            out_point,
            1_000,
            None,
            platform_version,
        )
        .expect("should not error");

        assert!(!result.is_valid(), "should be invalid");
        assert!(
            result.errors.iter().any(|e| matches!(
                e,
                ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionOutPointNotEnoughBalanceError(_)
                )
            )),
            "expected NotEnoughBalance error, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_return_replay_error_when_signable_bytes_already_used() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let out_point = build_out_point(0xDD);

        // Pre-hash the signable bytes to compute the tag that will be matched
        let mut pre_hasher = SignableBytesHasher::Bytes(b"prev-tried-bytes".to_vec());
        let used_tag = pre_hasher.to_hashed_bytes();

        let partial = AssetLockValue::new(
            10_000,
            vec![0x76, 0xa9],
            5_000,
            vec![used_tag],
            platform_version,
        )
        .expect("should build asset lock value");

        store_asset_lock_value(&platform, out_point, partial, platform_version);

        let platform_state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };

        // A fresh hasher with the same bytes — the stored used_tag should match.
        let mut hasher = SignableBytesHasher::Bytes(b"prev-tried-bytes".to_vec());

        let result = verify_asset_lock_is_not_spent_and_has_enough_balance_v0(
            &platform_ref,
            &mut hasher,
            out_point,
            1_000,
            None,
            platform_version,
        )
        .expect("should not error");

        assert!(!result.is_valid(), "should be invalid");
        assert!(
            result.errors.iter().any(|e| matches!(
                e,
                ConsensusError::BasicError(
                    BasicError::IdentityAssetLockStateTransitionReplayError(_)
                )
            )),
            "expected ReplayError, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_return_value_when_enough_balance_and_not_replayed() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let out_point = build_out_point(0xEE);

        let partial =
            AssetLockValue::new(10_000, vec![0x76, 0xa9], 5_000, vec![], platform_version)
                .expect("should build asset lock value");

        store_asset_lock_value(&platform, out_point, partial, platform_version);

        let platform_state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };

        let mut hasher = SignableBytesHasher::Bytes(b"brand-new-signable".to_vec());

        let result = verify_asset_lock_is_not_spent_and_has_enough_balance_v0(
            &platform_ref,
            &mut hasher,
            out_point,
            1_000,
            None,
            platform_version,
        )
        .expect("should not error");

        assert!(result.is_valid(), "should be valid: {:?}", result.errors);
        let value = result.data.expect("should have asset lock value");
        assert_eq!(value.remaining_credit_value(), 5_000);
    }
}
