use crate::error::Error;
use dpp::consensus::basic::identity::{
    IdentityKeyLimitsUpdateEmptyError, InvalidIdentityPublicKeyBudgetError,
};
use dpp::consensus::ConsensusError;
use dpp::state_transition::identity_key_limits_update_transition::accessors::IdentityKeyLimitsUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::identity_key_limits_update) trait IdentityKeyLimitsUpdateStateTransitionStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityKeyLimitsUpdateStateTransitionStructureValidationV0
    for IdentityKeyLimitsUpdateTransition
{
    /// The transition must change something, and a budget it sets must let the key spend
    /// something. Whether the key has these limits, and whether the values raise them, needs
    /// the stored key, so state validation decides that.
    fn validate_basic_structure_v0(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        if self.total_budget().is_none() && self.expires_at().is_none() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::from(IdentityKeyLimitsUpdateEmptyError::new(self.key_id())),
            ));
        }

        if self.total_budget() == Some(0) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::from(InvalidIdentityPublicKeyBudgetError::new(self.key_id())),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
