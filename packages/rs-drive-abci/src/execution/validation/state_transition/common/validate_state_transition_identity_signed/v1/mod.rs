use dpp::identity::PartialIdentity;
use dpp::state_transition::StateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::{PlatformVersion};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_state_transition_identity_signed::v0::ValidateStateTransitionIdentitySignatureV0;

pub(super) trait ValidateStateTransitionIdentitySignatureV1 {
    #[allow(clippy::too_many_arguments)] // Keep explicit versioned validation inputs.
    fn validate_state_transition_identity_signed_v1(
        &self,
        drive: &Drive,
        time_ms: u64,
        request_balance: bool,
        request_revision: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;
}
impl ValidateStateTransitionIdentitySignatureV1 for StateTransition {
    fn validate_state_transition_identity_signed_v1(
        &self,
        drive: &Drive,
        time_ms: u64,
        request_balance: bool,
        request_revision: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        use dpp::identity::contract_bounds::ContractBounds;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        let result = self.validate_state_transition_identity_signed_v0(
            drive,
            request_balance,
            request_revision,
            transaction,
            execution_context,
            platform_version,
        )?;
        if let Some(identity) = result.data.as_ref().filter(|_| result.is_valid()) {
            for key in identity.loaded_public_keys.values() {
                if let Some(ContractBounds::Scoped(scope)) = key.contract_bounds() {
                    if scope.is_expired(time_ms) {
                        return Ok(ConsensusValidationResult::new_with_error(
                            dpp::consensus::signature::ScopedKeyExpiredError::new(key.id()).into(),
                        ));
                    }
                    if !matches!(self, StateTransition::Batch(_)) {
                        return Ok(ConsensusValidationResult::new_with_error(
                            dpp::consensus::signature::ScopedKeyNonBatchError::new(key.id()).into(),
                        ));
                    }
                }
            }
        }
        Ok(result)
    }
}
