use crate::error::Error;
use dpp::identity::state_transition::AssetLockProved;

use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::identity_top_up) trait IdentityTopUpStateTransitionStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityTopUpStateTransitionStructureValidationV0 for IdentityTopUpTransition {
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        self.asset_lock_proof()
            .validate_structure(platform_version)
            .map_err(Error::Protocol)
    }
}
