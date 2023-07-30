#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

use crate::document::{Document, DocumentV0};
use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use bincode::{Decode, Encode};
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::version::PlatformVersion;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct DocumentReplaceTransitionActionV0<'a> {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction<'a>,
    /// The current revision we are setting
    pub revision: Revision,
    /// The time the document was last updated
    pub created_at: Option<TimestampMillis>,
    /// The time the document was last updated
    pub updated_at: Option<TimestampMillis>,
    /// Document properties
    pub data: BTreeMap<String, Value>,
}

pub trait DocumentReplaceTransitionActionAccessorsV0<'a> {
    fn base(&self) -> &DocumentBaseTransitionAction;
    fn base_owned(self) -> DocumentBaseTransitionAction<'a>;
    fn revision(&self) -> Revision;
    fn created_at(&self) -> Option<TimestampMillis>;
    fn updated_at(&self) -> Option<TimestampMillis>;
    fn data(&self) -> &BTreeMap<String, Value>;
    fn data_owned(self) -> BTreeMap<String, Value>;
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
    pub(super) fn try_from_replace_transition_v0(
        value: &DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionActionV0 {
            base,
            revision,
            created_at,
            updated_at,
            data,
        } = value;

        let id = base.id();

        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0 {
                id,
                owner_id,
                properties: data.clone(),
                revision: Some(*revision),
                created_at: created_at.clone(),
                updated_at: updated_at.clone(),
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::try_from_replace_transition".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
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
    pub(super) fn try_from_owned_replace_transition_v0(
        value: DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionActionV0 {
            base,
            revision,
            created_at,
            updated_at,
            data,
        } = value;

        let id = base.id();

        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0 {
                id,
                owner_id,
                properties: data,
                revision: Some(revision),
                created_at,
                updated_at,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::try_from_replace_transition".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
