mod v0;
mod v1;

use super::StateTransitionAwareError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::platform::Platform;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::Credits;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Turn a validated execution event into a [`StateTransitionExecutionResult`], applying the paid
    /// event and mapping its outcome.
    ///
    /// Versioned because the recent-address-balance recorded SET expanded at protocol v13: v0 records
    /// NOTHING for paid-invalid / unsuccessful-paid transitions (byte parity with pre-v13 nodes), v1
    /// records their balance effects. The outer `process_raw_state_transitions` loop is UNCHANGED
    /// across the bump — only this helper's behavior is — so only this helper is versioned; the loop
    /// calls this dispatcher exactly like `execute_event_v0` calls the dispatching
    /// `record_added_balance_outputs`.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::execution) fn process_validation_result<'a>(
        &self,
        raw_state_transition: &'a [u8],
        state_transition_name: &str,
        validation_result: ConsensusValidationResult<ExecutionEvent>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        block_credit_mints: &mut Credits,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<StateTransitionExecutionResult, StateTransitionAwareError<'a>> {
        match platform_version
            .drive_abci
            .methods
            .state_transition_processing
            .process_validation_result
        {
            0 => self.process_validation_result_v0(
                raw_state_transition,
                state_transition_name,
                validation_result,
                block_info,
                transaction,
                block_credit_mints,
                platform_version,
                previous_fee_versions,
            ),
            1 => self.process_validation_result_v1(
                raw_state_transition,
                state_transition_name,
                validation_result,
                block_info,
                transaction,
                block_credit_mints,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(StateTransitionAwareError {
                error: Error::Execution(ExecutionError::UnknownVersionMismatch {
                    method: "process_validation_result".to_string(),
                    known_versions: vec![0, 1],
                    received: version,
                }),
                raw_state_transition,
                state_transition_name: Some(state_transition_name.to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::address_funds::PlatformAddress;
    use dpp::balances::credits::CreditOperation;
    use dpp::consensus::ConsensusError;
    use std::collections::BTreeMap;

    // A `PaidFromShieldedPool` with `chargeable_failure = true` carrying an output credit is the shape
    // the IdentityCreateFromShieldedPool duplicate-key fallback produces at this seam: an APPLIED,
    // paid-invalid transition that still credits the fallback address (the net unshielded amount).
    fn fallback_event<'a>(
        output: Option<(PlatformAddress, dpp::fee::Credits)>,
    ) -> ExecutionEvent<'a> {
        ExecutionEvent::PaidFromShieldedPool {
            operations: vec![],
            fees_to_add_to_pool: 0,
            added_to_balance_outputs: output.map(|(addr, net)| BTreeMap::from([(addr, net)])),
            chargeable_failure: true,
        }
    }

    /// Drive the paid-invalid branch of the requested version (`process_validation_result_v0` when
    /// `via_v1` is false, `_v1` otherwise) and return the `PaidConsensusError`'s recorded balance
    /// changes. `platform_version` is threaded only for the executor's own versioned calls — neither
    /// helper branches on it, so this exercises each version's behavior directly.
    fn recorded_changes(
        via_v1: bool,
        event: ExecutionEvent<'_>,
        platform_version: &PlatformVersion,
    ) -> BTreeMap<PlatformAddress, CreditOperation> {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let transaction = platform.drive.grove.start_transaction();
        let fee_versions = CachedEpochIndexFeeVersions::new();

        let validation_result = ConsensusValidationResult::new_with_data_and_errors(
            event,
            vec![ConsensusError::DefaultError],
        );

        let result = if via_v1 {
            platform.platform.process_validation_result_v1(
                b"raw-st",
                "Unshield",
                validation_result,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                &fee_versions,
            )
        } else {
            platform.platform.process_validation_result_v0(
                b"raw-st",
                "Unshield",
                validation_result,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                &fee_versions,
            )
        }
        .expect("process_validation_result should not error");

        match result {
            StateTransitionExecutionResult::PaidConsensusError {
                address_balance_changes,
                ..
            } => address_balance_changes,
            other => panic!("expected PaidConsensusError, got {other:?}"),
        }
    }

    /// v0 = pre-v13 behavior (state-root parity with old nodes): an applied chargeable-failure credit
    /// is DROPPED, because v0 hands the executor no tracking map. This holds regardless of the
    /// `platform_version` passed in — v0 has no version conditional; the gating is the
    /// `process_validation_result` dispatch that routes v13 to v1 (see the v1 tests and the
    /// platform-version routing test).
    #[test]
    fn v0_records_nothing_even_with_credit() {
        let fallback_address = PlatformAddress::P2pkh([0xCD; 20]);

        for platform_version in [
            PlatformVersion::get(12).expect("v12 must exist"),
            PlatformVersion::get(13).expect("v13 must exist"),
        ] {
            let with_credit = recorded_changes(
                false,
                fallback_event(Some((fallback_address, 2500))),
                platform_version,
            );
            assert!(
                with_credit.is_empty(),
                "v0 must NOT record paid-invalid balance effects (byte parity with pre-v13 nodes)"
            );

            let without_credit = recorded_changes(false, fallback_event(None), platform_version);
            assert!(
                without_credit.is_empty(),
                "a bump-only paid-invalid transition records nothing"
            );
        }
    }

    /// v1 carries an applied paid-invalid credit into `PaidConsensusError` so
    /// `StateTransitionsProcessingResult::add` can merge it into `address_balances_updated` — the v13
    /// recorded-set expansion. The version boundary (v12 -> v0 drops, v13 -> v1 records) is covered by
    /// the platform-version routing test plus `v0_records_nothing_even_with_credit`.
    #[test]
    fn v1_records_paid_invalid_credit() {
        let fallback_address = PlatformAddress::P2pkh([0xCD; 20]);
        let changes = recorded_changes(
            true,
            fallback_event(Some((fallback_address, 2500))),
            PlatformVersion::get(13).expect("v13 must exist"),
        );
        assert_eq!(
            changes.get(&fallback_address),
            Some(&CreditOperation::AddToCredits(2500)),
            "v1 must carry the applied fallback credit into PaidConsensusError"
        );
    }

    /// A paid-invalid transition that credits no address records nothing even in v1 — the expansion
    /// must not fabricate credits for the common bump-only case.
    #[test]
    fn v1_without_address_credit_carries_empty_map() {
        let changes = recorded_changes(
            true,
            fallback_event(None),
            PlatformVersion::get(13).expect("v13 must exist"),
        );
        assert!(
            changes.is_empty(),
            "a bump-only paid-invalid transition must carry no address credit"
        );
    }
}
