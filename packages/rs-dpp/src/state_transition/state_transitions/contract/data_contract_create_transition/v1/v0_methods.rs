use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV1;

use crate::{data_contract::DataContract, identity::KeyID, NonConsensusError, ProtocolError};

use crate::serialization::Signable;

use crate::consensus::signature::{InvalidSignaturePublicKeySecurityLevelError, SignatureError};
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, PartialIdentity};
use crate::prelude::IdentityNonce;
use crate::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use platform_version::version::PlatformVersion;

impl DataContractCreateTransitionMethodsV0 for DataContractCreateTransitionV1 {
    fn new_from_data_contract<S: Signer<IdentityPublicKey>>(
        data_contract: DataContract,
        identity_nonce: IdentityNonce,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        _platform_version: &PlatformVersion,
        _feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        // Note: For V1, we don't set id/owner_id on the contract because we store
        // the fields directly. The id is derived from owner_id + identity_nonce.

        // Derive contract_system_version from the DataContract variant
        let contract_system_version = match &data_contract {
            DataContract::V0(_) => 0,
            DataContract::V1(_) => 1,
        };

        let transition = DataContractCreateTransition::V1(DataContractCreateTransitionV1 {
            contract_system_version,
            owner_id: identity.id,
            config: data_contract.config().clone(),
            schema_defs: data_contract.schema_defs().cloned(),
            document_schemas: data_contract
                .document_schemas()
                .into_iter()
                .map(|(k, v)| (k, v.clone()))
                .collect(),
            groups: data_contract.groups().clone(),
            tokens: data_contract.tokens().clone(),
            keywords: data_contract.keywords().clone(),
            description: data_contract.description().cloned(),
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
