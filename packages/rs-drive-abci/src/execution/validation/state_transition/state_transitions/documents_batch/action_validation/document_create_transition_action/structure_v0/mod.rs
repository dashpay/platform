use dpp::consensus::basic::document::{DocumentCreationNotAllowedError, InvalidDocumentTypeError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use dpp::data_contract::validation::DataContractValidationMethodsV0;
use dpp::identifier::Identifier;
use dpp::validation::{SimpleConsensusValidationResult};
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(super) trait DocumentCreateTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentCreateTransitionActionStructureValidationV0 for DocumentCreateTransitionAction {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let data_contract = &contract_fetch_info.contract;
        // Make sure that the document type is defined in the contract
        let document_type_name = self.base().document_type_name();

        let Some(document_type) = data_contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), data_contract.id())
                    .into(),
            ));
        };

        match document_type.creation_restriction_mode() {
            CreationRestrictionMode::NoRestrictions => {}
            CreationRestrictionMode::OwnerOnly => {
                if owner_id != data_contract.owner_id() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DocumentCreationNotAllowedError::new(
                            self.base().data_contract_id(),
                            document_type_name.clone(),
                            document_type.creation_restriction_mode(),
                        )
                        .into(),
                    ));
                }
            }
            CreationRestrictionMode::NoCreationAllowed => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DocumentCreationNotAllowedError::new(
                        self.base().data_contract_id(),
                        document_type_name.clone(),
                        document_type.creation_restriction_mode(),
                    )
                    .into(),
                ));
            }
        }

        // Validate user defined properties

        data_contract
            .validate_document_properties(document_type_name, self.data().into(), platform_version)
            .map_err(Error::Protocol)
    }
}
