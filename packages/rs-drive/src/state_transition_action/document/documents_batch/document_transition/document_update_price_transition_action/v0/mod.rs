pub mod transformer;

use dpp::document::Document;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

/// document transfer transition action v0
#[derive(Debug, Clone)]
pub struct DocumentUpdatePriceTransitionActionV0 {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction,
    /// The new document to be updated
    pub document: Document,
}

/// document transfer transition action accessors v0
pub trait DocumentUpdatePriceTransitionActionAccessorsV0 {
    /// base
    fn base(&self) -> &DocumentBaseTransitionAction;
    /// base owned
    fn base_owned(self) -> DocumentBaseTransitionAction;
    /// the document with updated price to be reinserted as a ref
    fn document(&self) -> &Document;
    /// the document with updated price to be reinserted as owned
    fn document_owned(self) -> Document;
}
