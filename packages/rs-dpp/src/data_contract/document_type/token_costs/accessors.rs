use crate::tokens::token_amount_on_contract_token::DocumentActionTokenCost;

/// Trait providing getters for retrieving token costs associated with different operations.
pub trait TokenCostGettersV0 {
    /// Returns the token cost associated with document creation, if applicable.
    fn document_creation_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns the token cost associated with document replacement (updating), if applicable.
    fn document_replacement_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns the token cost associated with document deletion, if applicable.
    fn document_deletion_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns the token cost associated with document transfer, if applicable.
    fn document_transfer_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns the token cost associated with updating the price of a document, if applicable.
    fn document_price_update_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns the token cost associated with document purchase, if applicable.
    fn document_purchase_token_cost(&self) -> Option<DocumentActionTokenCost>;

    /// Returns a reference to the token cost associated with document creation, if applicable.
    fn document_creation_token_cost_ref(&self) -> Option<&DocumentActionTokenCost>;

    /// Returns a reference to the token cost associated with document replacement (updating), if applicable.
    fn document_replacement_token_cost_ref(&self) -> Option<&DocumentActionTokenCost>;

    /// Returns a reference to the token cost associated with document deletion, if applicable.
    fn document_deletion_token_cost_ref(&self) -> Option<&DocumentActionTokenCost>;

    /// Returns a reference to the token cost associated with document transfer, if applicable.
    fn document_transfer_token_cost_ref(&self) -> Option<&DocumentActionTokenCost>;

    /// Returns a reference to the token cost associated with updating the price of a document, if applicable.
    fn document_price_update_token_cost_ref(&self) -> Option<&DocumentActionTokenCost>;

    /// Returns a reference to the token cost associated with document purchase, if applicable.
    fn document_purchase_token_cost_ref(&self) -> Option<&DocumentActionTokenCost>;
}

/// Trait providing setters for modifying token costs associated with different operations.
pub trait TokenCostSettersV0 {
    /// Sets the token cost for document creation.
    fn set_document_creation_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);

    /// Sets the token cost for document replacement (updating).
    fn set_document_replacement_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);

    /// Sets the token cost for document deletion.
    fn set_document_deletion_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);

    /// Sets the token cost for document transfer.
    fn set_document_transfer_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);

    /// Sets the token cost for updating the price of a document.
    fn set_document_price_update_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);

    /// Sets the token cost for document purchase.
    fn set_document_purchase_token_cost(&mut self, cost: Option<DocumentActionTokenCost>);
}
