use crate::document::document_transition::DocumentTransitionAction;
use crate::document::DocumentsBatchTransition;
use crate::identifier::Identifier;
use serde::{Deserialize, Serialize};

pub const DOCUMENTS_BATCH_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentsBatchTransitionAction {
    pub version: u32,
    pub owner_id: Identifier,
    // we want to skip serialization of transitions, as we does it manually in `to_object()`  and `to_json()`
    #[serde(skip_serializing)]
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
