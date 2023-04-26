use crate::document::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::document::document_transition::DocumentReplaceTransition;
use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentReplaceTransitionAction {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction,
    /// The current revision we are setting
    pub revision: Revision,
    /// The time the document was last updated
    pub created_at: Option<TimestampMillis>,
    /// The time the document was last updated
    pub updated_at: Option<TimestampMillis>,
    /// Document properties
    pub data: BTreeMap<String, Value>,
}

impl DocumentReplaceTransitionAction {
    pub fn from_document_replace_transition(
        document_replace_transition: &DocumentReplaceTransition,
        originally_created_at: Option<TimestampMillis>,
    ) -> Self {
        let DocumentReplaceTransition {
            base,
            revision,
            updated_at,
            data,
            ..
        } = document_replace_transition;
        DocumentReplaceTransitionAction {
            base: base.into(),
            revision: *revision,
            created_at: originally_created_at,
            updated_at: updated_at.clone(),
            data: data.clone().unwrap_or_default(),
        }
    }
}
