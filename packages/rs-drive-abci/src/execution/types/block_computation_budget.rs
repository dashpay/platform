//! The per-block ledger of smart-contract computation.
//!
//! `SystemLimits::smart_contract_computation` bounds how many computation units all contract
//! invocations in one block may consume together, ordinary and scheduled. This ledger is the
//! deterministic accounting both proposal creation and proposal validation run over the same
//! sequence of invocations: reserve an invocation's admitted bound before it runs, settle what
//! it actually consumed afterwards, release the rest. Reserving the bound rather than the actual
//! consumption means an invocation that would not fit is never started, so per-block exhaustion
//! is an admission outcome (the proposer delays the transition, a validator rejects the block)
//! and never a paid failure part-way through execution, and an invocation's own outcome does not
//! depend on its position in the block.
//!
//! The ledger is in-memory block state and is never serialised; it has no version wrapper.

use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::fee::smart_contract_computation::ComputationUnits;
use dpp::version::PlatformVersion;

/// The admitted computation bound of one contract invocation, held against the block's budget
/// until it is settled.
///
/// Not `Clone` and consumed by [`BlockComputationBudget::settle`], so a reservation can only be
/// spent once. Dropping it without settling keeps its bound held for the rest of the block; a
/// caller that abandons an invocation before it runs settles with zero consumption instead.
#[derive(Debug, PartialEq, Eq)]
#[must_use = "an unsettled reservation keeps its bound held for the rest of the block"]
pub struct ComputationReservation {
    bound: ComputationUnits,
}

impl ComputationReservation {
    /// The computation units held by this reservation.
    pub fn bound(&self) -> ComputationUnits {
        self.bound
    }
}

/// Why a reservation was refused: the block does not have `requested` units left, only
/// `remaining`. An admission outcome, not an error: the ledger is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockComputationBudgetExceeded {
    /// The bound the invocation asked to reserve.
    pub requested: ComputationUnits,
    /// The units the block still had available.
    pub remaining: ComputationUnits,
}

/// The computation units one block may still hand to contract invocations, and the units it has
/// already handed out.
///
/// Invariant after every operation: `consumed + held + remaining == limit`, where `held` is the
/// sum of the outstanding reservations. Every operation uses checked arithmetic; the invariant
/// keeps each intermediate value within `limit`, but the checks make that proof local to each
/// method rather than something a reader has to carry across the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockComputationBudget {
    limit: ComputationUnits,
    /// Units settled as actually consumed.
    consumed: ComputationUnits,
    /// Units still available to reserve: the limit minus consumed and held units.
    remaining: ComputationUnits,
}

impl BlockComputationBudget {
    /// The ledger for a block executed under `platform_version`, or `None` when the version
    /// predates smart contracts so that callers skip the contract path entirely.
    pub fn for_platform_version(platform_version: &PlatformVersion) -> Option<Self> {
        platform_version
            .system_limits
            .smart_contract_computation
            .as_ref()
            .map(|limits| Self::with_limit(limits.max_computation_units_per_block))
    }

    /// A ledger over an explicit limit.
    pub fn with_limit(limit: ComputationUnits) -> Self {
        Self {
            limit,
            consumed: 0,
            remaining: limit,
        }
    }

    /// The per-block limit this ledger enforces.
    pub fn limit(&self) -> ComputationUnits {
        self.limit
    }

    /// Units settled as actually consumed so far.
    pub fn consumed(&self) -> ComputationUnits {
        self.consumed
    }

    /// Units still available to reserve. Outstanding reservations are not available.
    pub fn remaining(&self) -> ComputationUnits {
        self.remaining
    }

    /// Holds `bound` units for one invocation.
    ///
    /// Refused, with the ledger unchanged, when fewer than `bound` units remain. Reserving zero
    /// units is allowed and holds nothing.
    pub fn reserve(
        &mut self,
        bound: ComputationUnits,
    ) -> Result<ComputationReservation, BlockComputationBudgetExceeded> {
        match self.remaining.checked_sub(bound) {
            Some(remaining) => {
                self.remaining = remaining;
                Ok(ComputationReservation { bound })
            }
            None => Err(BlockComputationBudgetExceeded {
                requested: bound,
                remaining: self.remaining,
            }),
        }
    }

