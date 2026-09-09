use super::super::StateTransitionAwareError;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::event_execution_result::EventExecutionResult;
use crate::platform_types::platform::Platform;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::util::hash::hash_single;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// v1 (protocol v13+) = the recorded-set expansion for turning a validated execution event into a
    /// [`StateTransitionExecutionResult`]. It additionally records the balance effects of paid-INVALID
    /// and unsuccessful-paid transitions that v0 drops: the invalid branch hands the executor a
    /// tracking map, and both `PaidConsensusError` constructions carry it. Identical to
    /// `process_validation_result_v0` apart from this tracking/carry; the `process_validation_result`
    /// method version selects which runs, so there is no version conditional inside either.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn process_validation_result_v1<'a>(
        &self,
        raw_state_transition: &'a [u8], //used for errors
        state_transition_name: &str,
        mut validation_result: ConsensusValidationResult<ExecutionEvent>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        block_credit_mints: &mut Credits,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<StateTransitionExecutionResult, StateTransitionAwareError<'a>> {
        // State Transition is invalid
        if !validation_result.is_valid() {
            // To prevent spam we should deduct fees for invalid state transitions as well.
            // There are three cases when the user can't pay fees:
            // 1. The state transition is funded by an asset lock transactions. This transactions are
            //    placed on the payment blockchain and they can't be partially spent.
            // 2. We can't prove that the state transition is associated with the identity
            // 3. The revision given by the state transition isn't allowed based on the state
            if validation_result.data.is_none() {
                let first_consensus_error = validation_result
                    .errors
                    // the first error must be present for an invalid result
                    .remove(0);

                if tracing::enabled!(tracing::Level::DEBUG) {
                    let st_hash = hex::encode(hash_single(raw_state_transition));

                    tracing::debug!(
                        error = ?first_consensus_error,
                        st_hash,
                        "Invalid {} state transition without identity ({}): {}",
                        state_transition_name,
                        st_hash,
                        &first_consensus_error
                    );
                }

                // We don't have execution event, so we can't pay for processing
                return Ok(StateTransitionExecutionResult::UnpaidConsensusError(
                    first_consensus_error,
                ));
            };

            let (execution_event, errors) = validation_result
                .into_data_and_errors()
                .expect("data must be present since we check it few lines above");

            let first_consensus_error = errors
                .first()
                .expect("error must be present since we check it few lines above")
                .clone();

            // In this case the execution event will be to pay for the state transition processing
            // This ONLY pays for what is needed to prevent attacks on the system

            // v1 = the v13 recorded-set expansion: an applied paid-INVALID transition's transparent
            // balance effects (a chargeable-failure fallback credit, or a PaidFromAddressInputs inline
            // adjustment) ARE tracked and reach `address_balances_updated` for incremental sync. v0
            // passes `None` here and drops them (pre-v13 state parity); the `process_validation_result`
            // method version selects which runs.
            let mut address_balances = BTreeMap::new();
            let event_execution_result = self
                .execute_event(
                    execution_event,
                    errors,
                    block_info,
                    transaction,
                    Some(&mut address_balances),
                    block_credit_mints,
                    platform_version,
                    previous_fee_versions,
                )
                .map_err(|error| StateTransitionAwareError {
                    error,
                    raw_state_transition,
                    state_transition_name: Some(state_transition_name.to_string()),
                })?;

            let state_transition_execution_result = match event_execution_result {
                EventExecutionResult::SuccessfulPaidExecution(estimated_fees, actual_fees)
                | EventExecutionResult::UnsuccessfulPaidExecution(estimated_fees, actual_fees, _) =>
                {
                    if tracing::enabled!(tracing::Level::DEBUG) {
                        let st_hash = hex::encode(hash_single(raw_state_transition));

                        tracing::debug!(
                            error = ?first_consensus_error,
                            st_hash,
                            ?estimated_fees,
                            ?actual_fees,
                            "Invalid {} state transition ({}): {}",
                            state_transition_name,
                            st_hash,
                            &first_consensus_error
                        );
                    }

                    StateTransitionExecutionResult::PaidConsensusError {
                        error: first_consensus_error,
                        actual_fees,
                        // v1 carries the tracked map: applied paid-invalid credits reach the recent
                        // tree from v13 (v0 carries an empty map).
                        address_balance_changes: address_balances,
                    }
                }
                EventExecutionResult::SuccessfulFreeExecution => {
                    if tracing::enabled!(tracing::Level::DEBUG) {
                        let st_hash = hex::encode(hash_single(raw_state_transition));

                        tracing::debug!(
                            error = ?first_consensus_error,
                            st_hash,
                            "Free invalid {} state transition ({}): {}",
                            state_transition_name,
                            st_hash,
                            &first_consensus_error
                        );
                    }

                    StateTransitionExecutionResult::UnpaidConsensusError(first_consensus_error)
                }
                EventExecutionResult::UnpaidConsensusExecutionError(mut payment_errors) => {
                    let payment_consensus_error = payment_errors
                        // the first error must be present for an invalid result
                        .remove(0);

                    if tracing::enabled!(tracing::Level::ERROR) {
                        let st_hash = hex::encode(hash_single(raw_state_transition));

                        tracing::error!(
                            main_error = ?first_consensus_error,
                            payment_error = ?payment_consensus_error,
                            st_hash,
                            "Not able to reduce balance for identity {} state transition ({}): {}",
                            state_transition_name,
                            st_hash,
                            payment_consensus_error
                        );
                    }

                    StateTransitionExecutionResult::InternalError(format!(
                        "{first_consensus_error} {payment_consensus_error}",
                    ))
                }
            };

            return Ok(state_transition_execution_result);
        }

        let (execution_event, errors) =
            validation_result.into_data_and_errors().map_err(|error| {
                StateTransitionAwareError {
                    error: error.into(),
                    raw_state_transition,
                    state_transition_name: Some(state_transition_name.to_string()),
                }
            })?;

        let mut address_balances = BTreeMap::new();
        let event_execution_result = self
            .execute_event(
                execution_event,
                errors,
                block_info,
                transaction,
                Some(&mut address_balances),
                block_credit_mints,
                platform_version,
                previous_fee_versions,
            )
            .map_err(|error| StateTransitionAwareError {
                error,
                raw_state_transition,
                state_transition_name: Some(state_transition_name.to_string()),
            })?;

        let state_transition_execution_result = match event_execution_result {
            EventExecutionResult::SuccessfulPaidExecution(estimated_fees, actual_fees) => {
                if tracing::enabled!(tracing::Level::DEBUG) {
                    let st_hash = hex::encode(hash_single(raw_state_transition));

                    tracing::debug!(
                        ?actual_fees,
                        ?estimated_fees,
                        st_hash,
                        "{} state transition ({}) successfully processed",
                        state_transition_name,
                        st_hash,
                    );
                }

                StateTransitionExecutionResult::SuccessfulExecution {
                    estimated_fees,
                    fee_result: actual_fees,
                    address_balance_changes: address_balances,
                }
            }
            EventExecutionResult::UnsuccessfulPaidExecution(
                estimated_fees,
                actual_fees,
                mut errors,
            ) => {
                let payment_consensus_error = errors
                    // the first error must be present for an invalid result
                    .remove(0);

                if tracing::enabled!(tracing::Level::DEBUG) {
                    let st_hash = hex::encode(hash_single(raw_state_transition));

                    tracing::debug!(
                        ?actual_fees,
                        ?estimated_fees,
                        st_hash,
                        "{} state transition ({}) processed and mark as invalid: {}",
                        state_transition_name,
                        st_hash,
                        payment_consensus_error
                    );
                }

                // v1 carries the tracked balance effects (charged fees, adjusted outputs) into the
                // result — the v13 recorded-set expansion. v0 discards them (pre-v13 parity). The
                // success arm's `SuccessfulExecution` recording is the pre-existing behavior gated by
                // `store_address_balances_to_recent_block_storage`, so it is left untouched.
                StateTransitionExecutionResult::PaidConsensusError {
                    error: payment_consensus_error,
                    actual_fees,
                    address_balance_changes: address_balances,
                }
            }
            EventExecutionResult::SuccessfulFreeExecution => {
                if tracing::enabled!(tracing::Level::DEBUG) {
                    let st_hash = hex::encode(hash_single(raw_state_transition));

                    tracing::debug!(
                        st_hash,
                        "Free {} state transition ({}) successfully processed",
                        state_transition_name,
                        st_hash,
                    );
                }

                StateTransitionExecutionResult::SuccessfulExecution {
                    estimated_fees: None,
                    fee_result: FeeResult::default(),
                    address_balance_changes: BTreeMap::new(),
                }
            }
            EventExecutionResult::UnpaidConsensusExecutionError(mut errors) => {
                // TODO: In case of balance is not enough, we need to reduce balance only for processing fees
                //  and return paid consensus error.
                //  Unpaid consensus error should be only if balance not enough even
                //  to cover processing fees
                let first_consensus_error = errors
                    // the first error must be present for an invalid result
                    .remove(0);

                if tracing::enabled!(tracing::Level::DEBUG) {
                    let st_hash = hex::encode(hash_single(raw_state_transition));

                    tracing::debug!(
                        error = ?first_consensus_error,
                        st_hash,
                        "Insufficient identity balance to process {} state transition ({}): {}",
                        state_transition_name,
                        st_hash,
                        first_consensus_error
                    );
                }

                StateTransitionExecutionResult::UnpaidConsensusError(first_consensus_error)
            }
        };

        Ok(state_transition_execution_result)
    }
}
