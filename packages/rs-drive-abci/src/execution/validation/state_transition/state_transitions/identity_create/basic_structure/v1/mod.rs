use crate::error::Error;
use crate::execution::validation::state_transition::identity_create::basic_structure::v0::IdentityCreateStateTransitionBasicStructureValidationV0;
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::basic::BasicError;
use dpp::identity::state_transition::AssetLockProved;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait IdentityCreateStateTransitionBasicStructureValidationV1
{
    fn validate_basic_structure_v1(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreateStateTransitionBasicStructureValidationV1 for IdentityCreateTransition {
    /// v0, then the identity id must be the one the asset lock's outpoint derives.
    ///
    /// The identity id is outside the bytes the asset lock key signs, and checking it reads
    /// only the transition, so it is checked here, before the asset lock proof is verified: a
    /// transition with a wrong identity id is refused without a fee and without that
    /// verification, in a block and in check_tx alike.
    fn validate_basic_structure_v1(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = self.validate_basic_structure_v0(platform_version)?;

        if !result.is_valid() {
            return Ok(result);
        }

        // v0 checked the asset lock proof's structure, so the outpoint exists and deriving the
        // identifier from it cannot fail
        if self.asset_lock_proof().create_identifier()? != self.identity_id() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                BasicError::InvalidIdentifierError(InvalidIdentifierError::new(
                    "identity_id".to_string(),
                    "does not match created identifier from asset lock".to_string(),
                ))
                .into(),
            ));
        }

        Ok(result)
    }
}
