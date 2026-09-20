use crate::error::Error;
use dpp::dashcore::Network;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use super::v1::DataContractUpdateStateTransitionBasicStructureValidationV1;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionBasicStructureValidationV2
{
    fn validate_basic_structure_v2(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionBasicStructureValidationV2 for DataContractUpdateTransition {
    /// Generation 2 (protocol version 14): generation 1, and a config that declares
    /// moderation must be well formed (at least one list, a non-empty moderator set within
    /// the limit). Whether a list may be turned off is judged against the stored contract by
    /// the state validation's config update rules, and that a newly named moderator exists
    /// by the state validation too.
    fn validate_basic_structure_v2(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let v1_result = self.validate_basic_structure_v1(network_type, platform_version)?;
        if !v1_result.is_valid() {
            return Ok(v1_result);
        }

        if let Some(moderation) = self.data_contract().config().moderation() {
            let result = moderation.validate(platform_version)?;
            if !result.is_valid() {
                return Ok(result);
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
