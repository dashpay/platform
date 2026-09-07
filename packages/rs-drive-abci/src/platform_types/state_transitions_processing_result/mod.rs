use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use std::collections::BTreeMap;

use crate::error::Error;
use crate::platform_types::event_execution_result::EstimatedFeeResult;
use dpp::fee::fee_result::FeeResult;

/// The reason the state transition was not executed
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotExecutedReason {
    /// The proposer ran out of time
    ProposerRanOutOfTime,
}

/// State Transition Execution Result represents a result of the single state transition execution.
/// There are five possible outcomes of the state transition execution described by this enum
#[derive(Debug, Clone, PartialEq)]
pub enum StateTransitionExecutionResult {
    /// State Transition is invalid, but we have a proved identity associated with it,
    /// and we can deduct processing fees calculated until this validation error happened
    PaidConsensusError {
        /// The consensus error that occurred
        error: ConsensusError,
        /// Actual fees charged
        actual_fees: FeeResult,
        /// Address balance changes applied by this paid-INVALID transition. Non-empty only for an
        /// APPLIED chargeable-failure transition that still credits a transparent address — e.g. the
        /// `IdentityCreateFromShieldedPool` duplicate-key fallback, which finalizes as an
        /// `UnshieldAction` (paid-invalid) yet credits the fallback address the net unshielded
        /// amount. These must reach `address_balances_updated` so incremental sync sees the GroveDB
        /// credit the block actually applied. Empty for ordinary paid-invalid transitions (bump-only
        /// penalties that credit no address).
        address_balance_changes: BTreeMap<PlatformAddress, CreditOperation>,
    },
    /// State Transition is invalid, and is not paid for because we either :
    ///     * don't have a proved identity associated with it so we can't deduct balance.
    ///     * the state transition revision causes this transaction to not be valid
    /// These state transitions can appear in prepare proposal but must never appear in process
    /// proposal.
    UnpaidConsensusError(ConsensusError),
    /// State Transition execution failed due to the internal drive-abci error
    InternalError(String),
    /// State Transition was successfully executed
    SuccessfulExecution {
        /// Estimated fees (if available)
        estimated_fees: Option<EstimatedFeeResult>,
        /// Actual fees charged
        fee_result: FeeResult,
        /// Address balance changes from this state transition
        address_balance_changes: BTreeMap<PlatformAddress, CreditOperation>,
    },
    /// State Transition was not executed at all.
    /// The only current reason for this is that the proposer reached the maximum time limit
    NotExecuted(NotExecutedReason),
}

/// State Transitions Processing Result produced by [process_raw_state_transitions] and represents
/// a result of a batch state transitions execution. It contains [StateTransitionExecutionResult] for
/// each state transition and aggregated fees.
#[derive(Debug, Default, Clone)]
pub struct StateTransitionsProcessingResult {
    execution_results: Vec<StateTransitionExecutionResult>,
    pub(crate) address_balances_updated: BTreeMap<PlatformAddress, CreditOperation>,
    invalid_paid_count: usize,
    invalid_unpaid_count: usize,
    valid_count: usize,
    failed_count: usize,
    fees: FeeResult,
    credit_mints: Credits,
}

