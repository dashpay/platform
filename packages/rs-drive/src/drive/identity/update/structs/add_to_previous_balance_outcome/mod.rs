use dpp::state_transition::fee::Credits;
use crate::drive::identity::update::structs::add_to_previous_balance_outcome::v0::{AddToPreviousBalanceOutcomeV0, AddToPreviousBalanceOutcomeV0Methods};

mod v0;

/// The outcome of paying for a fee
pub(in crate::drive::identity::update) enum AddToPreviousBalanceOutcome {
    V0(AddToPreviousBalanceOutcomeV0)
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