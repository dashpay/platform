use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Version 1: identical to v0 except for `PaidFromAssetLockToPool` (the
    /// `ShieldFromAssetLock` path), which is no longer gated on a per-transition GroveDB cost
    /// estimate.
    ///
    /// The pool fee a `ShieldFromAssetLock` books is the flat
    /// `compute_minimum_shielded_fee(num_actions) + asset-lock base cost` (its transform enforces
    /// `lock_value >= shield_amount + pool_fee`), i.e. the same kind of flat fee a
    /// `ShieldedTransfer`/`Unshield` carries — and those pool-paid events were never
    /// estimate-gated. Under GROVE_V4 the commitment-tree estimator is a deliberate per-append
    /// UPPER BOUND (a full epoch compaction and dense recompute charged on every note), so
    /// `estimate >= actual` holds by construction and the v0 gate `fee >= estimate` degenerates
    /// into a fixed comparison of two consensus constants — one the flat fee can never clear
    /// without pricing every shield at the once-per-epoch compaction spike. The flat fee is
    /// instead pinned against the epoch-AMORTIZED real cost (the
    /// `test_minimum_shielded_fee_covers_actual_grovedb_write_cost` tests in rs-drive); the pool
    /// absorbs the boundary append by design, and execution books
    /// `storage = min(actual_storage, fee)`, `processing = fee - storage` whatever the estimate
    /// would have said.
    pub(super) fn validate_fees_of_event_v1(
        &self,
        event: &ExecutionEvent,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<ConsensusValidationResult<FeeResult>, Error> {
        match event {
            ExecutionEvent::PaidFromAssetLockToPool {
                fees_to_add_to_pool,
                ..
            } => {
                // Advertise the authoritative pool fee for `gas_wanted`. Its storage/processing
                // split is only known once the operations are metered at execution, so the whole
                // fee is advertised as processing here; the total — what `gas_wanted` carries —
                // is exact.
                Ok(ConsensusValidationResult::new_with_data(
                    FeeResult::default_with_fees(0, *fees_to_add_to_pool),
                ))
            }
            _ => self.validate_fees_of_event_v0(
                event,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::execution_operation::ValidationOperation;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;

    /// Same event, same platform: a flat pool fee far below the estimated cost of the
    /// transition's operations is rejected by v0 (`fee >= estimate`) and admitted by v1, which
    /// still advertises the authoritative pool fee for `gas_wanted`.
    #[test]
    fn validate_fees_of_event_v1_paid_from_asset_lock_to_pool_is_not_estimate_gated() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let previous_fee_versions = Default::default();

        let fees_to_add_to_pool = 1_000_000u64;
        let estimated_cost = FeeResult::default_with_fees(0, 10 * fees_to_add_to_pool);
        let event = ExecutionEvent::PaidFromAssetLockToPool {
            fees_to_add_to_pool,
            added_to_balance_outputs: None,
            operations: vec![],
            execution_operations: vec![ValidationOperation::PrecalculatedOperation(estimated_cost)],
        };

        let v0 = platform
            .platform
            .validate_fees_of_event_v0(
                &event,
                &block_info,
                None,
                platform_version,
                &previous_fee_versions,
            )
            .expect("v0 must be Ok");
        assert!(
            matches!(
                v0.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::InvalidShieldedProofError(_)
                )]
            ),
            "v0 gates the flat pool fee on the estimate, got {:?}",
            v0.errors
        );

        let v1 = platform
            .platform
            .validate_fees_of_event_v1(
                &event,
                &block_info,
                None,
                platform_version,
                &previous_fee_versions,
            )
            .expect("v1 must be Ok");
        assert!(
            v1.errors.is_empty(),
            "v1 must not estimate-gate the flat pool fee, got {:?}",
            v1.errors
        );
        let fee = v1.into_data().expect("fee result present");
        assert_eq!(
            fee.total_base_fee(),
            fees_to_add_to_pool,
            "advertised pool fee must equal fees_to_add_to_pool (gas_wanted parity)"
        );
    }
}
