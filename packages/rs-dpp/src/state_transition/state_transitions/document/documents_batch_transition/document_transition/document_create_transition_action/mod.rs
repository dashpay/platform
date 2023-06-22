mod v0;

use crate::document::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::document::document_transition::DocumentCreateTransition;
use crate::identity::TimestampMillis;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

use crate::document::Document;
use crate::ProtocolError;

use serde::{Deserialize, Serialize};

pub use v0::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocumentCreateTransitionAction {
    V0(DocumentCreateTransitionActionV0),
}

impl Default for DocumentBaseTransitionAction {
    fn default() -> Self {
        DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0::default())
        // since only v0
    }
}

impl Document {
    /// Attempts to create a new `Document` from the given `DocumentCreateTransition` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentCreateTransition` containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    pub fn try_from_create_transition(
        value: &DocumentCreateTransitionAction,
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
