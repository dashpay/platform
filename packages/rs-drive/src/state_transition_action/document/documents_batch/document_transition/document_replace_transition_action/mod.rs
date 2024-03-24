mod v0;

use std::collections::BTreeMap;

use derive_more::From;

use dpp::document::Document;
use dpp::identity::TimestampMillis;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::{BlockHeight, CoreBlockHeight, Revision};
use dpp::ProtocolError;
pub use v0::*;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use dpp::version::PlatformVersion;

/// tranformer
pub mod transformer;

/// action
#[derive(Debug, Clone, From)]
pub enum DocumentReplaceTransitionAction {
    /// v0
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

    /// Returns the block height at which the document was created.
    fn created_at_block_height(&self) -> Option<BlockHeight> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.created_at_block_height,
        }
    }

    /// Returns the block height at which the document was last updated.
    fn updated_at_block_height(&self) -> Option<BlockHeight> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.updated_at_block_height,
        }
    }

    /// Returns the core block height at which the document was created.
    fn created_at_core_block_height(&self) -> Option<CoreBlockHeight> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.created_at_core_block_height,
        }
    }

    /// Returns the core block height at which the document was last updated.
    fn updated_at_core_block_height(&self) -> Option<CoreBlockHeight> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.updated_at_core_block_height,
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

/// document from replace transition
pub trait DocumentFromReplaceTransitionAction {
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionAction` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentReplaceTransitionAction` containing information about the document being replaced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_replace_transition_action(
        document_replace_transition_action: &DocumentReplaceTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionAction` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentReplaceTransitionAction` instance containing information about the document being replaced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_owned_replace_transition_action(
        document_replace_transition_action: DocumentReplaceTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromReplaceTransitionAction for Document {
    fn try_from_replace_transition_action(
        document_replace_transition_action: &DocumentReplaceTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_replace_transition_action {
            DocumentReplaceTransitionAction::V0(v0) => {
                Self::try_from_replace_transition_action_v0(v0, owner_id, platform_version)
            }
        }
    }

    fn try_from_owned_replace_transition_action(
        document_replace_transition_action: DocumentReplaceTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_replace_transition_action {
            DocumentReplaceTransitionAction::V0(v0) => {
                Self::try_from_owned_replace_transition_action_v0(v0, owner_id, platform_version)
            }
        }
    }
}
