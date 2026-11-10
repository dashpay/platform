use crate::error::Error;
use dpp::dashcore::Network;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use super::v2::DataContractUpdateStateTransitionBasicStructureValidationV2;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionBasicStructureValidationV3
{
    fn validate_basic_structure_v3(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionBasicStructureValidationV3 for DataContractUpdateTransition {
    /// Generation 3 (protocol version 15): generation 2 for an update that embeds its
    /// contract, and a delta, which carries only what it changes, is admitted.
    ///
    /// A delta is refused here, without a fee, only when it names an entry in two sections
    /// that cannot both apply: the one thing a delta says about itself. Everything else is a
    /// rule about the contract the delta merges into, which exists only once the stored
    /// contract is read. State validation, and check_tx's transformation, merge the delta
    /// onto it and hold the update embedding the result to generation 2 of this validation,
    /// so a delta is held to every structure rule of an update that embeds its contract, in
    /// a block as a paid nonce bump.
    fn validate_basic_structure_v3(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let DataContractUpdateTransition::V1(delta) = self else {
            return self.validate_basic_structure_v2(network_type, platform_version);
        };

        Ok(match delta.overlapping_entry() {
            Some(error) => SimpleConsensusValidationResult::new_with_error(error.into()),
            None => SimpleConsensusValidationResult::new(),
        })
    }
}
