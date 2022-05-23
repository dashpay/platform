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

impl DocumentTransition {
    pub fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentTransition::Create(d) => &d.base,
            DocumentTransition::Delete(d) => &d.base,
            DocumentTransition::Replace(d) => &d.base,
        }
    }

    pub fn as_transition_create(&self) -> Option<&DocumentCreateTransition> {
        if let Self::Create(ref t) = self {
            Some(t)
        } else {
            None
        }
    }
    pub fn as_transition_replace(&self) -> Option<&DocumentReplaceTransition> {
        if let Self::Replace(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    pub fn as_transition_delete(&self) -> Option<&DocumentDeleteTransition> {
        if let Self::Delete(ref t) = self {
            Some(t)
        } else {
            None
        }
    }
}
