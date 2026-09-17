use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;

use crate::{data_contract::DataContract, identity::KeyID, ProtocolError};

use crate::data_contract::accessors::v0::DataContractV0Setters;
use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, PartialIdentity};
use crate::prelude::IdentityNonce;
use crate::state_transition::data_contract_create_transition::methods::{
    sign_new_transition, DataContractCreateTransitionMethodsV0,
};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use platform_version::version::PlatformVersion;
use platform_version::TryIntoPlatformVersioned;

use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;

impl DataContractCreateTransitionMethodsV0 for DataContractCreateTransitionV0 {
    async fn new_from_data_contract<S: Signer<IdentityPublicKey>>(
        mut data_contract: DataContract,
        identity_nonce: IdentityNonce,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        platform_version: &PlatformVersion,
        _feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        data_contract.set_id(DataContract::generate_data_contract_id_v0(
            identity.id,
            identity_nonce,
        ));

        data_contract.set_owner_id(identity.id);

        let transition = DataContractCreateTransition::V0(DataContractCreateTransitionV0 {
            data_contract: data_contract.try_into_platform_versioned(platform_version)?,
            identity_nonce,
            user_fee_increase: 0,
            signature_public_key_id: key_id,
            signature: Default::default(),
        });

        sign_new_transition(transition.into(), identity, key_id, signer).await
    }
}
