use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;

/// Trait providing getters for retrieving token costs associated with different document operations.
pub trait DocumentTypeV1Getters {
    /// Returns the token cost associated with document creation, if applicable.
    ///
    /// # Returns
    /// - `Some((TokenContractPosition, TokenAmount))` if a creation cost exists.
    /// - `None` if no cost is set for document creation.
    fn document_creation_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)>;

    /// Returns the token cost associated with document replacement (updating), if applicable.
    ///
    /// # Returns
    /// - `Some((TokenContractPosition, TokenAmount))` if a replacement cost exists.
    /// - `None` if no cost is set for document replacement.
    fn document_replacement_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)>;

    /// Returns the token cost associated with document deletion, if applicable.
    ///
    /// # Returns
    /// - `Some((TokenContractPosition, TokenAmount))` if a deletion cost exists.
    /// - `None` if no cost is set for document deletion.
    fn document_deletion_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)>;

    /// Returns the token cost associated with document transfer, if applicable.
    ///
    /// # Returns
    /// - `Some((TokenContractPosition, TokenAmount))` if a transfer cost exists.
    /// - `None` if no cost is set for document transfer.
    fn document_transfer_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)>;

    /// Returns the token cost associated with updating the price of a document, if applicable.
    ///
    /// # Returns
    /// - `Some((TokenContractPosition, TokenAmount))` if an update price cost exists.
    /// - `None` if no cost is set for updating the price.
    fn document_update_price_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)>;

    /// Returns the token cost associated with document purchase, if applicable.
    ///
    /// # Returns
    /// - `Some((TokenContractPosition, TokenAmount))` if a purchase cost exists.
    /// - `None` if no cost is set for document purchase.
    fn document_purchase_token_cost(&self) -> Option<(TokenContractPosition, TokenAmount)>;
}
