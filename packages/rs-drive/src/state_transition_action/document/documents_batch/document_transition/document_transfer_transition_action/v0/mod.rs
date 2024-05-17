pub mod transformer;

use dpp::document::Document;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

/// document transfer transition action v0
#[derive(Debug, Clone)]
pub struct DocumentTransferTransitionActionV0 {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction,
    /// The new document to be inserted
    pub document: Document,
}

/// document transfer transition action accessors v0
pub trait DocumentTransferTransitionActionAccessorsV0 {
    /// base
    fn base(&self) -> &DocumentBaseTransitionAction;
    /// base owned
    fn base_owned(self) -> DocumentBaseTransitionAction;
    /// the document to be inserted as a ref
    fn document(&self) -> &Document;
    /// the document to be inserted as owned
    fn document_owned(self) -> Document;
}
