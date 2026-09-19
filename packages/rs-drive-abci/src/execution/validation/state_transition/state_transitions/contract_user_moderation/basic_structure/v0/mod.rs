use crate::error::Error;
use dpp::consensus::basic::contract_moderation::ContractModerationSelfTargetError;
use dpp::consensus::basic::overflow_error::OverflowError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationTransition;
use dpp::state_transition::StateTransitionOwned;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::contract_user_moderation) trait ContractUserModerationStateTransitionStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl ContractUserModerationStateTransitionStructureValidationV0
    for ContractUserModerationTransition
{
    /// An identity can not moderate itself, and a suspension ends within the JSON-safe range.
    /// Whether the contract keeps the list, who may
    /// moderate it and what the target's status is all need state, so state validation
    /// decides those.
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        if self.target_identity_id() == self.owner_id() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::from(ContractModerationSelfTargetError::new(self.owner_id())),
            ));
        }

        // `until` reaches clients as a JSON number and as a protobuf uint64 JavaScript reads as
        // a number, so a value past 2^53 - 1 would be rounded on the way.
        let max_until = platform_version.system_limits.max_contract_suspension_until;
        if let Some(until) = self.action().until().filter(|until| *until > max_until) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                OverflowError::new(format!(
                    "suspension end {} exceeds the latest allowed block time {}",
                    until, max_until
                ))
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
