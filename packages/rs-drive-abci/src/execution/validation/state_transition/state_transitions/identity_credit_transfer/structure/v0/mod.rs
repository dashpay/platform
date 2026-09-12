use crate::error::Error;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::validation::SimpleConsensusValidationResult;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_transfer) trait IdentityCreditTransferStateTransitionStructureValidationV0 {
    fn validate_basic_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditTransferStateTransitionStructureValidationV0
    for IdentityCreditTransferTransition
{
    fn validate_basic_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        // Delegate to the DPP-owned shared rule so client-side constructors
        // and drive-abci enforce identical self-transfer and minimum-amount
        // checks from a single source of truth.
        Ok(IdentityCreditTransferTransition::validate_basic_structure_v0(self))
    }
}
