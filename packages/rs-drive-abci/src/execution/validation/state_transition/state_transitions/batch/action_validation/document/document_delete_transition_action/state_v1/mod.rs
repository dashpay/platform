use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_base_transaction_action::DocumentBaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_delete_transition_action::state_v0::DocumentDeleteTransitionActionStateValidationV0;
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::fetch_keep_history_document_lifecycle;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::document::lifecycle::DocumentLifecycleState;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;

use dpp::consensus::state::document::document_owner_id_mismatch_error::DocumentOwnerIdMismatchError;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentDeleteTransitionActionStateValidationV1 {
    fn validate_state_v1(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentDeleteTransitionActionStateValidationV1 for DocumentDeleteTransitionAction {
    /// A keep-history document that has already been deleted is invisible to
    /// the ordinary by-id read v0 relies on, so deleting it a second time would
    /// otherwise look like deleting a document that never existed. The
    /// lifecycle read tells the two apart, and a second delete is refused
    /// rather than escalating into anything that removes a revision: only an
    /// erase does that, and only after this delete has committed the document
    /// to the deleted state.
    ///
    /// Every other document type is validated exactly as v0 validates it.
    fn validate_state_v1(
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

        if !document_type.documents_keep_history() {
            return self.validate_state_v0(
                platform,
                owner_id,
                block_info,
                execution_context,
                transaction,
                platform_version,
            );
        }

        let validation_result = self.base().validate_state(
            platform,
            owner_id,
            block_info,
            "delete",
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let lifecycle = fetch_keep_history_document_lifecycle(
            platform.drive,
            contract,
            document_type,
            self.base().id(),
            &block_info.epoch,
            execution_context,
            transaction,
            platform_version,
        )?;

        let document = match lifecycle {
            DocumentLifecycleState::Active(document) => document,
            // A deleted or erasing document is gone from every ordinary read,
            // which is exactly what a delete asks for, so asking again is the
            // same as asking about an id that holds nothing.
            DocumentLifecycleState::Deleted(_)
            | DocumentLifecycleState::Erasing
            | DocumentLifecycleState::Absent => {
                return Ok(ConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::DocumentNotFoundError(
                        DocumentNotFoundError::new(self.base().id()),
                    )),
                ));
            }
        };

        // The single ownership call site for a delete.
        if document.owner_id() != owner_id {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::DocumentOwnerIdMismatchError(
                    DocumentOwnerIdMismatchError::new(
                        self.base().id(),
                        owner_id,
                        document.owner_id(),
                    ),
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
