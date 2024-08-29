pub mod transformer;

use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identifier::Identifier;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

/// document purchase transition action v0
#[derive(Debug, Clone)]
pub struct DocumentPurchaseTransitionActionV0 {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction,
    /// The new document to be inserted
    pub document: Document,
    /// The original owner id
    pub original_owner_id: Identifier,
    /// Price
    pub price: Credits,
}

/// document purchase transition action accessors v0
pub trait DocumentPurchaseTransitionActionAccessorsV0 {
    /// base
    fn base(&self) -> &DocumentBaseTransitionAction;
    /// base owned
    fn base_owned(self) -> DocumentBaseTransitionAction;
    /// the document to be inserted as a ref
    fn document(&self) -> &Document;
    /// the document to be inserted as owned
    fn document_owned(self) -> Document;

    /// The original owner id
    fn original_owner_id(&self) -> Identifier;
    /// Price
    fn price(&self) -> Credits;
}
