use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::property_constraints::DocumentSystemValues;
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_delete_transition_action::state_v0::fetch_owned_document;
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
        let owned = fetch_owned_document(
            self,
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        let Some(document) = owned.data else {
            return Ok(SimpleConsensusValidationResult::new_with_errors(
                owned.errors,
            ));
        };

        // -->> Introduced in V1 <<--
        let contract = &self.base().data_contract_fetch_info().contract;
        let document_type_name = self.base().document_type_name();
        // `fetch_owned_document` found the type; refused as it would be, never let through
        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
            ));
        };
        if document_type.delete_constraints().is_empty() {
            return Ok(SimpleConsensusValidationResult::new());
        }

        // The rules read the stored document, its owner and its times and heights, and the
        // totals read here, each billed, as they will be once the document is gone
        let data = Value::from(document.properties().clone());
        let aggregates = read_delete_constraint_aggregates(
            platform.drive,
            contract,
            document_type_name,
            &document,
            &data,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        document_type
            .validate_delete_constraints(
                document.id(),
                &data,
                &DocumentSystemValues {
                    aggregates: Some(aggregates),
                    ..DocumentSystemValues::of_document(&document)
                },
                platform_version,
            )
            .map_err(Error::Protocol)
    }
}
