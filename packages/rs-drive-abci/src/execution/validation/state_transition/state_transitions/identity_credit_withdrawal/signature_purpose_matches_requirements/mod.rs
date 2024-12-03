pub(crate) mod v0;

use crate::error::Error;
use dpp::identity::PartialIdentity;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::identity_credit_withdrawal::signature_purpose_matches_requirements::v0::IdentityCreditWithdrawalStateTransitionSignaturePurposeMatchesRequirementsValidationV0;

pub(in crate::execution::validation::state_transition) trait IdentityCreditWithdrawalStateTransitionSignaturePurposeMatchesRequirementsValidation
{
    fn validate_signature_purpose_matches_requirements(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditWithdrawalStateTransitionSignaturePurposeMatchesRequirementsValidation
    for IdentityCreditWithdrawalTransition
{
    fn validate_signature_purpose_matches_requirements(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_withdrawal_state_transition_purpose_matches_requirements
        {
            0 => self.validate_signature_purpose_matches_requirements_v0(
                identity,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit withdrawal transition: validate_signature_purpose_matches_requirements".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
