use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::document::Document;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfoV0Getters,
};
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::drive::document::shared_unique_entry::unique_index_entry_is_shared;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
    DocumentFromCreateTransitionAction,
};
use drive::util::object_size_info::DocumentInfo;

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::state_v2::DocumentCreateTransitionActionStateValidationV2;
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::ConsumedLookupDocument;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentCreateTransitionActionStateValidationV3
{
    #[allow(clippy::too_many_arguments)]
    fn validate_state_v3(
        &mut self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        consumed_documents: &mut Vec<ConsumedLookupDocument>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentCreateTransitionActionStateValidationV3 for DocumentCreateTransitionAction {
    fn validate_state_v3(
        &mut self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        consumed_documents: &mut Vec<ConsumedLookupDocument>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.validate_state_v2(
            platform,
            owner_id,
            block_info,
            consumed_documents,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // A non-contested create must not take the index values of a live contest. A contest
        // decided by its index values takes every document with them, so this holds by
        // itself; a contest whose `fieldMatches` read something else, which registration
        // refuses from protocol version 15 but a contract registered earlier may keep, lets a
        // document with the same values bypass it. Such a document would take the unique index
        // entry the contest awards at its end, leaving the award nothing to take.
        // The poll is read at its path, which encodes the values as index keys, so a create of
        // `"\0"` finds the live contest of `""` (both are the key `[0]`) and is refused too.
        if self.prefunded_voting_balance().is_some() {
            return Ok(SimpleConsensusValidationResult::new());
        }
        let document_type = self.base().document_type()?;
        let Some(contested_index) = document_type.find_contested_index() else {
            return Ok(SimpleConsensusValidationResult::new());
        };
        if contested_index.contest_is_decided_by_index_values(document_type.flattened_properties())
        {
            return Ok(SimpleConsensusValidationResult::new());
        }

        let index_values = contested_index
            .extract_contested_values(
                self.data(),
                document_type.flattened_properties(),
                platform_version,
            )
            .map_err(Error::Protocol)?;
        // A document whose unique index entry Drive keeps per document (a value it lacks, or
        // one with an empty index key, such as an empty byte array) takes nothing from a
        // contest: nothing is read
        let document =
            Document::try_from_create_transition_action(self, owner_id, platform_version)?;
        if !unique_index_entry_is_shared(
            &DocumentInfo::DocumentRefInfo((&document, None)),
            contested_index,
            document_type,
            Some(owner_id.to_buffer()),
            platform_version,
        )? {
            return Ok(SimpleConsensusValidationResult::new());
        }
        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id: self.base().data_contract_id(),
            document_type_name: self.base().document_type_name().clone(),
            index_name: contested_index.name.clone(),
            index_values,
        }
        .resolve_owned_with_provided_arc_contract_fetch_info(
            self.base().data_contract_fetch_info(),
        )?;

        let (fee_result, stored_info) = platform
            .drive
            .fetch_contested_document_vote_poll_stored_info(
                &vote_poll,
                Some(&block_info.epoch),
                transaction,
                platform_version,
            )?;
        if let Some(fee_result) = fee_result {
            execution_context
                .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));
        }

        if stored_info.is_some_and(|stored_info| {
            matches!(
                stored_info.vote_poll_status(),
                ContestedDocumentVotePollStatus::Started(_)
            )
        }) {
            return Ok(ConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::DuplicateUniqueIndexError(
                    DuplicateUniqueIndexError::new(
                        self.base().id(),
                        contested_index
                            .properties
                            .iter()
                            .map(|property| property.name.clone())
                            .collect(),
                    ),
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
