use crate::document::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::document::document_transition::DocumentReplaceTransition;
use crate::identity::TimestampMillis;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use crate::prelude::Revision;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentReplaceTransitionAction {
    #[serde(flatten)]
    pub base: DocumentBaseTransitionAction,
    #[serde(rename = "$revision")]
    pub revision: Revision,
    #[serde(skip_serializing_if = "Option::is_none", rename = "$updatedAt")]
    pub updated_at: Option<TimestampMillis>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl From<DocumentReplaceTransition> for DocumentReplaceTransitionAction {
    fn from(value: DocumentReplaceTransition) -> Self {
        let DocumentReplaceTransition {
            base,
            revision,
            updated_at,
            data,
            ..
        } = value;
        DocumentReplaceTransitionAction {
            base: base.into(),
            revision,
            updated_at,
            data: data.map(|value| value.into()),
        }
    }
}

impl From<&DocumentReplaceTransition> for DocumentReplaceTransitionAction {
    fn from(value: &DocumentReplaceTransition) -> Self {
        let DocumentReplaceTransition {
            base,
            revision,
            updated_at,
            data,
            ..
        } = value;
        DocumentReplaceTransitionAction {
            base: base.into(),
            revision: *revision,
            updated_at: updated_at.clone(),
            data: data.clone().map(|value| value.into()),
        }
    }
}
