use dpp::consensus::basic::identity::InvalidIdentityCreditTransferAmountError;
use dpp::consensus::basic::value_error::ValueError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::ProtocolError;
// use dpp::platform_value::
use crate::error::Error;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::validation::SimpleConsensusValidationResult;

const MIN_TRANSFER_AMOUNT: u64 = 1000;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_transfer) trait IdentityCreditTransferStateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditTransferStateTransitionStructureValidationV0
    for IdentityCreditTransferTransition
{
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::new();

        if self.amount() < MIN_TRANSFER_AMOUNT {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidIdentityCreditTransferAmountError::new(self.amount(), MIN_TRANSFER_AMOUNT)
                    .into(),
            ));
        }

        Ok(result)
    }
}
