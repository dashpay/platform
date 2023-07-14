use dpp::fee::Credits;

/// The outcome of adding to a previous balance
pub(in crate::drive::identity::update) struct AddToPreviousBalanceOutcomeV0 {
    /// Is some if the balance was modified
    pub(super) balance_modified: Option<Credits>,
    /// Is some if the negative credit balance was modified
    pub(super) negative_credit_balance_modified: Option<Credits>,
}

pub trait AddToPreviousBalanceOutcomeV0Methods {
    fn balance_modified(&self) -> Option<Credits>;
    fn negative_credit_balance_modified(&self) -> Option<Credits>;
}
