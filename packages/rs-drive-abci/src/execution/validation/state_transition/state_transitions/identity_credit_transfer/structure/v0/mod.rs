use dpp::consensus::basic::BasicError;
use dpp::consensus::basic::identity::InvalidIdentityCreditTransferAmountError;
use dpp::consensus::basic::value_error::ValueError;
use dpp::consensus::ConsensusError;
use dpp::ProtocolError;
// use dpp::platform_value::
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::identity_credit_transfer_transition::validation::basic::identity_credit_transfer_basic::IDENTITY_CREDIT_TRANSFER_TRANSITION_SCHEMA_VALIDATOR;
use dpp::validation::SimpleConsensusValidationResult;
use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_protocol_version::v0::validate_protocol_version_v0;
use crate::execution::validation::state_transition::common::validate_schema::v0::validate_schema_v0;

const MIN_TRANSFER_AMOUNT: u64 = 1000;

pub(crate) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for IdentityCreditTransferTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        if self.amount() < MIN_TRANSFER_AMOUNT {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidIdentityCreditTransferAmountError::new(self.amount(), MIN_TRANSFER_AMOUNT).into(),
            ));
        }

        Ok(validate_protocol_version_v0(self.protocol_version))
    }
}
