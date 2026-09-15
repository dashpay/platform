mod v0;

pub use v0::*;

use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, KeyID, PartialIdentity};
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV0, DataContractUpdateTransitionV1,
};
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::ProtocolError;

use crate::prelude::{IdentityNonce, UserFeeIncrease};
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransitionMethodsV0 for DataContractUpdateTransition {
    /// Always builds a full-contract (V0) transition: a lone contract can
    /// not become a delta. Callers holding the stored contract should use
    /// [`DataContractUpdateTransition::new_from_contract_update`], which
    /// picks the form the platform version defaults to.
    async fn new_from_data_contract<S: Signer<IdentityPublicKey>>(
        data_contract: DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match feature_version.unwrap_or(0) {
            0 => {
                DataContractUpdateTransitionV0::new_from_data_contract(
                    data_contract,
                    identity,
                    key_id,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    feature_version,
                )
                .await
            }
            1 => Err(ProtocolError::Generic(
                "a delta-based (V1) data contract update needs the stored contract, use new_from_contract_update"
                    .to_string(),
            )),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown DataContractUpdateTransition version for new_from_data_contract {v}"
            ))),
        }
    }
}

impl DataContractUpdateTransition {
    /// Builds and signs the update that turns `old_contract` into
    /// `new_contract`, in the form `feature_version` names or, when it is
    /// `None`, the form the platform version defaults to: a full-contract
    /// V0 transition, or a delta-based V1 one.
    #[allow(clippy::too_many_arguments)]
    pub async fn new_from_contract_update<S: Signer<IdentityPublicKey>>(
        old_contract: &DataContract,
        new_contract: &DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .contract_update_state_transition
                .default_current_version,
        ) {
            0 => {
                DataContractUpdateTransitionV0::new_from_data_contract(
                    new_contract.clone(),
                    identity,
                    key_id,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                    platform_version,
                    feature_version,
                )
                .await
            }
            1 => {
                DataContractUpdateTransitionV1::new_from_contract_update(
                    old_contract,
                    new_contract,
                    identity,
                    key_id,
                    identity_contract_nonce,
                    user_fee_increase,
                    signer,
                )
                .await
            }
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown DataContractUpdateTransition version for new_from_contract_update {v}"
            ))),
        }
    }
}
