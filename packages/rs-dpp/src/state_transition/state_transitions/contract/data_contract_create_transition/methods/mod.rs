pub mod v0;
pub mod v1;

pub use v0::*;
pub use v1::*;

use crate::consensus::signature::{InvalidSignaturePublicKeySecurityLevelError, SignatureError};
use crate::contract_group::{ContractGroupMembership, ContractGroupRegistration};
use crate::data_contract::DataContract;
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, KeyID, PartialIdentity};
use crate::prelude::IdentityNonce;
use crate::serialization::Signable;
use crate::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionV0, DataContractCreateTransitionV1,
};
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::NonConsensusError;
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

/// Signs a freshly built data contract create transition with `key_id` of `identity`, after
/// checking the key meets the transition's security level requirement. Every version's
/// constructor ends here, so the signing rules cannot drift between versions.
pub(super) async fn sign_new_transition<S: Signer<IdentityPublicKey>>(
    mut state_transition: StateTransition,
    identity: &PartialIdentity,
    key_id: KeyID,
    signer: &S,
) -> Result<StateTransition, ProtocolError> {
    let value = state_transition.signable_bytes()?;

    // The public key ids don't always match the keys in the map, so look the key up by id.
    let public_key = identity
        .loaded_public_keys
        .values()
        .find(|public_key| public_key.id() == key_id)
        .ok_or(ProtocolError::NonConsensusError(
            NonConsensusError::StateTransitionCreationError("public key did not exist".to_string()),
        ))?;

    let security_level_requirements = state_transition
        .security_level_requirement(public_key.purpose())
        .ok_or(ProtocolError::CorruptedCodeExecution(
            "expected security level requirements".to_string(),
        ))?;

    if !security_level_requirements.contains(&public_key.security_level()) {
        return Err(ProtocolError::ConsensusError(Box::new(
            SignatureError::InvalidSignaturePublicKeySecurityLevelError(
                InvalidSignaturePublicKeySecurityLevelError::new(
                    public_key.security_level(),
                    security_level_requirements,
                ),
            )
            .into(),
        )));
    }

    let signature = signer.sign(public_key, &value).await?;
    state_transition.set_signature(signature);

    Ok(state_transition)
}
