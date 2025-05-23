use crate::data_contract::document_type::token_costs::accessors::{
    TokenCostGettersV0, TokenCostSettersV0,
};
use crate::tokens::token_amount_on_contract_token::DocumentActionTokenCost;

/// Token costs for various document operations.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct TokenCostsV0 {
    /// Cost of creating a document.
    pub create: Option<DocumentActionTokenCost>,

    /// Cost of replacing a document.
    pub replace: Option<DocumentActionTokenCost>,

    /// Cost of deleting a document.
    pub delete: Option<DocumentActionTokenCost>,

    /// Cost of transferring a document.
    pub transfer: Option<DocumentActionTokenCost>,

    /// Cost of updating the price of a document.
    pub update_price: Option<DocumentActionTokenCost>,

    /// Cost of purchasing a document.
    pub purchase: Option<DocumentActionTokenCost>,
}

/// Implementation of the `TokenCostGettersV0` trait for `TokenCostsV0`.
impl TokenCostGettersV0 for TokenCostsV0 {
    fn document_creation_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.create
    }

    fn document_replacement_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.replace
    }

    fn document_deletion_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.delete
    }

    fn document_transfer_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.transfer
    }

    fn document_price_update_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.update_price
    }

    fn document_purchase_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.purchase
    }

    fn document_creation_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        self.create.as_ref()
    }

    fn document_replacement_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        self.replace.as_ref()
    }

    fn document_deletion_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        self.delete.as_ref()
    }

    fn document_transfer_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        self.transfer.as_ref()
    }

    fn document_price_update_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        self.update_price.as_ref()
    }

    fn document_purchase_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        self.purchase.as_ref()
    }
}

/// Implementation of the `TokenCostSettersV0` trait for `TokenCostsV0`.
impl TokenCostSettersV0 for TokenCostsV0 {
    fn set_document_creation_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.create = cost;
    }

    fn set_document_replacement_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.replace = cost;
    }

    fn set_document_deletion_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.delete = cost;
    }

    fn set_document_transfer_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.transfer = cost;
    }

    fn set_document_price_update_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.update_price = cost;
    }

    fn set_document_purchase_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.purchase = cost;
    }
}
