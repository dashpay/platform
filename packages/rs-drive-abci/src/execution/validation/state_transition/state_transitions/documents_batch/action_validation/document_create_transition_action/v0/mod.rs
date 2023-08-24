use dpp::consensus::basic::document::{InvalidDocumentTypeError, MissingDocumentTypeError};
use dpp::consensus::state::document::document_timestamps_mismatch_error::DocumentTimestampsMismatchError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::validation::DataContractValidationMethodsV0;
use dpp::document::extended_document::property_names;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(super) trait DocumentCreateTransitionActionValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentCreateTransitionActionValidationV0 for DocumentCreateTransitionAction {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let data_contract = &contract_fetch_info.contract;
        // Make sure that the document type is defined in the contract
        let document_type_name = self.base().document_type_name();

        let Some(document_type) = data_contract
            .document_type_optional_for_name(document_type_name) else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), data_contract.id()).into(),
            ));
        };

        // Make sure that timestamps are present if required
        let required_fields = document_type.required_fields();

        if required_fields.contains(property_names::CREATED_AT) && self.created_at().is_none() {
            // TODO: Create a special consensus error for this
            return Ok(SimpleConsensusValidationResult::new_with_error(
                MissingDocumentTypeError::new().into(),
            ));
        }

        if required_fields.contains(property_names::UPDATED_AT) && self.updated_at().is_none() {
            // TODO: Create a special consensus error for this
            return Ok(SimpleConsensusValidationResult::new_with_error(
                MissingDocumentTypeError::new().into(),
            ));
        }

        if self.created_at().is_some()
            && self.updated_at().is_some()
            && self.created_at() != self.updated_at()
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DocumentTimestampsMismatchError::new(self.base().id()).into(),
            ));
        }

        // Validate user defined properties

        data_contract
            .validate_document_properties(document_type_name, self.data().into(), platform_version)
            .map_err(Error::Protocol)
    }
}
