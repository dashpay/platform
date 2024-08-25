use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use dpp::consensus::state::document::document_contest_currently_locked_error::DocumentContestCurrentlyLockedError;
use dpp::consensus::state::document::document_contest_identity_already_contestant::DocumentContestIdentityAlreadyContestantError;
use dpp::consensus::state::document::document_contest_not_joinable_error::DocumentContestNotJoinableError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfoV0Getters};
use drive::error::drive::DriveError;
use drive::query::TransactionArg;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::documents_batch::state::v0::fetch_contender::fetch_contender;
use crate::execution::validation::state_transition::documents_batch::state::v0::fetch_documents::fetch_document_with_id;
use crate::platform_types::platform::PlatformStateRef;

pub(super) trait DocumentCreateTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentCreateTransitionActionStateValidationV0 for DocumentCreateTransitionAction {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.base().data_contract_fetch_info();

        let contract = &contract_fetch_info.contract;

        let document_type_name = self.base().document_type_name();

        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
            ));
        };

        // TODO: Use multi get https://github.com/facebook/rocksdb/wiki/MultiGet-Performance
        // We should check to see if a document already exists in the state
        let (already_existing_document, fee_result) = fetch_document_with_id(
            platform.drive,
            contract,
            document_type,
            self.base().id(),
            transaction,
            platform_version,
        )?;

        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

        if already_existing_document.is_some() {
            return Ok(ConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::DocumentAlreadyPresentError(
                    DocumentAlreadyPresentError::new(self.base().id()),
                )),
            ));
        }

        // we also need to validate that the new document wouldn't conflict with any other document
        // this means for example having overlapping unique indexes

        if document_type.indexes().values().any(|index| index.unique) {
            let validation_result = platform
                .drive
                .validate_document_create_transition_action_uniqueness(
                    contract,
                    document_type,
                    self,
                    owner_id,
                    transaction,
                    platform_version,
                )
                .map_err(Error::Drive)?;

            if !validation_result.is_valid() {
                return Ok(validation_result);
            }
        }

        if let Some((contested_document_resource_vote_poll, _)) = self.prefunded_voting_balance() {
            if let Some(stored_info) = self.current_store_contest_info() {
                // We have previous stored info
                match stored_info.vote_poll_status() {
                    ContestedDocumentVotePollStatus::NotStarted => {
                        Ok(SimpleConsensusValidationResult::new())
                    }
                    ContestedDocumentVotePollStatus::Awarded(_) => {
                        // This is weird as it should have already been found when querying the document, however it is possible
                        // That it was destroyed
                        Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::StateError(StateError::DocumentAlreadyPresentError(
                                DocumentAlreadyPresentError::new(self.base().id()),
                            )),
                        ))
                    }
                    ContestedDocumentVotePollStatus::Locked => {
                        Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::StateError(StateError::DocumentContestCurrentlyLockedError(
                                DocumentContestCurrentlyLockedError::new(
                                    contested_document_resource_vote_poll.into(),
                                    stored_info.clone(),
                                    platform_version.fee_version.vote_resolution_fund_fees.contested_document_vote_resolution_unlock_fund_required_amount,
                                ))),
                        ))
                    }
                    ContestedDocumentVotePollStatus::Started(start_block) => {
                        // We need to make sure that if there is a contest, it is in its first week
                        // The week might be more or less, as it's a versioned parameter
                        let time_ms_since_start = block_info.time_ms.checked_sub(start_block.time_ms).ok_or(Error::Drive(drive::error::Error::Drive(DriveError::CorruptedDriveState(format!("it makes no sense that the start block time {} is before our current block time {}", start_block.time_ms, block_info.time_ms)))))?;
                        let join_time_allowed = platform_version.dpp.validation.voting.allow_other_contenders_time_ms;
                        if time_ms_since_start > join_time_allowed {
                            return Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(StateError::DocumentContestNotJoinableError(
                                DocumentContestNotJoinableError::new(
                                    contested_document_resource_vote_poll.into(),
                                    stored_info.clone(),
                                    start_block.time_ms,
                                    block_info.time_ms,
                                    join_time_allowed,
                                )))))
                        }

                        // we need to also make sure that we are not already a contestant

                        let (maybe_existing_contender, fee_result) = fetch_contender(platform.drive, contested_document_resource_vote_poll, owner_id, block_info, transaction, platform_version)?;

                        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                        if maybe_existing_contender.is_some() {
                            Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(StateError::DocumentContestIdentityAlreadyContestantError(DocumentContestIdentityAlreadyContestantError::new(contested_document_resource_vote_poll.into(), owner_id)))))
                        } else {
                            Ok(SimpleConsensusValidationResult::new())
                        }
                    }
                }
            } else {
                Ok(SimpleConsensusValidationResult::new())
            }
        } else {
            Ok(SimpleConsensusValidationResult::new())
        }
    }
}
