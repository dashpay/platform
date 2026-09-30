use crate::drive::Drive;
use crate::error::Error;
use crate::prove::prove_state_transition::ProofCreationResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 1: the proof of an owned, fee-paying transition (document and
    /// token batches, contract creates and updates, identity updates and key
    /// limit updates, contract moderation) carries the owner's credit balance
    /// next to its result, in one merged query. Every other transition proves
    /// as in version 0.
    pub(super) fn prove_state_transition_v1(
        &self,
        state_transition: &StateTransition,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ProofCreationResult<Vec<u8>>, Error> {
        self.prove_state_transition_internal(state_transition, transaction, true, platform_version)
    }
}
