pub mod v0;

pub use v0::*;

use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};
use crate::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionV0,
};
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::ProtocolError;
use platform_value::{Bytes32, Identifier};
use platform_version::version::PlatformVersion;

impl DataContractCreateTransitionMethodsV0 for DataContractCreateTransition {
    fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        entropy: Bytes32,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .contract_create_state_transition
                .default_current_version,
        ) {
            0 => DataContractCreateTransitionV0::new_from_data_contract(
                data_contract,
                entropy,
                identity,
                key_id,
                signer,
                platform_version,
                feature_version,
            ),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown DataContractCreateTransition version for new_from_data_contract {v}"
            ))),
        }
    }
}
