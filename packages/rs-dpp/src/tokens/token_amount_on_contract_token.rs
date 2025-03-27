use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use platform_value::Identifier;

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct DocumentActionTokenCost {
    /// If this is not set, it means that we are using our own contract id
    pub contract_id: Option<Identifier>,
    /// Token contract position
    pub token_contract_position: TokenContractPosition,
    /// The amount
    pub token_amount: TokenAmount,
    /// Who is paying for gas fees for this action
    pub gas_fees_paid_by: GasFeesPaidBy,
}
