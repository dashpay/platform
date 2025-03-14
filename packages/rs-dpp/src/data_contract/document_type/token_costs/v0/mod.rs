use crate::balances::credits::TokenAmount;
use crate::data_contract::document_type::token_costs::accessors::{
    TokenCostGettersV0, TokenCostSettersV0,
};
use crate::data_contract::TokenContractPosition;

/// Token costs for various document operations.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct TokenCostsV0 {
    /// Cost of creating a document.
    pub create: Option<(TokenContractPosition, TokenAmount)>,

    /// Cost of replacing a document.
    pub replace: Option<(TokenContractPosition, TokenAmount)>,

    /// Cost of deleting a document.
    pub delete: Option<(TokenContractPosition, TokenAmount)>,

    /// Cost of transferring a document.
    pub transfer: Option<(TokenContractPosition, TokenAmount)>,

    /// Cost of updating the price of a document.
    pub update_price: Option<(TokenContractPosition, TokenAmount)>,

    /// Cost of purchasing a document.
    pub purchase: Option<(TokenContractPosition, TokenAmount)>,
}

/// Implementation of the `TokenCostGettersV0` trait for `TokenCostsV0`.
impl TokenCostGettersV0 for TokenCostsV0 {
    fn document_creation_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)> {
        self.create.clone()
    }

    fn document_replacement_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)> {
        self.replace.clone()
    }

    fn document_deletion_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)> {
        self.delete.clone()
    }

    fn document_transfer_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)> {
        self.transfer.clone()
    }

    fn document_price_update_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)> {
        self.update_price.clone()
    }

    fn document_purchase_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)> {
        self.purchase.clone()
    }
}

/// Implementation of the `TokenCostSettersV0` trait for `TokenCostsV0`.
impl TokenCostSettersV0 for TokenCostsV0 {
    fn set_document_creation_token_cost(
        &mut self,
        cost: Option<(TokenContractPosition, TokenAmount)>,
    ) {
        self.create = cost;
    }

    fn set_document_replacement_token_cost(
        &mut self,
        cost: Option<(TokenContractPosition, TokenAmount)>,
    ) {
        self.replace = cost;
    }

    fn set_document_deletion_token_cost(
        &mut self,
        cost: Option<(TokenContractPosition, TokenAmount)>,
    ) {
        self.delete = cost;
    }

    fn set_document_transfer_token_cost(
        &mut self,
        cost: Option<(TokenContractPosition, TokenAmount)>,
    ) {
        self.transfer = cost;
    }

    fn set_document_price_update_token_cost(
        &mut self,
        cost: Option<(TokenContractPosition, TokenAmount)>,
    ) {
        self.update_price = cost;
    }

    fn set_document_purchase_token_cost(
        &mut self,
        cost: Option<(TokenContractPosition, TokenAmount)>,
    ) {
        self.purchase = cost;
    }
}
