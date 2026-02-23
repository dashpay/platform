use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::consensus::ConsensusError;
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
        /// Nullifiers inserted by shielded spend actions
        nullifiers_inserted: Vec<[u8; 32]>,
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
    pub(crate) nullifiers_inserted: Vec<[u8; 32]>,
    invalid_paid_count: usize,
    invalid_unpaid_count: usize,
    valid_count: usize,
    failed_count: usize,
    fees: FeeResult,
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
            StateTransitionExecutionResult::PaidConsensusError { actual_fees, .. } => {
                self.invalid_paid_count += 1;
                self.fees.checked_add_assign(actual_fees.clone())?;
            }
            StateTransitionExecutionResult::UnpaidConsensusError(_) => {
                self.invalid_unpaid_count += 1;
            }
            StateTransitionExecutionResult::SuccessfulExecution {
                fee_result: actual_fees,
                address_balance_changes,
                nullifiers_inserted,
                ..
            } => {
                self.valid_count += 1;

                self.fees.checked_add_assign(actual_fees.clone())?;

                // Merge address balance changes
                self.add_address_balances_in_update(address_balance_changes.clone());

                // Collect nullifiers from shielded spend actions
                if !nullifiers_inserted.is_empty() {
                    self.nullifiers_inserted.extend(nullifiers_inserted);
                }
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

    /// Add nullifiers that were inserted during state transition processing
    pub fn add_nullifiers_inserted(&mut self, nullifiers: Vec<[u8; 32]>) {
        self.nullifiers_inserted.extend(nullifiers);
    }
}
