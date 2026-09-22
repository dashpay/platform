use crate::drive::Drive;
use crate::error::Error;
use crate::query::ContractLookupFn;
use crate::verify::RootHash;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::proof_result::StateTransitionProofOutcome;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

impl Drive {
    /// Version 1: the proof of an owned, fee-paying transition carries the
    /// owner's credit balance next to its result, and the outcome carries it.
    /// Every other transition verifies as in version 0.
    pub(super) fn verify_state_transition_was_executed_with_proof_v1(
        state_transition: &StateTransition,
        block_info: &BlockInfo,
        proof: &[u8],
        known_contracts_provider_fn: &ContractLookupFn,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StateTransitionProofOutcome), Error> {
        Self::verify_state_transition_was_executed_with_proof_internal(
            state_transition,
            block_info,
            proof,
            known_contracts_provider_fn,
            true,
            platform_version,
        )
    }
}
