use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::signing_key_limits::SigningKeyLimits;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::validate_state_transition_identity_signed::v0::ValidateStateTransitionIdentitySignatureV0;
use dpp::consensus::signature::{ContractBoundedKeyNonBatchError, PublicKeyBudgetExhaustedError};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
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
    /// v1 runs the v0 signature checks, then:
    ///
    /// * refuses a contract-bound AUTHENTICATION key on any transition other than a Batch: such
    ///   a key may only act inside its contract, and batch members are checked against the
    ///   bounds in batch advanced-structure validation;
    /// * for a signing key with a budget or an expiry, records its limits in the execution
    ///   context, and refuses it here when its budget is already spent. Whether the key has
    ///   expired, and whether what is left of the budget covers this transition, needs the block
    ///   time and the fee, so fee validation decides that from the recorded limits.
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

                if key.has_limits() {
                    let remaining_budget = if key.budget().is_some() {
                        // Priced like the retrieval of one more key of the identity.
                        execution_context.add_operation(ValidationOperation::RetrieveIdentity(
                            RetrieveIdentityInfo::one_key(),
                        ));
                        let remaining_budget = drive
                            .fetch_identity_key_remaining_budget(
                                identity.id.to_buffer(),
                                key.id(),
                                transaction,
                                platform_version,
                            )?
                            .ok_or_else(|| {
                                Error::Execution(ExecutionError::CorruptedDriveResponse(format!(
                                    "key {} of identity {} is budgeted but has no remaining budget in state",
                                    key.id(),
                                    identity.id
                                )))
                            })?;
                        if remaining_budget == 0 {
                            return Ok(ConsensusValidationResult::new_with_error(
                                PublicKeyBudgetExhaustedError::new(key.id()).into(),
                            ));
                        }
                        Some(remaining_budget)
                    } else {
                        None
                    };

                    execution_context.set_signing_key_limits(SigningKeyLimits {
                        key_id: key.id(),
                        expires_at: key.expires_at(),
                        remaining_budget,
                    });
                }
            }
        }
        Ok(result)
    }
}
