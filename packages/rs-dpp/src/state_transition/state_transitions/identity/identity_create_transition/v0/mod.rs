mod json_conversion;
mod state_transition_like;
mod types;
mod v0_methods;
mod value_conversion;

use std::convert::{TryFrom, TryInto};

use crate::platform_serialization::PlatformSignable;
use crate::serialization_traits::PlatformSerializable;
use crate::serialization_traits::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::{BinaryData, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::identity::signer::Signer;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::Identity;
use crate::identity::KeyType::ECDSA_HASH160;
use crate::prelude::Identifier;

use crate::state_transition::{StateTransitionConvert, StateTransitionLike, StateTransitionType};
use crate::version::FeatureVersion;
use crate::{BlsModule, NonConsensusError, ProtocolError};
use platform_value::btreemap_extensions::BTreeValueRemoveInnerValueFromMapHelper;
use crate::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreation;


#[derive(Debug, Copy, Clone, Default)]
pub struct SerializationOptions {
    pub skip_signature: bool,
    pub into_validating_json: bool,
}

#[derive(
Serialize,
Deserialize,
Debug,
Clone,
PartialEq,
Encode,
Decode,
PlatformDeserialize,
PlatformSerialize,
PlatformSignable,
)]
#[serde(rename_all = "camelCase")]
#[serde(try_from = "IdentityCreateTransitionV0Inner")]
#[platform_error_type(ProtocolError)]
pub struct IdentityCreateTransitionV0 {
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    // The signable
    #[platform_signable(into = "Vec<IdentityPublicKeyInCreationSignable>")]
    pub public_keys: Vec<IdentityPublicKeyInCreation>,
    pub asset_lock_proof: AssetLockProof,
    // Generic identity ST fields
    pub protocol_version: u32,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
    #[serde(skip)]
    #[platform_signable(exclude_from_sig_hash)]
    pub identity_id: Identifier,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreateTransitionV0Inner {
    #[serde(rename = "type")]
    transition_type: StateTransitionType,
    // Own ST fields
    public_keys: Vec<IdentityPublicKeyInCreation>,
    asset_lock_proof: AssetLockProof,
    // Generic identity ST fields
    protocol_version: u32,
    signature: BinaryData,
}

impl TryFrom<IdentityCreateTransitionV0Inner> for IdentityCreateTransitionV0 {
    type Error = ProtocolError;

    fn try_from(value: IdentityCreateTransitionV0Inner) -> Result<Self, Self::Error> {
        let IdentityCreateTransitionV0Inner {
            transition_type,
            public_keys,
            asset_lock_proof,
            protocol_version,
            signature,
        } = value;
        let identity_id = asset_lock_proof.create_identifier()?;
        Ok(Self {
            transition_type,
            public_keys,
            asset_lock_proof,
            protocol_version,
            signature,
            identity_id,
        })
    }
}

//todo: there shouldn't be a default
impl Default for IdentityCreateTransitionV0 {
    fn default() -> Self {
        Self {
            transition_type: StateTransitionType::IdentityCreate,
            public_keys: Default::default(),
            asset_lock_proof: Default::default(),
            identity_id: Default::default(),
            protocol_version: Default::default(),
            signature: Default::default(),
        }
    }
}

impl TryFrom<Identity> for IdentityCreateTransitionV0 {
    type Error = ProtocolError;

    fn try_from(identity: Identity) -> Result<Self, Self::Error> {
        let mut identity_create_transition = IdentityCreateTransitionV0::default();
        identity_create_transition.set_protocol_version(identity.feature_version as u32);

        let public_keys = identity
            .get_public_keys()
            .iter()
            .map(|(_, public_key)| public_key.into())
            .collect::<Vec<IdentityPublicKeyInCreation>>();
        identity_create_transition.set_public_keys(public_keys);

        let asset_lock_proof = identity.get_asset_lock_proof().ok_or_else(|| {
            ProtocolError::Generic(String::from("Asset lock proof is not present"))
        })?;

        identity_create_transition
            .set_asset_lock_proof(asset_lock_proof.to_owned())
            .map_err(ProtocolError::from)?;

        Ok(identity_create_transition)
    }
}

/// Main state transition functionality implementation
impl IdentityCreateTransitionV0 {
    pub fn try_from_identity_with_signer<S: Signer>(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        signer: &S,
        bls: &impl BlsModule,
    ) -> Result<Self, ProtocolError> {
        let mut identity_create_transition = IdentityCreateTransitionV0::default();
        identity_create_transition.set_protocol_version(identity.feature_version as u32);

        let public_keys = identity
            .get_public_keys()
            .iter()
            .map(|(_, public_key)| public_key.clone().into())
            .collect();
        identity_create_transition.set_public_keys(public_keys);

        identity_create_transition
            .set_asset_lock_proof(asset_lock_proof)
            .map_err(ProtocolError::from)?;

        let key_signable_bytes = identity_create_transition.signable_bytes()?;

        identity_create_transition
            .public_keys
            .iter_mut()
            .zip(identity.get_public_keys().iter())
            .try_for_each(|(public_key_with_witness, (_, public_key))| {
                if public_key.key_type.is_unique_key_type() {
                    let signature = signer.sign(public_key, &key_signable_bytes)?;
                    public_key_with_witness.signature = signature;
                }
                Ok::<(), ProtocolError>(())
            })?;

        identity_create_transition.sign_by_private_key(
            asset_lock_proof_private_key,
            ECDSA_HASH160,
            bls,
        )?;

        Ok(identity_create_transition)
    }

    /// Get State Transition type
    pub fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreate
    }

    /// Set asset lock
    pub fn set_asset_lock_proof(
        &mut self,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(), NonConsensusError> {
        self.identity_id = asset_lock_proof.create_identifier()?;

        self.asset_lock_proof = asset_lock_proof;

        Ok(())
    }

    /// Get asset lock proof
    pub fn get_asset_lock_proof(&self) -> &AssetLockProof {
        &self.asset_lock_proof
    }

    /// Get identity public keys
    pub fn get_public_keys(&self) -> &[IdentityPublicKeyInCreation] {
        &self.public_keys
    }

    /// Replaces existing set of public keys with a new one
    pub fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyInCreation>) -> &mut Self {
        self.public_keys = public_keys;

        self
    }

    /// Adds public keys to the existing public keys array
    pub fn add_public_keys(
        &mut self,
        public_keys: &mut Vec<IdentityPublicKeyInCreation>,
    ) -> &mut Self {
        self.public_keys.append(public_keys);

        self
    }

    /// Returns identity id
    pub fn get_identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    /// Returns Owner ID
    pub fn get_owner_id(&self) -> &Identifier {
        &self.identity_id
    }



    pub fn set_protocol_version(&mut self, protocol_version: u32) {
        self.protocol_version = protocol_version;
    }


}

impl StateTransitionConvert for IdentityCreateTransitionV0 {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![
            property_names::SIGNATURE,
            property_names::PUBLIC_KEYS_SIGNATURE,
        ]
    }
    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![property_names::IDENTITY_ID]
    }
    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }


}

impl StateTransitionLike for IdentityCreateTransitionV0 {
    /// Returns ids of created identities
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![*self.get_identity_id()]
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        self.protocol_version as FeatureVersion
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        StateTransitionType::IdentityCreate
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }
}