impl StateTransitionsProcessingResult {
    /// Add address balances, combining operations according to these rules:
    /// - Set + Set = second Set wins
    /// - Set + Add = Set to combined value
    /// - Add + Add = saturating add
    /// - Add + Set = Set (discard the Add)
    pub fn add_address_balances_in_update(
        &mut self,
        address_balances: BTreeMap<PlatformAddress, CreditOperation>,
    ) {
        for (address, new_op) in address_balances {
            self.address_balances_updated
                .entry(address)
                .and_modify(|existing| {
                    *existing = match (&existing, &new_op) {
                        // Set + Set = second Set wins
                        (CreditOperation::SetCredits(_), CreditOperation::SetCredits(new_val)) => {
                            CreditOperation::SetCredits(*new_val)
                        }
                        // Set + Add = Set to combined value
                        (
                            CreditOperation::SetCredits(set_val),
                            CreditOperation::AddToCredits(add_val),
                        ) => CreditOperation::SetCredits(set_val.saturating_add(*add_val)),
                        // Add + Add = saturating add
                        (
                            CreditOperation::AddToCredits(add1),
                            CreditOperation::AddToCredits(add2),
                        ) => CreditOperation::AddToCredits(add1.saturating_add(*add2)),
                        // Add + Set = Set (discard the Add)
                        (
                            CreditOperation::AddToCredits(_),
                            CreditOperation::SetCredits(set_val),
                        ) => CreditOperation::SetCredits(*set_val),
                    };
                })
                .or_insert(new_op);
        }
    }
    /// Add a new execution result
    pub fn add(&mut self, execution_result: StateTransitionExecutionResult) -> Result<(), Error> {
        match &execution_result {
            StateTransitionExecutionResult::InternalError(_) => {
                self.failed_count += 1;
            }
            StateTransitionExecutionResult::PaidConsensusError {
                actual_fees,
                address_balance_changes,
                ..
            } => {
                self.invalid_paid_count += 1;
                self.fees.checked_add_assign(actual_fees.clone())?;

                // An applied chargeable-failure transition (e.g. the identity-create-from-shielded-
                // pool fallback) can credit a transparent address even though it is paid-invalid;
                // merge those credits so incremental sync sees them. Ordinary paid-invalid
                // transitions carry an empty map, so this is a no-op for them.
                self.add_address_balances_in_update(address_balance_changes.clone());
            }
            StateTransitionExecutionResult::UnpaidConsensusError(_) => {
                self.invalid_unpaid_count += 1;
            }
            StateTransitionExecutionResult::SuccessfulExecution {
                fee_result: actual_fees,
                address_balance_changes,
                ..
            } => {
                self.valid_count += 1;

                self.fees.checked_add_assign(actual_fees.clone())?;

                // Merge address balance changes
                self.add_address_balances_in_update(address_balance_changes.clone());
            }
            StateTransitionExecutionResult::NotExecuted(_) => {
                self.failed_count += 1;
            }
        }

        self.execution_results.push(execution_result);

        Ok(())
    }

    /// Returns the number of paid invalid state transitions
    pub fn invalid_paid_count(&self) -> usize {
        self.invalid_paid_count
    }

    /// Returns the number of unpaid invalid state transitions
    pub fn invalid_unpaid_count(&self) -> usize {
        self.invalid_unpaid_count
    }

    /// Returns the number of valid state transitions
    pub fn valid_count(&self) -> usize {
        self.valid_count
    }

    /// Returns the number of failed state transitions
    pub fn failed_count(&self) -> usize {
        self.failed_count
    }

    /// Sets the credits the block's applied state transitions minted into Platform
    pub fn set_credit_mints(&mut self, credit_mints: Credits) {
        self.credit_mints = credit_mints;
    }

    /// Returns the credits the block's applied state transitions minted into Platform
    /// (their `AddToSystemCredits` operations): the state-transition share of the block's
    /// credit inflow, recorded for the net daily withdrawal limit.
    pub fn credit_mints(&self) -> Credits {
        self.credit_mints
    }

    /// Returns the aggregated fees
    pub fn aggregated_fees(&self) -> &FeeResult {
        &self.fees
    }

    /// Transform into the state transition execution results
    pub fn into_execution_results(self) -> Vec<StateTransitionExecutionResult> {
        self.execution_results
    }

    /// State Transition execution results
    pub fn execution_results(&self) -> &Vec<StateTransitionExecutionResult> {
        &self.execution_results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `PaidConsensusError` carrying address credits (the applied chargeable-failure fallback,
    /// e.g. IdentityCreateFromShieldedPool's duplicate-key path) must merge those credits into
    /// `address_balances_updated`, exactly like `SuccessfulExecution` — otherwise a credit the block
    /// applied never reaches the recent-address-balance-changes tree that incremental sync reads.
    #[test]
    fn add_merges_paid_consensus_error_address_changes() {
        let mut result = StateTransitionsProcessingResult::default();
        let address = PlatformAddress::P2pkh([0xCD; 20]);

        result
            .add(StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::DefaultError,
                actual_fees: FeeResult::default(),
                address_balance_changes: BTreeMap::from([(
                    address,
                    CreditOperation::AddToCredits(2500),
                )]),
            })
            .expect("add should succeed");

        assert_eq!(result.invalid_paid_count(), 1);
        assert_eq!(
            result.address_balances_updated.get(&address),
            Some(&CreditOperation::AddToCredits(2500)),
            "PaidConsensusError credits must reach address_balances_updated"
        );
    }

    /// An ordinary `PaidConsensusError` (bump-only penalty, no credit) must leave
    /// `address_balances_updated` empty.
    #[test]
    fn add_paid_consensus_error_without_changes_leaves_balances_empty() {
        let mut result = StateTransitionsProcessingResult::default();

        result
            .add(StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::DefaultError,
                actual_fees: FeeResult::default(),
                address_balance_changes: BTreeMap::new(),
            })
            .expect("add should succeed");

        assert_eq!(result.invalid_paid_count(), 1);
        assert!(result.address_balances_updated.is_empty());
    }
}
