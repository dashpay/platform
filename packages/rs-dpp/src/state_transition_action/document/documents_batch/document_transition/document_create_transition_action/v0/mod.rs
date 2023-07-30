#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

use crate::document::{Document, DocumentV0};
use crate::identity::TimestampMillis;
use bincode::{Decode, Encode};
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

use crate::ProtocolError;
use serde::{Deserialize, Serialize};
use crate::data_contract::base::DataContractBaseMethodsV0;
use crate::data_contract::document_type::v0::v0_methods::DocumentTypeV0Methods;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
use crate::version::PlatformVersion;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct DocumentCreateTransitionActionV0<'a> {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction<'a>,
    /// The creation time of the document
    pub created_at: Option<TimestampMillis>,
    //todo: remove updated_at
    /// The time the document was last updated
    pub updated_at: Option<TimestampMillis>,
    /// Document properties
    pub data: BTreeMap<String, Value>,
}

pub trait DocumentCreateTransitionActionAccessorsV0<'a> {
    fn base(&self) -> &DocumentBaseTransitionAction;
    fn base_owned(self) -> DocumentBaseTransitionAction<'a>;
    fn created_at(&self) -> Option<TimestampMillis>;
    fn updated_at(&self) -> Option<TimestampMillis>;
    fn data(&self) -> &BTreeMap<String, Value>;
    fn data_owned(self) -> BTreeMap<String, Value>;
}

impl Document {
    /// Attempts to create a new `Document` from the given `DocumentCreateTransition` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentCreateTransitionActionV0` containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    pub(crate) fn try_from_create_transition_v0(
        v0: &DocumentCreateTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionActionV0 {
            base,
            created_at,
            updated_at,
            data,
        } = v0;

        match base {
            DocumentBaseTransitionAction::V0(base_v0) => {
                let DocumentBaseTransitionActionV0 {
                    id,
                    document_type_name,
                    data_contract,
                    ..
                } = base_v0;

                let document_type =
                    data_contract.document_type_for_name(document_type_name.as_str())?;

                match platform_version
                    .dpp
                    .document_versions
                    .document_structure_version
                {
                    0 => Ok(DocumentV0 {
                        id: *id,
                        owner_id,
                        properties: data.clone(),
                        revision: document_type.initial_revision(),
                        created_at: created_at.clone(),
                        updated_at: updated_at.clone(),
                    }
                    .into()),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "Document::try_from_create_transition_v0".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }

    /// Attempts to create a new `Document` from the given `DocumentCreateTransition` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentCreateTransition` instance containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    pub(crate) fn try_from_owned_create_transition_v0(
        v0: DocumentCreateTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionActionV0 {
            base,
            created_at,
            updated_at,
            data,
        } = v0;

        match base {
            DocumentBaseTransitionAction::V0(base_v0) => {
                let DocumentBaseTransitionActionV0 {
                    id,
                    document_type_name,
                    data_contract,
                    ..
                } = base_v0;

                let document_type =
                    data_contract.document_type_for_name(document_type_name.as_str())?;

                match platform_version
                    .dpp
                    .document_versions
                    .document_structure_version
                {
                    0 => Ok(DocumentV0 {
                        id,
                        owner_id,
                        properties: data,
                        revision: document_type.initial_revision(),
                        created_at,
                        updated_at,
                    }
                    .into()),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "Document::try_from_owned_create_transition_v0".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }
}
