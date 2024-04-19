mod v0;

use derive_more::From;
use dpp::document::Document;

use dpp::platform_value::Identifier;
use dpp::ProtocolError;
pub use v0::*;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use dpp::version::PlatformVersion;

/// transformer
pub mod transformer;

/// action
#[derive(Debug, Clone, From)]
pub enum DocumentTransferTransitionAction {
    /// v0
    V0(DocumentTransferTransitionActionV0),
}

impl DocumentTransferTransitionActionAccessorsV0 for DocumentTransferTransitionAction {
    fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentTransferTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentTransferTransitionAction::V0(v0) => v0.base,
        }
    }

    fn document(&self) -> &Document {
        match self {
            DocumentTransferTransitionAction::V0(v0) => &v0.document,
        }
    }

    fn document_owned(self) -> Document {
        match self {
            DocumentTransferTransitionAction::V0(v0) => v0.document,
        }
    }
}

/// document from transfer transition
pub trait DocumentFromTransferTransitionAction {
    /// Attempts to create a new `Document` from the given `DocumentTransferTransitionAction` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentTransferTransitionAction` containing information about the document being transferd.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_transfer_transition_action(
        document_transfer_transition_action: &DocumentTransferTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentTransferTransitionAction` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentTransferTransitionAction` instance containing information about the document being transferd.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_owned_transfer_transition_action(
        document_transfer_transition_action: DocumentTransferTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
