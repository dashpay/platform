use dpp::block::epoch::Epoch;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::documents_batch::state::v0::fetch_documents::fetch_document_with_id;
use crate::platform_types::platform::PlatformStateRef;

pub(super) trait DocumentCreateTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        epoch: &Epoch,
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
        _epoch: &Epoch,
        _execution_context: &mut StateTransitionExecutionContext,
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
        let already_existing_document = fetch_document_with_id(
            platform.drive,
            contract,
            document_type,
            self.base().id(),
            transaction,
            platform_version,
        )?;

        if already_existing_document.is_some() {
            return Ok(ConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::DocumentAlreadyPresentError(
                    DocumentAlreadyPresentError::new(self.base().id()),
                )),
            ));
        }

        // we also need to validate that the new document wouldn't conflict with any other document
        // this means for example having overlapping unique indexes

        if document_type.indices().iter().any(|index| index.unique) {
            platform
                .drive
                .validate_document_create_transition_action_uniqueness(
                    contract,
                    document_type,
                    self,
                    owner_id,
                    transaction,
                    platform_version,
                )
                .map_err(Error::Drive)
        } else {
            Ok(SimpleConsensusValidationResult::new())
        }
    }
}
