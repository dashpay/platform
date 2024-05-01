use dpp::consensus::basic::identity::{
    IdentityCreditTransferToSelfError, InvalidIdentityCreditTransferAmountError,
};

// use dpp::platform_value::
use crate::error::Error;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::validation::SimpleConsensusValidationResult;

const MIN_TRANSFER_AMOUNT: u64 = 100000;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_transfer) trait IdentityCreditTransferStateTransitionStructureValidationV0 {
    fn validate_basic_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditTransferStateTransitionStructureValidationV0
    for IdentityCreditTransferTransition
{
    fn validate_basic_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let result = SimpleConsensusValidationResult::new();

        if self.identity_id() == self.recipient_id() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityCreditTransferToSelfError::default().into(),
            ));
        }

        if self.amount() < MIN_TRANSFER_AMOUNT {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidIdentityCreditTransferAmountError::new(self.amount(), MIN_TRANSFER_AMOUNT)
                    .into(),
            ));
        }

        Ok(result)
    }
}