    /// Settles a reservation with the units the invocation actually consumed: the consumed
    /// units are recorded and the unused part of the bound is returned to the block. Returns
    /// the released units.
    ///
    /// `actual` above the reserved bound is `ExecutionError::CorruptedCodeExecution`: the
    /// runtime is handed the bound as its budget and cannot legally exceed it, so this is a
    /// broken runtime, not an admission outcome. The ledger is unchanged in that case.
    pub fn settle(
        &mut self,
        reservation: ComputationReservation,
        actual: ComputationUnits,
    ) -> Result<ComputationUnits, Error> {
        let released = reservation
            .bound
            .checked_sub(actual)
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "a contract invocation consumed more computation than its reserved bound",
            )))?;

        let consumed =
            self.consumed
                .checked_add(actual)
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "consumed smart-contract computation overflowed the block ledger",
                )))?;
        let remaining = self
            .remaining
            .checked_add(released)
            .ok_or(Error::Execution(ExecutionError::Overflow(
                "released smart-contract computation overflowed the block ledger",
            )))?;

        self.consumed = consumed;
        self.remaining = remaining;

        Ok(released)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::version::PlatformVersion;

    fn ledger(limit: ComputationUnits) -> BlockComputationBudget {
        BlockComputationBudget::with_limit(limit)
    }

    #[test]
    fn should_not_exist_before_the_5_0_protocol_version() {
        let platform_version_14 = PlatformVersion::get(14).expect("protocol version 14 exists");
        assert_eq!(
            BlockComputationBudget::for_platform_version(platform_version_14),
            None,
            "protocol version 14 predates smart contracts and must have no computation ledger"
        );

        let latest = PlatformVersion::latest();
        let limit = latest
            .system_limits
            .smart_contract_computation
            .as_ref()
            .expect("the latest protocol version bounds smart-contract computation")
            .max_computation_units_per_block;
        let budget = BlockComputationBudget::for_platform_version(latest)
            .expect("the latest protocol version must have a computation ledger");

        assert_eq!(budget.limit(), limit);
        assert_eq!(budget.remaining(), limit);
        assert_eq!(budget.consumed(), 0);
    }

    #[test]
    fn should_reserve_up_to_the_exact_remaining_budget() {
        let mut budget = ledger(1_000);

        let exact = budget
            .reserve(1_000)
            .expect("a bound equal to the remaining budget must fit");
        assert_eq!(exact.bound(), 1_000);
        assert_eq!(budget.remaining(), 0);

        let refused = budget
            .reserve(1)
            .expect_err("one unit more than the remaining budget must be refused");
        assert_eq!(
            refused,
            BlockComputationBudgetExceeded {
                requested: 1,
                remaining: 0,
            }
        );
        assert_eq!(
            budget,
            BlockComputationBudget {
                limit: 1_000,
                consumed: 0,
                remaining: 0,
            },
            "a refused reservation must leave the ledger unchanged"
        );

        let released = budget
            .settle(exact, 1_000)
            .expect("consuming the whole bound is legal");
        assert_eq!(released, 0);
        assert_eq!(budget.consumed(), 1_000);
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn should_settle_actual_consumption_and_release_the_unused_bound() {
        let mut budget = ledger(10_000);

        let reservation = budget.reserve(1_000).expect("1,000 units fit in 10,000");
        assert_eq!(budget.remaining(), 9_000);

        let released = budget
            .settle(reservation, 600)
            .expect("consuming less than the bound is legal");

        assert_eq!(released, 400);
        assert_eq!(budget.consumed(), 600);
        assert_eq!(budget.remaining(), 9_400);
    }

    #[test]
    fn should_reject_settling_more_than_the_reserved_bound() {
        let mut budget = ledger(10_000);
        let reservation = budget.reserve(1_000).expect("1,000 units fit in 10,000");

        let result = budget.settle(reservation, 1_001);

        assert!(
            matches!(
                result,
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(_)))
            ),
            "expected a corrupted code execution error, got {result:?}"
        );
        assert_eq!(
            budget,
            BlockComputationBudget {
                limit: 10_000,
                consumed: 0,
                remaining: 9_000,
            },
            "a rejected settlement must leave the ledger unchanged, with the bound still held"
        );
    }

    #[test]
    fn should_account_many_reservations_without_double_spending() {
        let limit = 1_000;
        let mut budget = ledger(limit);
        let mut expected_consumed = 0;

        for (bound, actual) in [(300, 300), (300, 100), (200, 0), (250, 250), (150, 50)] {
            let held_before = limit - budget.consumed() - budget.remaining();
            assert_eq!(
                held_before, 0,
                "nothing is held between settled invocations"
            );

            let reservation = budget.reserve(bound).expect("each bound fits");
            assert_eq!(
                budget.consumed() + reservation.bound() + budget.remaining(),
                limit,
                "consumed + held + remaining must equal the limit while a reservation is out"
            );

            let released = budget.settle(reservation, actual).expect("actual <= bound");
            expected_consumed += actual;

            assert_eq!(released, bound - actual);
            assert_eq!(budget.consumed(), expected_consumed);
            assert_eq!(
                budget.consumed() + budget.remaining(),
                limit,
                "consumed + remaining must equal the limit after settlement"
            );
        }

        // 300 + 100 + 0 + 250 + 50 = 700 consumed; 300 units remain, so a 301-unit bound is
        // refused even though the sum of the bounds reserved so far (1,200) exceeded the limit:
        // released units are available again, spent units are not.
        assert_eq!(budget.consumed(), 700);
        assert_eq!(budget.remaining(), 300);
        let refused = budget
            .reserve(301)
            .expect_err("301 units exceed the 300 remaining");
        assert_eq!(refused.remaining, 300);
        // A reservation moved into `settle` cannot be settled again: the type system enforces
        // it, which is why `ComputationReservation` is neither `Clone` nor `Copy`.
    }

    #[test]
    fn should_keep_the_budget_of_an_unsettled_reservation_held() {
        let mut budget = ledger(1_000);

        let outstanding = budget.reserve(600).expect("600 units fit in 1,000");
        assert_eq!(budget.remaining(), 400);

        let refused = budget
            .reserve(401)
            .expect_err("an outstanding reservation keeps its bound unavailable");
        assert_eq!(refused.remaining, 400);

        // Abandoning an invocation before it runs is settled with zero consumption, which
        // returns the whole bound; dropping the reservation instead would keep the 600 held.
        let released = budget
            .settle(outstanding, 0)
            .expect("zero consumption is legal");
        assert_eq!(released, 600);
        assert_eq!(budget.consumed(), 0);
        assert_eq!(budget.remaining(), 1_000);
    }

    #[test]
    fn should_use_checked_arithmetic_at_the_top_of_the_range() {
        let mut budget = ledger(ComputationUnits::MAX);

        let whole = budget
            .reserve(ComputationUnits::MAX)
            .expect("the whole budget can be reserved at once");
        assert_eq!(budget.remaining(), 0);
        let refused = budget
            .reserve(1)
            .expect_err("nothing remains once the whole budget is held");
        assert_eq!(refused.remaining, 0);

        let released = budget
            .settle(whole, ComputationUnits::MAX)
            .expect("consuming the whole budget is legal");
        assert_eq!(released, 0);
        assert_eq!(budget.consumed(), ComputationUnits::MAX);
        assert_eq!(budget.remaining(), 0);

        // Reserving zero still succeeds with nothing left, and settling it changes nothing.
        let empty = budget.reserve(0).expect("a zero bound always fits");
        let released = budget.settle(empty, 0).expect("zero consumption is legal");
        assert_eq!(released, 0);
        assert_eq!(budget.consumed(), ComputationUnits::MAX);
        assert_eq!(budget.remaining(), 0);
    }
}
