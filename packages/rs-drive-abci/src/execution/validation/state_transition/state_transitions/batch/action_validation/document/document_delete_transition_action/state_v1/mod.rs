use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::property_constraints::DocumentSystemValues;
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_base_transaction_action::DocumentBaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_delete_transition_action::state_v0::check_ownership;
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::fetch_document_with_id;
use crate::execution::validation::state_transition::batch::transformer::v0::property_constraint_aggregates::read_delete_constraint_aggregates;
use crate::platform_types::platform::PlatformStateRef;

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
    /// Protocol version 14: the checks of version 0 (the base transition, the stored
    /// document, its owner), then the document type's `deleteConstraints`, judged on the
    /// stored document with the `countOf` and `sumOf` totals they read as they will be once
    /// it is gone. The first rule the document breaks refuses the delete with
    /// `DocumentDeleteConstraintViolatedError` (40147), a state error, so the delete is paid
    /// and the nonce bumped. A type without delete rules reads nothing more than version 0.
    fn validate_state_v1(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
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

        let contract_fetch_info = self.base().data_contract_fetch_info();

        let contract = &contract_fetch_info.contract;

        let document_type_name = self.base().document_type_name();

        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
            ));
        };

        let original_document = fetch_document_with_id(
            platform.drive,
            contract,
            document_type,
            self.base().id(),
            &block_info.epoch,
            execution_context,
            transaction,
            platform_version,
        )?;

        let Some(document) = original_document else {
            return Ok(ConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::DocumentNotFoundError(
                    DocumentNotFoundError::new(self.base().id()),
                )),
            ));
        };

        let ownership_result = check_ownership(self, &document, &owner_id);
        if !ownership_result.is_valid() || document_type.delete_constraints().is_empty() {
            return Ok(ownership_result);
        }

        // -->> Introduced in V1 <<--
        // The rules read the stored document, its owner and its times and heights, and the
        // totals read here, each billed, as they will be once the document is gone
        let aggregates = read_delete_constraint_aggregates(
            platform.drive,
            contract,
            document_type_name,
            &document,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        document_type
            .validate_delete_constraints(
                document.id(),
                document.properties(),
                &DocumentSystemValues {
                    aggregates: Some(aggregates),
                    ..DocumentSystemValues::of_document(&document)
                },
                platform_version,
            )
            .map_err(Error::Protocol)
    }
}
