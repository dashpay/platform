use crate::error::Error;
use dpp::consensus::basic::contract_moderation::{
    ContractModerationSelfTargetError, InvalidContractModerationDocumentFieldsError,
};
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
    /// An identity can not moderate itself, a suspension ends within the JSON-safe range, the
    /// text of the reason a ban, a suspension, a warning, a document deletion, a field change or
    /// the proposal of a settled document's deletion carries fits `SystemLimits::max_contract_moderation_reason_length` (the reason's code is
    /// not checked), and a field change names at least one field, none of them a system
    /// property. A document deletion or field change names a document, not an identity: whose
    /// it is, and so whether the moderator may act on it, is only known once the state is read.
    /// Whether the contract keeps the list, who may
    /// moderate it and what the target's status is all need state, so state validation
    /// decides those.
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        if self.target_identity_id() == Some(self.owner_id()) {
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

        if let Some(reason) = self.action().reason() {
            let result = reason.validate(platform_version);
            if !result.is_valid() {
                return Ok(result);
            }
        }

        // A field change names at least one field, and only fields a document type can keep for
        // its moderators: top-level schema properties, never a system one, which the platform
        // manages. Whether the type keeps each one needs the contract, so state decides that.
        if let Some((_, _, fields)) = self.action().changed_document() {
            let refusal = if fields.is_empty() {
                Some("it names no field".to_string())
            } else {
                fields
                    .keys()
                    .find(|field| field.is_empty() || field.starts_with('$'))
                    .map(|field| format!("\"{field}\" is no property a moderator can write"))
            };
            if let Some(message) = refusal {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    InvalidContractModerationDocumentFieldsError::new(message).into(),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
