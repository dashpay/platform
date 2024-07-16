pub mod transformer;

use dpp::block::block_info::BlockInfo;
use dpp::document::{Document, DocumentV0};
use dpp::platform_value::{Identifier, Value};
use std::collections::BTreeMap;
use std::vec;

use dpp::ProtocolError;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::property_names::{
    CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, TRANSFERRED_AT,
    TRANSFERRED_AT_BLOCK_HEIGHT, TRANSFERRED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT,
    UPDATED_AT_BLOCK_HEIGHT, UPDATED_AT_CORE_BLOCK_HEIGHT,
};
use dpp::fee::Credits;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;

/// document create transition action v0
#[derive(Debug, Clone)]
pub struct DocumentCreateTransitionActionV0 {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction,
    /// The block_info at the time of creation
    pub block_info: BlockInfo,
    /// Document properties
    pub data: BTreeMap<String, Value>,
    /// Pre funded balance (for unique index conflict resolution voting - the identity will put money
    /// aside that will be used by voters to vote)
    pub prefunded_voting_balance:
        Option<(ContestedDocumentResourceVotePollWithContractInfo, Credits)>,
    /// We store contest info only in the case of a new contested document that creates a new contest
    pub current_store_contest_info: Option<ContestedDocumentVotePollStoredInfo>,
    /// We store contest info only in the case of a new contested document that creates a new contest
    pub should_store_contest_info: Option<ContestedDocumentVotePollStoredInfo>,
}

/// document create transition action accessors v0
pub trait DocumentCreateTransitionActionAccessorsV0 {
    /// base
    fn base(&self) -> &DocumentBaseTransitionAction;
    /// base owned
    fn base_owned(self) -> DocumentBaseTransitionAction;
    /// block info
    fn block_info(&self) -> BlockInfo;
    /// data
    fn data(&self) -> &BTreeMap<String, Value>;
    /// data mut
    fn data_mut(&mut self) -> &mut BTreeMap<String, Value>;
    /// data owned
    fn data_owned(self) -> BTreeMap<String, Value>;

    /// Take the prefunded voting balance vec (and replace it with an empty vec).
    fn take_prefunded_voting_balance(
        &mut self,
    ) -> Option<(ContestedDocumentResourceVotePollWithContractInfo, Credits)>;

    /// pre funded balance (for unique index conflict resolution voting - the identity will put money
    /// aside that will be used by voters to vote)
    fn prefunded_voting_balance(
        &self,
    ) -> &Option<(ContestedDocumentResourceVotePollWithContractInfo, Credits)>;

    /// Get the should store contest info (if it should be stored)
    fn should_store_contest_info(&self) -> &Option<ContestedDocumentVotePollStoredInfo>;

    /// Take the should store contest info (if it should be stored) and replace it with None.
    fn take_should_store_contest_info(&mut self) -> Option<ContestedDocumentVotePollStoredInfo>;

    /// Get the current store contest info (if it should be stored)
    fn current_store_contest_info(&self) -> &Option<ContestedDocumentVotePollStoredInfo>;

    /// Take the current store contest info (if it should be stored) and replace it with None.
    fn take_current_store_contest_info(&mut self) -> Option<ContestedDocumentVotePollStoredInfo>;
}

