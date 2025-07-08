use crate::data_contract::TokenContractPosition;
use crate::tokens::token_amount_on_contract_token::DocumentActionTokenCost;
use platform_value::Identifier;
use std::collections::{BTreeMap, BTreeSet};

/// Trait providing getters for retrieving token costs associated with different document operations.
pub trait DocumentTypeV1Getters {
    /// Returns the token cost associated with document creation, if applicable.
    ///
    /// # Returns
    /// - `Some(TokenActionCost)` if a creation cost exists.
    /// - `None` if no cost is set for document creation.
    fn document_creation_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns the token cost associated with document replacement (updating), if applicable.
    ///
    /// # Returns
    /// - `Some(TokenActionCost)` if a replacement cost exists.
    /// - `None` if no cost is set for document replacement.
    fn document_replacement_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns the token cost associated with document deletion, if applicable.
    ///
    /// # Returns
    /// - `Some(TokenActionCost)` if a deletion cost exists.
    /// - `None` if no cost is set for document deletion.
    fn document_deletion_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns the token cost associated with document transfer, if applicable.
    ///
    /// # Returns
    /// - `Some(TokenActionCost)` if a transfer cost exists.
    /// - `None` if no cost is set for document transfer.
    fn document_transfer_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns the token cost associated with updating the price of a document, if applicable.
    ///
    /// # Returns
    /// - `Some(TokenActionCost)` if an update price cost exists.
    /// - `None` if no cost is set for updating the price.
    fn document_update_price_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns the token cost associated with document purchase, if applicable.
    ///
    /// # Returns
    /// - `Some(TokenActionCost)` if a purchase cost exists.
    /// - `None` if no cost is set for document purchase.
    fn document_purchase_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns all document token costs. This is generally used only in internal validation.
    fn all_document_token_costs(&self) -> Vec<&DocumentActionTokenCost>;

    /// Returns the tokens used by external token costs as a set of token contract positions per contract.
    /// This is generally used only in internal validation.
    fn all_external_token_costs_contract_tokens(
        &self,
    ) -> BTreeMap<Identifier, BTreeSet<TokenContractPosition>>;
}

/// Trait providing setters for assigning token costs to different document operations.
pub trait DocumentTypeV1Setters {
    /// Sets the token cost for document creation.
    ///
    /// # Arguments
    /// - `cost`: `Some(DocumentActionTokenCost)` to set a cost, or `None` to clear it.
    fn set_document_creation_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);

    /// Sets the token cost for document replacement (updating).
    ///
    /// # Arguments
    /// - `cost`: `Some(DocumentActionTokenCost)` to set a cost, or `None` to clear it.
    fn set_document_replacement_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);

    /// Sets the token cost for document deletion.
    ///
    /// # Arguments
    /// - `cost`: `Some(DocumentActionTokenCost)` to set a cost, or `None` to clear it.
    fn set_document_deletion_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);

    /// Sets the token cost for document transfer.
    ///
    /// # Arguments
    /// - `cost`: `Some(DocumentActionTokenCost)` to set a cost, or `None` to clear it.
    fn set_document_transfer_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);

    /// Sets the token cost for updating the price of a document.
    ///
    /// # Arguments
    /// - `cost`: `Some(DocumentActionTokenCost)` to set a cost, or `None` to clear it.
    fn set_document_price_update_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);

    /// Sets the token cost for document purchase.
    ///
    /// # Arguments
    /// - `cost`: `Some(DocumentActionTokenCost)` to set a cost, or `None` to clear it.
    fn set_document_purchase_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);
}
