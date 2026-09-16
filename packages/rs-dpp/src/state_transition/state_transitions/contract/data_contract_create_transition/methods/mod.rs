pub mod v0;
pub mod v1;

pub use v0::*;
pub use v1::*;

use crate::contract_group::{ContractGroupMembership, ContractGroupRegistration};
use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, KeyID, PartialIdentity};
use crate::prelude::IdentityNonce;
use crate::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionV0, DataContractCreateTransitionV1,
};
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractCreateTransitionMethodsV0 for DataContractCreateTransition {
    async fn new_from_data_contract<S: Signer<IdentityPublicKey>>(
        data_contract: DataContract,
        identity_nonce: IdentityNonce,
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
            0 => {
                DataContractCreateTransitionV0::new_from_data_contract(
                    data_contract,
                    identity_nonce,
                    identity,
                    key_id,
                    signer,
                    platform_version,
                    feature_version,
                )
                .await
            }
            1 => {
                DataContractCreateTransitionV1::new_from_data_contract(
                    data_contract,
                    identity_nonce,
                    identity,
                    key_id,
                    signer,
                    platform_version,
                    feature_version,
                )
                .await
            }
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown DataContractCreateTransition version for new_from_data_contract {v}"
            ))),
        }
    }
}

impl DataContractCreateTransitionMethodsV1 for DataContractCreateTransition {
    async fn new_from_data_contract_with_contract_group<S: Signer<IdentityPublicKey>>(
        data_contract: DataContract,
        identity_nonce: IdentityNonce,
        contract_group: Option<ContractGroupRegistration>,
        contract_group_memberships: Vec<ContractGroupMembership>,
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
            0 => Err(ProtocolError::UnknownVersionError(
                "DataContractCreateTransition version 0 cannot carry contract groups; \
                 they arrive with version 1 (protocol version 14)"
                    .to_string(),
            )),
            1 => {
                DataContractCreateTransitionV1::new_from_data_contract_with_contract_group(
                    data_contract,
                    identity_nonce,
                    contract_group,
                    contract_group_memberships,
                    identity,
                    key_id,
                    signer,
                    platform_version,
                    feature_version,
                )
                .await
            }
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown DataContractCreateTransition version for \
                 new_from_data_contract_with_contract_group {v}"
            ))),
        }
    }
}