/// documents from create transition v0
pub trait DocumentFromCreateTransitionActionV0 {
    /// Attempts to create a new `Document` from the given `DocumentCreateTransitionActionV0` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentCreateTransitionActionV0` instance containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_owned_create_transition_action_v0(
        v0: DocumentCreateTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentCreateTransitionActionV0` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentCreateTransitionActionV0` containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_create_transition_action_v0(
        v0: &DocumentCreateTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromCreateTransitionActionV0 for Document {
    fn try_from_create_transition_action_v0(
        v0: &DocumentCreateTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionActionV0 {
            base,
            block_info,
            data,
            ..
        } = v0;

        match base {
            DocumentBaseTransitionAction::V0(base_v0) => {
                let DocumentBaseTransitionActionV0 {
                    id,
                    document_type_name,
                    data_contract,
                    ..
                } = base_v0;

                let document_type = data_contract
                    .contract
                    .document_type_for_name(document_type_name.as_str())?;

                let required_fields = document_type.required_fields();

                let is_created_at_required = required_fields.contains(CREATED_AT);
                let is_updated_at_required = required_fields.contains(UPDATED_AT);
                let is_transferred_at_required = required_fields.contains(TRANSFERRED_AT);

                let is_created_at_block_height_required =
                    required_fields.contains(CREATED_AT_BLOCK_HEIGHT);
                let is_updated_at_block_height_required =
                    required_fields.contains(UPDATED_AT_BLOCK_HEIGHT);
                let is_transferred_at_block_height_required =
                    required_fields.contains(TRANSFERRED_AT_BLOCK_HEIGHT);

                let is_created_at_core_block_height_required =
                    required_fields.contains(CREATED_AT_CORE_BLOCK_HEIGHT);
                let is_updated_at_core_block_height_required =
                    required_fields.contains(UPDATED_AT_CORE_BLOCK_HEIGHT);
                let is_transferred_at_core_block_height_required =
                    required_fields.contains(TRANSFERRED_AT_CORE_BLOCK_HEIGHT);

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
                        created_at: if is_created_at_required {
                            Some(block_info.time_ms)
                        } else {
                            None
                        },
                        updated_at: if is_updated_at_required {
                            Some(block_info.time_ms)
                        } else {
                            None
                        },
                        transferred_at: if is_transferred_at_required {
                            Some(block_info.time_ms)
                        } else {
                            None
                        },
                        created_at_block_height: if is_created_at_block_height_required {
                            Some(block_info.height)
                        } else {
                            None
                        },
                        updated_at_block_height: if is_updated_at_block_height_required {
                            Some(block_info.height)
                        } else {
                            None
                        },
                        transferred_at_block_height: if is_transferred_at_block_height_required {
                            Some(block_info.height)
                        } else {
                            None
                        },
                        created_at_core_block_height: if is_created_at_core_block_height_required {
                            Some(block_info.core_height)
                        } else {
                            None
                        },
                        updated_at_core_block_height: if is_updated_at_core_block_height_required {
                            Some(block_info.core_height)
                        } else {
                            None
                        },
                        transferred_at_core_block_height:
                            if is_transferred_at_core_block_height_required {
                                Some(block_info.core_height)
                            } else {
                                None
                            },
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

    fn try_from_owned_create_transition_action_v0(
        v0: DocumentCreateTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionActionV0 {
            base,
            block_info,
            data,
            ..
        } = v0;

        match base {
            DocumentBaseTransitionAction::V0(base_v0) => {
                let DocumentBaseTransitionActionV0 {
                    id,
                    document_type_name,
                    data_contract,
                    ..
                } = base_v0;

                let document_type = data_contract
                    .contract
                    .document_type_for_name(document_type_name.as_str())?;

                let required_fields = document_type.required_fields();

                let is_created_at_required = required_fields.contains(CREATED_AT);
                let is_updated_at_required = required_fields.contains(UPDATED_AT);
                let is_transferred_at_required = required_fields.contains(TRANSFERRED_AT);

                let is_created_at_block_height_required =
                    required_fields.contains(CREATED_AT_BLOCK_HEIGHT);
                let is_updated_at_block_height_required =
                    required_fields.contains(UPDATED_AT_BLOCK_HEIGHT);
                let is_transferred_at_block_height_required =
                    required_fields.contains(TRANSFERRED_AT_BLOCK_HEIGHT);

                let is_created_at_core_block_height_required =
                    required_fields.contains(CREATED_AT_CORE_BLOCK_HEIGHT);
                let is_updated_at_core_block_height_required =
                    required_fields.contains(UPDATED_AT_CORE_BLOCK_HEIGHT);
                let is_transferred_at_core_block_height_required =
                    required_fields.contains(TRANSFERRED_AT_CORE_BLOCK_HEIGHT);

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
                        created_at: if is_created_at_required {
                            Some(block_info.time_ms)
                        } else {
                            None
                        },
                        updated_at: if is_updated_at_required {
                            Some(block_info.time_ms)
                        } else {
                            None
                        },
                        transferred_at: if is_transferred_at_required {
                            Some(block_info.time_ms)
                        } else {
                            None
                        },
                        created_at_block_height: if is_created_at_block_height_required {
                            Some(block_info.height)
                        } else {
                            None
                        },
                        updated_at_block_height: if is_updated_at_block_height_required {
                            Some(block_info.height)
                        } else {
                            None
                        },
                        transferred_at_block_height: if is_transferred_at_block_height_required {
                            Some(block_info.height)
                        } else {
                            None
                        },
                        created_at_core_block_height: if is_created_at_core_block_height_required {
                            Some(block_info.core_height)
                        } else {
                            None
                        },
                        updated_at_core_block_height: if is_updated_at_core_block_height_required {
                            Some(block_info.core_height)
                        } else {
                            None
                        },
                        transferred_at_core_block_height:
                            if is_transferred_at_core_block_height_required {
                                Some(block_info.core_height)
                            } else {
                                None
                            },
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
