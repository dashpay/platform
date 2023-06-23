use dpp::identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::identity::state_transition::identity_credit_transfer_transition::validation::basic::identity_credit_transfer_basic::IDENTITY_CREDIT_TRANSFER_TRANSITION_SCHEMA_VALIDATOR;
use dpp::validation::SimpleConsensusValidationResult;
use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_protocol_version::v0::validate_protocol_version_v0;
use crate::execution::validation::state_transition::common::validate_schema::v0::validate_schema_v0;

pub(crate) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for IdentityCreditTransferTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let result =
            validate_schema_v0(&IDENTITY_CREDIT_TRANSFER_TRANSITION_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(validate_protocol_version_v0(self.protocol_version))
    }
}
