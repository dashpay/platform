pub mod transformer;

use dpp::document::{Document, DocumentV0};
use dpp::identity::TimestampMillis;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::{BlockHeight, CoreBlockHeight, Revision};
use dpp::ProtocolError;

use std::collections::BTreeMap;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;

/// document replace transition action v0
#[derive(Debug, Clone)]
pub struct DocumentReplaceTransitionActionV0 {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction,
    /// The current revision we are setting
    pub revision: Revision,
    /// The time the document was last updated
    pub created_at: Option<TimestampMillis>,
    /// The time the document was last updated
    pub updated_at: Option<TimestampMillis>,
    /// The block height at which the document was created
    pub created_at_block_height: Option<BlockHeight>,
    /// The block height at which the document was last updated
    pub updated_at_block_height: Option<BlockHeight>,
    /// The core block height at which the document was created
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    /// The core block height at which the document was last updated
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
    /// Document properties
    pub data: BTreeMap<String, Value>,
}

/// document replace transition action accessors v0
pub trait DocumentReplaceTransitionActionAccessorsV0 {
    /// base
    fn base(&self) -> &DocumentBaseTransitionAction;
    /// base owned
    fn base_owned(self) -> DocumentBaseTransitionAction;
    /// revision
    fn revision(&self) -> Revision;
    /// created at
    fn created_at(&self) -> Option<TimestampMillis>;
    /// updated at
    fn updated_at(&self) -> Option<TimestampMillis>;
    /// data
    fn data(&self) -> &BTreeMap<String, Value>;
    /// data owned
    fn data_owned(self) -> BTreeMap<String, Value>;
}

/// document from replace transition v0
pub trait DocumentFromReplaceTransitionActionV0 {
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
    fn try_from_replace_transition_action_v0(
        value: &DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
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
    fn try_from_owned_replace_transition_action_v0(
        value: DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromReplaceTransitionActionV0 for Document {
    fn try_from_replace_transition_action_v0(
        value: &DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionActionV0 {
            base,
            revision,
            created_at,
            updated_at,
            created_at_block_height,
            updated_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
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
                created_at: *created_at,
                updated_at: *updated_at,
                created_at_block_height: *created_at_block_height,
                updated_at_block_height: *updated_at_block_height,
                created_at_core_block_height: *created_at_core_block_height,
                updated_at_core_block_height: *updated_at_core_block_height,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::try_from_replace_transition".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn try_from_owned_replace_transition_action_v0(
        value: DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionActionV0 {
            base,
            revision,
            created_at,
            updated_at,
            created_at_block_height,
            updated_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
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
                created_at_block_height,
                updated_at_block_height,
                created_at_core_block_height,
                updated_at_core_block_height,
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
