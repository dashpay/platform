use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;

use crate::{data_contract::DataContract, identity::KeyID, NonConsensusError, ProtocolError};

use crate::serialization::Signable;

use crate::consensus::signature::{InvalidSignaturePublicKeySecurityLevelError, SignatureError};
use crate::data_contract::accessors::v0::DataContractV0Setters;
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::signer::Signer;
use crate::identity::PartialIdentity;
use crate::prelude::IdentityNonce;
use crate::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use platform_version::version::PlatformVersion;
use platform_version::TryIntoPlatformVersioned;

use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;

impl DataContractCreateTransitionMethodsV0 for DataContractCreateTransitionV0 {
    fn new_from_data_contract<S: Signer>(
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

        let mut state_transition: StateTransition = transition.into();
        let value = state_transition.signable_bytes()?;

        // The public key ids don't always match the keys in the map, so we need to do this.
        let matching_key = identity
            .loaded_public_keys
            .iter()
            .find_map(|(&key, public_key)| {
                if public_key.id() == key_id {
                    Some(key)
                } else {
                    None
                }
            })
            .expect("No matching public key id found in the map");

        let public_key = identity.loaded_public_keys.get(&matching_key).ok_or(
            ProtocolError::NonConsensusError(NonConsensusError::StateTransitionCreationError(
                "public key did not exist".to_string(),
            )),
        )?;

        let security_level_requirements = state_transition.security_level_requirement().ok_or(
            ProtocolError::CorruptedCodeExecution(
                "expected security level requirements".to_string(),
            ),
        )?;

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

        // There was an error here where the public key supplied was not one belonging to the signer.
        match signer.sign(public_key, &value) {
            Ok(signature) => {
                state_transition.set_signature(signature);
            }
            Err(e) => {
                return Err(e);
            }
        }

        Ok(state_transition)
    }
}
