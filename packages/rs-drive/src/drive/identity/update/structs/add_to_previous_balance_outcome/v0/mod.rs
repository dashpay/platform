use dpp::fee::Credits;

/// The outcome of adding to a previous balance
#[derive(Debug)]
pub(in crate::drive::identity::update) struct AddToPreviousBalanceOutcomeV0 {
    /// Is some if the balance was modified
    pub(in crate::drive::identity::update) balance_modified: Option<Credits>,
    /// Is some if the negative credit balance was modified
    pub(in crate::drive::identity::update) negative_credit_balance_modified: Option<Credits>,
}

/// An accessor trait for after balance modification
pub trait AddToPreviousBalanceOutcomeV0Methods {
    /// the modified balance
    fn balance_modified(&self) -> Option<Credits>;
    /// the negative credit balance after modification
    fn negative_credit_balance_modified(&self) -> Option<Credits>;
}
