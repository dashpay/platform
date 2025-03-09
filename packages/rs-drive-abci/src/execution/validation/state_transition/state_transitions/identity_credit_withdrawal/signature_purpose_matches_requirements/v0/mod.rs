use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::errors::consensus::basic::identity::WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{PartialIdentity, identity_public_key::Purpose};
use dpp::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(super) trait IdentityCreditWithdrawalStateTransitionSignaturePurposeMatchesRequirementsValidationV0
{
    fn validate_signature_purpose_matches_requirements_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditWithdrawalStateTransitionSignaturePurposeMatchesRequirementsValidationV0
    for IdentityCreditWithdrawalTransition
{
    fn validate_signature_purpose_matches_requirements_v0(
        &self,
        identity: &PartialIdentity,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        if let Some(output_script) = self.output_script() {
            let Some(signing_key) = identity
                .loaded_public_keys
                .get(&self.signature_public_key_id())
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "we should have a loaded key at this point",
                )));
            };

            if signing_key.purpose() == Purpose::OWNER {
                result.add_error(
                    WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError::new(
                        output_script.clone(),
                        signing_key.id(),
                    ),
                );
            }
        }

        Ok(result)
    }
}
