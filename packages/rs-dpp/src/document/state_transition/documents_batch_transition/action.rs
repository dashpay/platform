use crate::document::document_transition::DocumentTransitionAction;
use crate::document::DocumentsBatchTransition;
use crate::identifier::Identifier;
use platform_value::Error;
use serde::{Deserialize, Serialize};

pub const DOCUMENTS_BATCH_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Debug, Clone)]
pub struct DocumentsBatchTransitionAction {
    /// The version of the transition
    pub version: u32,
    /// The owner making the transitions
    pub owner_id: Identifier,
    /// The inner transitions
    pub transitions: Vec<DocumentTransitionAction>,
}

impl From<DocumentsBatchTransition> for DocumentsBatchTransitionAction {
    fn from(value: DocumentsBatchTransition) -> Self {
        let DocumentsBatchTransition {
            owner_id,
            transitions,
            ..
        } = value;
        DocumentsBatchTransitionAction {
            version: DOCUMENTS_BATCH_TRANSITION_ACTION_VERSION,
            owner_id,
            transitions: transitions.into_iter().map(|t| t.into()).collect(),
        }
    }
}

impl From<&DocumentsBatchTransition> for DocumentsBatchTransitionAction {
    fn from(value: &DocumentsBatchTransition) -> Self {
        let DocumentsBatchTransition {
            owner_id,
            transitions,
            ..
        } = value;
        DocumentsBatchTransitionAction {
            version: DOCUMENTS_BATCH_TRANSITION_ACTION_VERSION,
            owner_id: *owner_id,
            transitions: transitions.iter().map(|t| t.into()).collect(),
        }
    }
}
