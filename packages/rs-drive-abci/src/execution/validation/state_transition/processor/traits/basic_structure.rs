use crate::error::Error;
use dpp::dash_spv::Network;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionBasicStructureValidationV0 {
    /// Validates the structure of a transaction by checking its basic elements.
    ///
    /// # Arguments
    ///
    /// * `network_type` - The network we are on, mainnet/testnet/a devnet/a regtest.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn validate_basic_structure(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// True if the state transition has basic structure validation.
    /// Currently only data contract update does not
    fn has_basic_structure_validation(&self, _platform_version: &PlatformVersion) -> bool {
        true
    }
}

impl StateTransitionBasicStructureValidationV0 for StateTransition {
    fn validate_basic_structure(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::MasternodeVote(_) => {
                // no basic structure validation
                Ok(SimpleConsensusValidationResult::new())
            }
            StateTransition::IdentityCreate(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::IdentityUpdate(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::IdentityTopUp(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::Batch(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::IdentityCreditTransfer(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::DataContractCreate(st) => {
                if platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .contract_create_state_transition
                    .basic_structure
                    .is_some()
                {
                    st.validate_basic_structure(network_type, platform_version)
                } else {
                    Ok(SimpleConsensusValidationResult::new())
                }
            }
            StateTransition::DataContractUpdate(st) => {
                if platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .contract_update_state_transition
                    .basic_structure
                    .is_some()
                {
                    st.validate_basic_structure(network_type, platform_version)
                } else {
                    Ok(SimpleConsensusValidationResult::new())
                }
            }
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::IdentityCreateFromAddresses(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::IdentityTopUpFromAddresses(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::AddressFundsTransfer(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
        }
    }
    fn has_basic_structure_validation(&self, platform_version: &PlatformVersion) -> bool {
        match self {
            StateTransition::DataContractCreate(_) => {
                // Added in protocol version 9 (version 2.0)
                platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .contract_create_state_transition
                    .basic_structure
                    .is_some()
            }
            StateTransition::DataContractUpdate(_) => {
                // Added in protocol version 9  (version 2.0)
                platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .contract_update_state_transition
                    .basic_structure
                    .is_some()
            }
            StateTransition::Batch(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_) => true,
            StateTransition::MasternodeVote(_) => false,
        }
    }
}
