use crate::document::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::document::document_transition::DocumentReplaceTransition;
use crate::document::Document;
use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
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
            updated_at: *updated_at,
            data: data.clone().unwrap_or_default(),
        }
    }
}

impl Document {
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionAction` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentReplaceTransitionAction` containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    pub fn try_from_replace_transition(
        value: &DocumentReplaceTransitionAction,
        owner_id: Identifier,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionAction {
            base,
            revision,
            created_at,
            updated_at,
            data,
        } = value;

        let DocumentBaseTransitionAction { id, .. } = base;

        Ok(Document {
            id: *id,
            owner_id,
            properties: data.clone(),
            revision: Some(*revision),
            created_at: *created_at,
            updated_at: *updated_at,
        })
    }

    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionAction` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentReplaceTransitionAction` instance containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    pub fn try_from_owned_replace_transition(
        value: DocumentReplaceTransitionAction,
        owner_id: Identifier,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionAction {
            base,
            revision,
            created_at,
            updated_at,
            data,
        } = value;

        let DocumentBaseTransitionAction { id, .. } = base;

        Ok(Document {
            id,
            owner_id,
            properties: data,
            revision: Some(revision),
            created_at,
            updated_at,
        })
    }
}
