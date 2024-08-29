use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};
use crate::serialization::Signable;

use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV0,
};
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::{NonConsensusError, ProtocolError};
use platform_version::version::PlatformVersion;
use platform_version::TryIntoPlatformVersioned;

impl DataContractUpdateTransitionMethodsV0 for DataContractUpdateTransitionV0 {
    fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        _feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let transition = DataContractUpdateTransition::V0(DataContractUpdateTransitionV0 {
            identity_contract_nonce,
            data_contract: data_contract.try_into_platform_versioned(platform_version)?,
            user_fee_increase,
            signature_public_key_id: key_id,
            signature: Default::default(),
        });

        let mut state_transition: StateTransition = transition.into();
        let value = state_transition.signable_bytes()?;
        let public_key =
            identity
                .loaded_public_keys
                .get(&key_id)
                .ok_or(ProtocolError::NonConsensusError(
                    NonConsensusError::StateTransitionCreationError(
                        "public key did not exist".to_string(),
                    ),
                ))?;
        state_transition.set_signature(signer.sign(public_key, &value)?);
        Ok(state_transition)
    }
}
