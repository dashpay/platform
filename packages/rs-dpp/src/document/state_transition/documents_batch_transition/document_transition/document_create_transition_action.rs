use crate::document::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::document::document_transition::DocumentCreateTransition;
use crate::identity::TimestampMillis;
use platform_value::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct DocumentCreateTransitionAction {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction,
    /// The creation time of the document
    pub created_at: Option<TimestampMillis>,
    //todo: remove updated_at
    /// The time the document was last updated
    pub updated_at: Option<TimestampMillis>,
    /// Document properties
    pub data: BTreeMap<String, Value>,
}

impl From<DocumentCreateTransition> for DocumentCreateTransitionAction {
    fn from(value: DocumentCreateTransition) -> Self {
        let DocumentCreateTransition {
            base,
            created_at,
            updated_at,
            data,
            ..
        } = value;
        DocumentCreateTransitionAction {
            base: base.into(),
            created_at,
            updated_at,
            data: data.unwrap_or_default(),
        }
    }
}

impl From<&DocumentCreateTransition> for DocumentCreateTransitionAction {
    fn from(value: &DocumentCreateTransition) -> Self {
        let DocumentCreateTransition {
            base,
            created_at,
            updated_at,
            data,
            ..
        } = value;
        DocumentCreateTransitionAction {
            base: base.into(),
            created_at: *created_at,
            updated_at: *updated_at,
            data: data.clone().unwrap_or_default(),
        }
    }
}
