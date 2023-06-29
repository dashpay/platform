use dpp::state_transition::fee::fee_result::FeeResult;
use crate::drive::identity::update::structs::apply_balance_change_outcome::v0::{ApplyBalanceChangeOutcomeV0, ApplyBalanceChangeOutcomeV0Methods};

mod v0;

/// The outcome of paying for a fee
pub enum ApplyBalanceChangeOutcome {
    V0(ApplyBalanceChangeOutcomeV0)
}

impl ApplyBalanceChangeOutcomeV0Methods for ApplyBalanceChangeOutcome {
    fn actual_fee_paid(&self) -> &FeeResult {
        match self {
            ApplyBalanceChangeOutcome::V0(v0) => &v0.actual_fee_paid,
        }
    }

    fn actual_fee_paid_owned(self) -> FeeResult {
        match self {
            ApplyBalanceChangeOutcome::V0(v0) => v0.actual_fee_paid,
        }
    }

    fn actual_fee_paid_mut(&mut self) -> &mut FeeResult {
        match self {
            ApplyBalanceChangeOutcome::V0(v0) => &mut v0.actual_fee_paid,
        }
    }
}