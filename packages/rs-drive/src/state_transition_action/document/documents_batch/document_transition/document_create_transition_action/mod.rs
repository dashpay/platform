pub mod transformer;
mod v0;

use derive_more::From;
use dpp::identity::TimestampMillis;
use dpp::platform_value::{Identifier, Value};
use std::collections::BTreeMap;

use dpp::document::Document;
use dpp::ProtocolError;

pub use v0::*;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction};
use dpp::version::{PlatformVersion};

#[derive(Debug, Clone, From)]
pub enum DocumentCreateTransitionAction {
    V0(DocumentCreateTransitionActionV0),
}

impl DocumentCreateTransitionActionAccessorsV0 for DocumentCreateTransitionAction {
    fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentCreateTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentCreateTransitionAction::V0(v0) => v0.base,
        }
    }

    fn created_at(&self) -> Option<TimestampMillis> {
        match self {
            DocumentCreateTransitionAction::V0(v0) => v0.created_at,
        }
    }

    fn updated_at(&self) -> Option<TimestampMillis> {
        match self {
            DocumentCreateTransitionAction::V0(v0) => v0.updated_at,
        }
    }

    fn data(&self) -> &BTreeMap<String, Value> {
        match self {
            DocumentCreateTransitionAction::V0(v0) => &v0.data,
        }
    }

    fn data_mut(&mut self) -> &mut BTreeMap<String, Value> {
        match self {
            DocumentCreateTransitionAction::V0(v0) => &mut v0.data,
        }
    }

    fn data_owned(self) -> BTreeMap<String, Value> {
        match self {
            DocumentCreateTransitionAction::V0(v0) => v0.data,
        }
    }
}

pub trait DocumentFromCreateTransition {
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
    fn try_from_create_transition(
        document_create_transition_action: &DocumentCreateTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
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
    fn try_from_owned_create_transition(
        document_create_transition_action: DocumentCreateTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromCreateTransition for Document {
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
    fn try_from_create_transition(
        document_create_transition_action: &DocumentCreateTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_create_transition_action {
            DocumentCreateTransitionAction::V0(v0) => {
                Self::try_from_create_transition_v0(v0, owner_id, platform_version)
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
    fn try_from_owned_create_transition(
        document_create_transition_action: DocumentCreateTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_create_transition_action {
            DocumentCreateTransitionAction::V0(v0) => {
                Self::try_from_owned_create_transition_v0(v0, owner_id, platform_version)
            }
        }
    }
}
