use dpp::fee::fee_result::FeeResult;

/// The outcome of paying for a fee
#[derive(Debug)]
pub struct ApplyBalanceChangeOutcomeV0 {
    /// The actual fee paid by the identity
    pub actual_fee_paid: FeeResult,
}

/// Trait for interacting with balance change outcomes.
pub trait ApplyBalanceChangeOutcomeV0Methods {
    /// Returns an immutable reference to the actual fee paid.
    fn actual_fee_paid(&self) -> &FeeResult;

    /// Consumes and returns the actual fee paid.
    fn actual_fee_paid_owned(self) -> FeeResult;

    /// Returns a mutable reference to the actual fee paid.
    fn actual_fee_paid_mut(&mut self) -> &mut FeeResult;
}
