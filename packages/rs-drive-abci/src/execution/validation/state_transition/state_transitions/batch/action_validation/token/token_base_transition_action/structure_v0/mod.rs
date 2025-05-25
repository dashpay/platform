use dpp::errors::consensus::basic::BasicError;
use dpp::errors::consensus::basic::token::ContractHasNoTokensError;
use dpp::errors::consensus::basic::token::InvalidTokenPositionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::validation::{SimpleConsensusValidationResult};
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenBaseTransitionActionStructureValidationV0 {
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
                        contract.contract.tokens().keys().last().copied(),
                        token_position,
                    ))
                    .into(),
                ))
            };
        }
        Ok(SimpleConsensusValidationResult::default())
    }
}
