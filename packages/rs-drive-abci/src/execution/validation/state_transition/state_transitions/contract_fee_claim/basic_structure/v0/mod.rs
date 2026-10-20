use crate::error::Error;
use dpp::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::contract_fee_claim) trait ContractFeeClaimStateTransitionStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl ContractFeeClaimStateTransitionStructureValidationV0 for ContractFeeClaimTransition {
    /// A claim names a contract and one of two pots, and a transition naming anything else does
    /// not decode, so there is nothing about its structure left to refuse. Who may claim the
    /// pot, whether it was already claimed this epoch and whether it holds anything all need
    /// state, so state validation decides those.
    fn validate_basic_structure_v0(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        Ok(SimpleConsensusValidationResult::new())
    }
}
