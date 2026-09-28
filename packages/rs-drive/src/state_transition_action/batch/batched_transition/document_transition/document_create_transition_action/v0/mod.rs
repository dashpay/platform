pub mod transformer;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::property_constraints::AggregateRead;
use dpp::document::{Document, DocumentV0};
use dpp::platform_value::{Identifier, Value};
use std::collections::BTreeMap;
use std::vec;

use dpp::ProtocolError;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::document::property_names::{
    CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, TRANSFERRED_AT,
    TRANSFERRED_AT_BLOCK_HEIGHT, TRANSFERRED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT,
    UPDATED_AT_BLOCK_HEIGHT, UPDATED_AT_CORE_BLOCK_HEIGHT,
};
use dpp::fee::Credits;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
use crate::state_transition_action::batch::batched_transition::document_transition::drop_transient_values;

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
    /// The `countOf` and `sumOf` totals the document type's `propertyConstraints` rules
    /// read, each as it will be once this write is done, read from state when the action is
    /// built; `None` when the rules judging the write read none, and boxed, since only
    /// such a write holds any and the action is one variant of a large enum.
    pub property_constraint_aggregates: Option<Box<BTreeMap<AggregateRead, i128>>>,
    /// The documents this create consumes: commitments it revealed through a `refersTo` lookup
    /// with a computed key declaring `consume`, deleted in the same state transition. Empty
    /// when the action is built; the batch state validation (protocol version 14) sets it once
    /// the create is accepted.
    pub consumed_documents: Vec<ConsumedDocument>,
}

/// A document of the create's own contract that the create deletes because it revealed it:
/// the commitment a `refersTo` lookup with a computed key found, declaring `consume`. Its
/// owner is the writer (registration demands the `$ownerId` agreement pair), so the delete is
/// the one that owner could have made, and its storage is refunded the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumedDocument {
    /// The consumed document's id.
    pub document_id: Identifier,
    /// The consumed document's type, in the create's contract.
    pub document_type_name: String,
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

    /// Sets what a contested create pays into its contest, which state validation settles at
    /// the fund to join the contest once it has checked the contender stated at least that. A
    /// create that joins no contest is left as it is.
    fn set_prefunded_voting_fund(&mut self, fund: Credits);

    /// Get the should store contest info (if it should be stored)
    fn should_store_contest_info(&self) -> &Option<ContestedDocumentVotePollStoredInfo>;

    /// Take the should store contest info (if it should be stored) and replace it with None.
    fn take_should_store_contest_info(&mut self) -> Option<ContestedDocumentVotePollStoredInfo>;

    /// Get the current store contest info (if it should be stored)
    fn current_store_contest_info(&self) -> &Option<ContestedDocumentVotePollStoredInfo>;

    /// Take the current store contest info (if it should be stored) and replace it with None.
    fn take_current_store_contest_info(&mut self) -> Option<ContestedDocumentVotePollStoredInfo>;

    /// The `countOf` and `sumOf` totals the rules judging this write read, each as it will
    /// be once the write is done
    fn property_constraint_aggregates(&self) -> &BTreeMap<AggregateRead, i128>;

    /// Sets the totals the rules judging this write read, once they are read from state
    fn set_property_constraint_aggregates(&mut self, aggregates: BTreeMap<AggregateRead, i128>);

    /// The documents this create consumes, deleted in the same state transition.
    fn consumed_documents(&self) -> &[ConsumedDocument];

    /// Sets the documents this create consumes.
    fn set_consumed_documents(&mut self, consumed_documents: Vec<ConsumedDocument>);
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
    fn try_from_owned_create_transition_action_v0(
        v0: DocumentCreateTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentCreateTransitionActionV0 {
            base,
            block_info,
            mut data,
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

                drop_transient_values(&mut data, document_type.transient_fields());

                let creator_id = if document_type.should_use_creator_id(
                    data_contract.contract.system_version_type(),
                    data_contract.contract.config().version(),
                    platform_version,
                )? {
                    Some(owner_id)
                } else {
                    None
                };

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
                        contract_version: None,
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
                        creator_id,
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

        let mut data = data.clone();

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

                drop_transient_values(&mut data, document_type.transient_fields());

                let creator_id = if document_type.should_use_creator_id(
                    data_contract.contract.system_version_type(),
                    data_contract.contract.config().version(),
                    platform_version,
                )? {
                    Some(owner_id)
                } else {
                    None
                };

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
                        contract_version: None,
                        id: *id,
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
                        creator_id,
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
}
