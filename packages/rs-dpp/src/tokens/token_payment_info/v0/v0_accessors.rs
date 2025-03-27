use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use platform_value::Identifier;

/// Trait providing accessor and mutator methods for `TokenPaymentInfoV0`.
pub trait TokenPaymentInfoAccessorsV0 {
    /// Returns a cloned copy of the `payment_token_contract_id` field.
    fn payment_token_contract_id(&self) -> Option<Identifier>;

    /// Returns a reference to the `payment_token_contract_id` field.
    ///
    /// This method avoids copying and can be used when only read access is needed.
    fn payment_token_contract_id_ref(&self) -> &Option<Identifier>;

    /// Returns the `token_contract_position` field.
    fn token_contract_position(&self) -> TokenContractPosition;

    /// Returns a copy of the `minimum_token_cost` field.
    fn minimum_token_cost(&self) -> Option<TokenAmount>;

    /// Returns a copy of the `maximum_token_cost` field.
    fn maximum_token_cost(&self) -> Option<TokenAmount>;

    /// Sets the `payment_token_contract_id` field to the given value.
    fn set_payment_token_contract_id(&mut self, id: Option<Identifier>);

    /// Sets the `token_contract_position` field to the given value.
    fn set_token_contract_position(&mut self, position: TokenContractPosition);

    /// Sets the `minimum_token_cost` field to the given value.
    fn set_minimum_token_cost(&mut self, cost: Option<TokenAmount>);

    /// Sets the `maximum_token_cost` field to the given value.
    fn set_maximum_token_cost(&mut self, cost: Option<TokenAmount>);

    /// Returns the `gas_fees_paid_by` strategy.
    fn gas_fees_paid_by(&self) -> GasFeesPaidBy;

    /// Sets the `gas_fees_paid_by` strategy.
    fn set_gas_fees_paid_by(&mut self, payer: GasFeesPaidBy);
}
