use crate::document::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::document::document_transition::DocumentCreateTransition;
use crate::identity::TimestampMillis;
use platform_value::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentCreateTransitionAction {
    /// Document Base Transition
    #[serde(flatten)]
    pub base: DocumentBaseTransitionAction,

    #[serde(rename = "$createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<TimestampMillis>,
    #[serde(rename = "$updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<TimestampMillis>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
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
            data: data.map(|value| value.into()),
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
            created_at: created_at.clone(),
            updated_at: updated_at.clone(),
            data: data.clone().map(|value| value.into()),
        }
    }
}
