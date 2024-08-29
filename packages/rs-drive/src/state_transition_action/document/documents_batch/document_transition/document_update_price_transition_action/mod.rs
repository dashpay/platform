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
pub enum DocumentUpdatePriceTransitionAction {
    /// v0
    V0(DocumentUpdatePriceTransitionActionV0),
}

impl DocumentUpdatePriceTransitionActionAccessorsV0 for DocumentUpdatePriceTransitionAction {
    fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => v0.base,
        }
    }

    fn document(&self) -> &Document {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => &v0.document,
        }
    }

    fn document_owned(self) -> Document {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => v0.document,
        }
    }
}

/// document from update price transition
pub trait DocumentFromUpdatePriceTransitionAction {
    /// Attempts to create a new `Document` from the given `DocumentUpdatePriceTransitionAction` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentUpdatePriceTransitionAction` containing information about the document being update_priced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_update_price_transition_action(
        document_update_price_transition_action: &DocumentUpdatePriceTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentUpdatePriceTransitionAction` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentUpdatePriceTransitionAction` instance containing information about the document being update_priced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_owned_update_price_transition_action(
        document_update_price_transition_action: DocumentUpdatePriceTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
