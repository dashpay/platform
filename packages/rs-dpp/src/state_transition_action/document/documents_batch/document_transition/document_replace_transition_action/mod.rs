mod v0;

use std::collections::BTreeMap;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use platform_value::{Identifier, Value};
pub use v0::*;
use crate::document::Document;
use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use crate::ProtocolError;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::version::PlatformVersion;

#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum DocumentReplaceTransitionAction {
    V0(DocumentReplaceTransitionActionV0),
}

impl DocumentReplaceTransitionActionAccessorsV0 for DocumentReplaceTransitionAction {
    fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.base,
        }
    }

    fn revision(&self) -> Revision {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.revision,
        }
    }

    fn created_at(&self) -> Option<TimestampMillis> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.created_at,
        }
    }

    fn updated_at(&self) -> Option<TimestampMillis> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.updated_at,
        }
    }

    fn data(&self) -> &BTreeMap<String, Value> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => &v0.data,
        }
    }

    fn data_owned(self) -> BTreeMap<String, Value> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.data,
        }
    }
}

impl Document {
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransition` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentReplaceTransition` containing information about the document being replaced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    pub fn try_from_replace_transition(
        document_replace_transition_action: &DocumentReplaceTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_replace_transition_action {
            DocumentReplaceTransitionAction::V0(v0) => {
                Self::try_from_replace_transition_v0(v0, owner_id, platform_version)
            }
        }
    }

    /// Attempts to create a new `Document` from the given `DocumentReplaceTransition` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentReplaceTransition` instance containing information about the document being replaced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    pub fn try_from_owned_replace_transition(
        document_replace_transition_action: DocumentReplaceTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_replace_transition_action {
            DocumentReplaceTransitionAction::V0(v0) => {
                Self::try_from_owned_replace_transition_v0(v0, owner_id, platform_version)
            }
        }
    }
}
