/// transformer
pub mod transformer;
mod v0;

use derive_more::From;

use dpp::block::block_info::BlockInfo;
use dpp::platform_value::{Identifier, Value};
use std::collections::BTreeMap;

use dpp::document::Document;
use dpp::fee::Credits;

use dpp::ProtocolError;

pub use v0::*;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction};
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;

/// document create transition action
#[derive(Debug, Clone, From)]
pub enum DocumentCreateTransitionAction {
    /// v0
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

    fn block_info(&self) -> BlockInfo {
        match self {
            DocumentCreateTransitionAction::V0(v0) => v0.block_info,
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

    fn take_prefunded_voting_balance(
        &mut self,
    ) -> Option<(ContestedDocumentResourceVotePollWithContractInfo, Credits)> {
        match self {
            DocumentCreateTransitionAction::V0(v0) => v0.prefunded_voting_balance.take(),
        }
    }

    fn prefunded_voting_balance(
        &self,
    ) -> &Option<(ContestedDocumentResourceVotePollWithContractInfo, Credits)> {
        match self {
            DocumentCreateTransitionAction::V0(v0) => &v0.prefunded_voting_balance,
        }
    }

    fn should_store_contest_info(&self) -> &Option<ContestedDocumentVotePollStoredInfo> {
        match self {
            DocumentCreateTransitionAction::V0(v0) => &v0.should_store_contest_info,
        }
    }

    fn take_should_store_contest_info(&mut self) -> Option<ContestedDocumentVotePollStoredInfo> {
        match self {
            DocumentCreateTransitionAction::V0(v0) => v0.should_store_contest_info.take(),
        }
    }

    fn current_store_contest_info(&self) -> &Option<ContestedDocumentVotePollStoredInfo> {
        match self {
            DocumentCreateTransitionAction::V0(v0) => &v0.current_store_contest_info,
        }
    }

    fn take_current_store_contest_info(&mut self) -> Option<ContestedDocumentVotePollStoredInfo> {
        match self {
            DocumentCreateTransitionAction::V0(v0) => v0.current_store_contest_info.take(),
        }
    }
}

/// document from create transition
pub trait DocumentFromCreateTransitionAction {
    /// Attempts to create a new `Document` from the given `DocumentCreateTransitionAction` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentCreateTransitionAction` containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_create_transition_action(
        document_create_transition_action: &DocumentCreateTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentCreateTransitionAction` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentCreateTransitionAction` instance containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_owned_create_transition_action(
        document_create_transition_action: DocumentCreateTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromCreateTransitionAction for Document {
    fn try_from_create_transition_action(
        document_create_transition_action: &DocumentCreateTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_create_transition_action {
            DocumentCreateTransitionAction::V0(v0) => {
                Self::try_from_create_transition_action_v0(v0, owner_id, platform_version)
            }
        }
    }

    fn try_from_owned_create_transition_action(
        document_create_transition_action: DocumentCreateTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_create_transition_action {
            DocumentCreateTransitionAction::V0(v0) => {
                Self::try_from_owned_create_transition_action_v0(v0, owner_id, platform_version)
            }
        }
    }
}
