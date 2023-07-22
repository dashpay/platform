use derive_more::From;
use dpp::fee::Credits;

mod v0;

pub use v0::*;

/// The outcome of paying for a fee
#[derive(Debug, From)]
pub(in crate::drive::identity::update) enum AddToPreviousBalanceOutcome {
    V0(AddToPreviousBalanceOutcomeV0),
}

impl AddToPreviousBalanceOutcomeV0Methods for AddToPreviousBalanceOutcome {
    fn balance_modified(&self) -> Option<Credits> {
        match self {
            AddToPreviousBalanceOutcome::V0(v0) => v0.balance_modified,
        }
    }

    fn negative_credit_balance_modified(&self) -> Option<Credits> {
        match self {
            AddToPreviousBalanceOutcome::V0(v0) => v0.negative_credit_balance_modified,
        }
    }
}
