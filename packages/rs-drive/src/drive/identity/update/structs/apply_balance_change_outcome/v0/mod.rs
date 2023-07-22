use dpp::fee::fee_result::FeeResult;

/// The outcome of paying for a fee
#[derive(Debug)]
pub(in crate::drive::identity::update) struct ApplyBalanceChangeOutcomeV0 {
    /// The actual fee paid by the identity
    pub actual_fee_paid: FeeResult,
}

pub trait ApplyBalanceChangeOutcomeV0Methods {
    fn actual_fee_paid(&self) -> &FeeResult;
    fn actual_fee_paid_owned(self) -> FeeResult;
    fn actual_fee_paid_mut(&mut self) -> &mut FeeResult;
}
