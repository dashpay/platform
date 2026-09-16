use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_state_transition_identity_signed::v0::ValidateStateTransitionIdentitySignatureV0;
use dpp::consensus::signature::ContractBoundedKeyNonBatchError;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{PartialIdentity, Purpose};
use dpp::state_transition::StateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(super) trait ValidateStateTransitionIdentitySignatureV1 {
    fn validate_state_transition_identity_signed_v1(
        &self,
        drive: &Drive,
        request_balance: bool,
        request_revision: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;
}

impl ValidateStateTransitionIdentitySignatureV1 for StateTransition {
    /// v1 runs the v0 signature checks, then refuses a contract-bound AUTHENTICATION key on any
    /// transition other than a Batch: such a key may only act inside its contract, and batch
    /// members are checked against the bounds in batch advanced-structure validation.
    fn validate_state_transition_identity_signed_v1(
        &self,
        drive: &Drive,
        request_balance: bool,
        request_revision: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        let result = self.validate_state_transition_identity_signed_v0(
            drive,
            request_balance,
            request_revision,
            transaction,
            execution_context,
            platform_version,
        )?;
        if let Some(identity) = result.data.as_ref().filter(|_| result.is_valid()) {
            // v0 loads exactly the signing key, but look it up by id so another loaded key can
            // never veto a transition it did not sign.
            let signing_key = self
                .signature_public_key_id()
                .and_then(|key_id| identity.loaded_public_keys.get(&key_id));
            if let Some(key) = signing_key {
                if key.purpose() == Purpose::AUTHENTICATION
                    && key.contract_bounds().is_some()
                    && !matches!(self, StateTransition::Batch(_))
                {
                    return Ok(ConsensusValidationResult::new_with_error(
                        ContractBoundedKeyNonBatchError::new(key.id()).into(),
                    ));
                }
            }
        }
        Ok(result)
    }
}
