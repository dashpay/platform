use crate::document::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::document::document_transition::DocumentReplaceTransition;
use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct DocumentReplaceTransitionAction {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction,
    /// The current revision we are setting
    pub revision: Revision,
    //todo: remove updated_at
    /// The time the document was last updated
    pub updated_at: Option<TimestampMillis>,
    /// Document properties
    pub data: BTreeMap<String, Value>,
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
            data: data.unwrap_or_default(),
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
            data: data.clone().unwrap_or_default(),
        }
    }
}
