use crate::error::Error;
use dpp::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use dpp::consensus::state::state_error::StateError;
use dpp::identity::state_transition::AssetLockProved;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait IdentityCreateStateTransitionBasicStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreateStateTransitionBasicStructureValidationV0 for IdentityCreateTransition {
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = self
            .asset_lock_proof()
            .validate_structure(platform_version)?;

        if !result.is_valid() {
            return Ok(result);
        }

        if self.public_keys().len()
            > platform_version
                .dpp
                .state_transitions
                .identities
                .max_public_keys_in_creation as usize
        {
            Ok(SimpleConsensusValidationResult::new_with_error(
                StateError::MaxIdentityPublicKeyLimitReachedError(
                    MaxIdentityPublicKeyLimitReachedError::new(
                        platform_version
                            .dpp
                            .state_transitions
                            .identities
                            .max_public_keys_in_creation as usize,
                    ),
                )
                .into(),
            ))
        } else {
            Ok(SimpleConsensusValidationResult::new())
        }
    }
}
