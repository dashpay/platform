use std::collections::BTreeMap;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::document_contest_document_with_same_id_already_present_error::DocumentContestDocumentWithSameIdAlreadyPresentError;
use dpp::consensus::state::document::document_contest_maximum_contenders_reached_error::DocumentContestMaximumContendersReachedError;
use dpp::consensus::state::document::document_contest_not_paid_for_error::DocumentContestNotPaidForError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::required_vote_resolution_fund_to_join;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
};

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::state_v1::DocumentCreateTransitionActionStateValidationV1;
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::{ConsumedLookupDocument, DocumentReferenceValidation};
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::has_contested_document_with_document_id;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentCreateTransitionActionStateValidationV2
{
    #[allow(clippy::too_many_arguments)]
    fn validate_state_v2(
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

impl DocumentCreateTransitionActionStateValidationV2 for DocumentCreateTransitionAction {
    fn validate_state_v2(
        &mut self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        consumed_documents: &mut Vec<ConsumedLookupDocument>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.validate_state_v1(
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // A contest accepts at most `max_contenders_per_contest` contenders, so the end of the
        // poll can tally and clean up every one in a block, and the fund a contender pays to
        // join doubles once it holds `contested_document_contenders_before_fund_doubling`
        // contenders and again for every `contested_document_contenders_per_fund_doubling` more,
        // so filling it costs far more than the fund times the contenders. v1 has let the
        // document join the contest when it exists; a new contest has no contenders to count.
        if let Some((contested_document_resource_vote_poll, most_it_pays)) =
            self.prefunded_voting_balance()
        {
            let contenders = if self.current_store_contest_info().is_some() {
                let max_contenders = platform_version.system_limits.max_contenders_per_contest;
                let (fee_result, contenders) = platform
                    .drive
                    .fetch_contested_document_vote_poll_contender_count(
                        contested_document_resource_vote_poll,
                        max_contenders,
                        &block_info.epoch,
                        transaction,
                        platform_version,
                    )?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                if contenders >= max_contenders {
                    return Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::DocumentContestMaximumContendersReachedError(
                                DocumentContestMaximumContendersReachedError::new(
                                    contested_document_resource_vote_poll.into(),
                                    max_contenders,
                                ),
                            ),
                        ),
                    ));
                }
                contenders
            } else {
                0
            };

            // The contender states the most it pays and is charged the fund to join, what it
            // stated beyond that staying with it
            let fund_to_join = required_vote_resolution_fund_to_join(
                &contested_document_resource_vote_poll.contract.id(),
                &contested_document_resource_vote_poll.document_type_name,
                contenders,
                platform_version,
            );
            if *most_it_pays < fund_to_join {
                return Ok(ConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::DocumentContestNotPaidForError(
                        DocumentContestNotPaidForError::new(
                            self.base().id(),
                            fund_to_join,
                            *most_it_pays,
                        ),
                    )),
                ));
            }
            self.set_prefunded_voting_fund(fund_to_join);
        }

        // The creator of a document being created is its writer; the commitments the create
        // reveals and consumes are collected for the batch to delete with it, and the values
        // of the type's derived index properties, from the documents the references fetched,
        // for Drive to key the document by without reading those documents again
        let mut derived_index_values = BTreeMap::new();
        let reference_result = self.base().validate_document_references(
            self.data(),
            owner_id,
            Some(owner_id),
            None,
            None,
            platform,
            block_info,
            consumed_documents,
            Some(&mut derived_index_values),
            transaction,
            execution_context,
            platform_version,
        )?;
        if !reference_result.is_valid() {
            return Ok(reference_result);
        }
        self.set_derived_index_values(derived_index_values);

        // A non-contested create must not take the id of a live contested
        // document. Both derive the id from the same owner, document type and
        // entropy, and v1 probes contested storage only for contested creates,
        // so the non-contested document could occupy primary storage while the
        // contest is live and awarding the winner would then fail with
        // `CorruptedDocumentAlreadyExists`, which fails block execution.
        if self.prefunded_voting_balance().is_none() {
            let contract_fetch_info = self.base().data_contract_fetch_info_ref();
            let document_type = self.base().document_type()?;

            if document_type.find_contested_index().is_some() {
                let (contested_document_already_exists, fee_result) =
                    has_contested_document_with_document_id(
                        platform.drive,
                        &contract_fetch_info.contract,
                        document_type,
                        self.base().id(),
                        Some(&block_info.epoch),
                        transaction,
                        platform_version,
                    )?;

                execution_context
                    .add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

                if contested_document_already_exists {
                    return Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::DocumentContestDocumentWithSameIdAlreadyPresentError(
                                DocumentContestDocumentWithSameIdAlreadyPresentError::new(
                                    self.base().id(),
                                ),
                            ),
                        ),
                    ));
                }
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
