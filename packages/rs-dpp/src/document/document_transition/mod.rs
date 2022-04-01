mod document_base_transition;
pub use document_base_transition::*;

mod document_create_transition;
pub use document_create_transition::*;

mod document_delete_transition;
pub use document_delete_transition::*;

mod document_replace_transition;
pub use document_replace_transition::*;

#[derive(Debug, Clone)]
pub enum DocumentTransition {
    Create(DocumentCreateTransition),
    Replace(DocumentReplaceTransition),
    Delete(DocumentDeleteTransition),
}
