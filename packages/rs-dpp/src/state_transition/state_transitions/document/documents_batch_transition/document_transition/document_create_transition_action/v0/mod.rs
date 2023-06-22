use crate::document::document_transition::document_create_transition::DocumentCreateTransitionV0;
use crate::document::document_transition::{
    DocumentBaseTransitionAction, DocumentCreateTransition,
};
use crate::document::Document;
use crate::identity::TimestampMillis;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

use crate::ProtocolError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DocumentCreateTransitionActionV0 {
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

impl From<DocumentCreateTransitionV0> for DocumentCreateTransitionActionV0 {
    fn from(value: DocumentCreateTransitionV0) -> Self {
        let DocumentCreateTransitionV0 {
            base,
            created_at,
            updated_at,
            data,
            ..
        } = value;
        DocumentCreateTransitionActionV0 {
            base: base.into(),
            created_at,
            updated_at,
            data: data.unwrap_or_default(),
        }
    }
}

impl From<&DocumentCreateTransitionV0> for DocumentCreateTransitionActionV0 {
    fn from(value: &DocumentCreateTransitionV0) -> Self {
        let DocumentCreateTransitionV0 {
            base,
            created_at,
            updated_at,
            data,
            ..
        } = value;
        DocumentCreateTransitionActionV0 {
            base: base.into(),
            created_at: *created_at,
            updated_at: *updated_at,
            data: data.clone().unwrap_or_default(),
        }
    }
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
        value: &DocumentCreateTransitionActionV0,
        owner_id: Identifier,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionActionV0 {
            base,
            created_at,
            updated_at,
            data,
        } = value;

        let DocumentBaseTransitionActionV0 {
            id,
            document_type_name,
            data_contract,
            ..
        } = base;

        let document_type = data_contract.document_type_for_name(document_type_name.as_str())?;

        Ok(Document {
            id: *id,
            owner_id,
            properties: data.clone(),
            revision: document_type.initial_revision(),
            created_at: created_at.clone(),
            updated_at: updated_at.clone(),
        })
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
    pub fn try_from_owned_create_transition(
        value: DocumentCreateTransitionAction,
        owner_id: Identifier,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionAction {
            base,
            created_at,
            updated_at,
            data,
        } = value;

        let DocumentBaseTransitionAction {
            id,
            document_type_name,
            data_contract,
            ..
        } = base;

        let document_type = data_contract.document_type_for_name(document_type_name.as_str())?;

        Ok(Document {
            id,
            owner_id,
            properties: data,
            revision: document_type.initial_revision(),
            created_at,
            updated_at,
        })
    }
}
