use crate::error::Error;
use dpp::identity::state_transition::AssetLockProved;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait IdentityCreateStateTransitionStructureValidationV0
{
    fn validate_base_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreateStateTransitionStructureValidationV0 for IdentityCreateTransition {
    fn validate_base_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = self
            .asset_lock_proof()
            .validate_structure(platform_version)?;

        if !result.is_valid() {
            return Ok(result);
        }

        IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            self.public_keys(),
            true,
            platform_version,
        )
        .map_err(Error::Protocol)
    }
}
