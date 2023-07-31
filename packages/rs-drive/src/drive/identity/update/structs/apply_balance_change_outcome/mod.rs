use derive_more::From;
use dpp::fee::fee_result::FeeResult;

mod v0;
pub use v0::*;

/// The outcome of paying for a fee
#[derive(Debug, From)]
pub enum ApplyBalanceChangeOutcome {
    /// Version 0
    V0(ApplyBalanceChangeOutcomeV0),
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
