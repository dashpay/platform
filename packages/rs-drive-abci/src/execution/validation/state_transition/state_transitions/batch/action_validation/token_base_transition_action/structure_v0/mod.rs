use dpp::consensus::basic::BasicError;
use dpp::consensus::basic::token::contract_has_no_tokens_error::ContractHasNoTokensError;
use dpp::consensus::basic::token::InvalidTokenPositionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use dpp::validation::{SimpleConsensusValidationResult};
use drive::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(super) trait TokenBaseTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenBaseTransitionActionStructureValidationV0 for TokenBaseTransitionAction {
    fn validate_structure_v0(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let token_position = self.token_position();
        let contract = self.data_contract_fetch_info_ref();
        if contract.contract.tokens().get(&token_position).is_none() {
            return if contract.contract.tokens().is_empty() {
                Ok(SimpleConsensusValidationResult::new_with_error(
                    BasicError::ContractHasNoTokensError(ContractHasNoTokensError::new(
                        contract.contract.id(),
                    ))
                    .into(),
                ))
            } else {
                Ok(SimpleConsensusValidationResult::new_with_error(
                    BasicError::InvalidTokenPositionError(InvalidTokenPositionError::new(
                        *contract
                            .contract
                            .tokens()
                            .keys()
                            .last()
                            .expect("we already checked this was not empty"),
                        token_position,
                    ))
                    .into(),
                ))
            };
        }
        Ok(SimpleConsensusValidationResult::default())
    }
}
