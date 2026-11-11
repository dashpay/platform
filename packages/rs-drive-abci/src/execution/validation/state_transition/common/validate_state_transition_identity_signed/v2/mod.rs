use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_state_transition_identity_signed::v1::ValidateStateTransitionIdentitySignatureV1;
use dpp::consensus::signature::PublicKeyWithLimitsCannotSignDataContractTransitionError;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::PartialIdentity;
use dpp::state_transition::StateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(super) trait ValidateStateTransitionIdentitySignatureV2 {
    fn validate_state_transition_identity_signed_v2(
        &self,
        drive: &Drive,
        request_balance: bool,
        request_revision: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;
}

impl ValidateStateTransitionIdentitySignatureV2 for StateTransition {
    /// v2 runs the v1 checks, then refuses a signing key with a budget or an expiry on a
    /// `DataContractCreate` and on a `DataContractUpdate`.
    ///
    /// A budget counts what the transitions the key signs take from the identity, and an expiry
    /// ends the key's signing, while a data contract commits its owner beyond the transition
    /// that sets it up or changes it. So a contract is something only a key without limits may
    /// set up or change, whatever the contract says.
    ///
    /// Like every signature refusal it is unpaid: the identity balance, the key budget and the
    /// nonce stay as they are. A key v1 already refuses (a spent budget) keeps v1's answer.
    fn validate_state_transition_identity_signed_v2(
        &self,
        drive: &Drive,
        request_balance: bool,
        request_revision: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        let result = self.validate_state_transition_identity_signed_v1(
            drive,
            request_balance,
            request_revision,
            transaction,
            execution_context,
            platform_version,
        )?;
        if !matches!(
            self,
            StateTransition::DataContractCreate(_) | StateTransition::DataContractUpdate(_)
        ) {
            return Ok(result);
        }
        // Looked up by id, as v1 does, so that only the key that signed is judged.
        let limited_signing_key = result
            .data
            .as_ref()
            .filter(|_| result.is_valid())
            .and_then(|identity| {
                self.signature_public_key_id()
                    .and_then(|key_id| identity.loaded_public_keys.get(&key_id))
            })
            .filter(|key| key.has_limits());
        if let Some(key) = limited_signing_key {
            return Ok(ConsensusValidationResult::new_with_error(
                PublicKeyWithLimitsCannotSignDataContractTransitionError::new(key.id()).into(),
            ));
        }
        Ok(result)
    }
}
