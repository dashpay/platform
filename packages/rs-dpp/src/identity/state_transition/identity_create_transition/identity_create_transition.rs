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
use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreationWithWitness;
use crate::identity::Identity;
use crate::identity::KeyType::ECDSA_HASH160;
use crate::prelude::Identifier;

use crate::state_transition::{StateTransitionConvert, StateTransitionLike, StateTransitionType};
use crate::{BlsModule, NonConsensusError, ProtocolError};
use platform_value::btreemap_extensions::BTreeValueRemoveInnerValueFromMapHelper;

pub const IDENTIFIER_FIELDS: [&str; 1] = [property_names::IDENTITY_ID];
pub const BINARY_FIELDS: [&str; 3] = [
    property_names::PUBLIC_KEYS_DATA,
    property_names::PUBLIC_KEYS_SIGNATURE,
    property_names::SIGNATURE,
];
pub const U32_FIELDS: [&str; 1] = [property_names::PROTOCOL_VERSION];

mod property_names {
    pub const PUBLIC_KEYS: &str = "publicKeys";
    pub const PUBLIC_KEYS_DATA: &str = "publicKeys[].data";
    pub const PUBLIC_KEYS_SIGNATURE: &str = "publicKeys[].signature";
    pub const ASSET_LOCK_PROOF: &str = "assetLockProof";
    pub const SIGNATURE: &str = "signature";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TRANSITION_TYPE: &str = "type";
    pub const IDENTITY_ID: &str = "identityId";
}

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
#[serde(try_from = "IdentityCreateTransitionInner")]
#[platform_error_type(ProtocolError)]
pub struct IdentityCreateTransition {
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    // Own ST fields
    pub public_keys: Vec<IdentityPublicKeyInCreationWithWitness>,
    pub asset_lock_proof: AssetLockProof,
    // Generic identity ST fields
    pub protocol_version: u32,
    #[exclude_from_sig_hash]
    pub signature: BinaryData,
    #[serde(skip)]
    #[exclude_from_sig_hash]
    pub identity_id: Identifier,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreateTransitionInner {
    #[serde(rename = "type")]
    transition_type: StateTransitionType,
    // Own ST fields
    public_keys: Vec<IdentityPublicKeyInCreationWithWitness>,
    asset_lock_proof: AssetLockProof,
    // Generic identity ST fields
    protocol_version: u32,
    signature: BinaryData,
}

impl TryFrom<IdentityCreateTransitionInner> for IdentityCreateTransition {
    type Error = ProtocolError;

    fn try_from(value: IdentityCreateTransitionInner) -> Result<Self, Self::Error> {
        let IdentityCreateTransitionInner {
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
impl Default for IdentityCreateTransition {
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

impl TryFrom<Identity> for IdentityCreateTransition {
    type Error = ProtocolError;

    fn try_from(identity: Identity) -> Result<Self, Self::Error> {
        let mut identity_create_transition = IdentityCreateTransition::default();
        identity_create_transition.set_protocol_version(identity.protocol_version);

        let public_keys = identity
            .get_public_keys()
            .iter()
            .map(|(_, public_key)| public_key.into())
            .collect::<Vec<IdentityPublicKeyInCreationWithWitness>>();
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
impl IdentityCreateTransition {
    pub fn try_from_identity_with_signer<S: Signer>(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        signer: &S,
        bls: &impl BlsModule,
    ) -> Result<Self, ProtocolError> {
        let mut identity_create_transition = IdentityCreateTransition::default();
        identity_create_transition.set_protocol_version(identity.protocol_version);

        let public_keys = identity
            .get_public_keys()
            .iter()
            .map(|(_, public_key)| {
                IdentityPublicKeyInCreationWithWitness::from_public_key_signed_external(
                    public_key.clone(),
                    signer,
                )
            })
            .collect::<Result<Vec<IdentityPublicKeyInCreationWithWitness>, ProtocolError>>()?;
        identity_create_transition.set_public_keys(public_keys);

        identity_create_transition
            .set_asset_lock_proof(asset_lock_proof)
            .map_err(ProtocolError::from)?;

        identity_create_transition.sign_by_private_key(
            asset_lock_proof_private_key,
            ECDSA_HASH160,
            bls,
        )?;

        Ok(identity_create_transition)
    }

    pub fn from_raw_object(raw_object: Value) -> Result<Self, ProtocolError> {
        let mut state_transition = Self::default();

        let mut transition_map = raw_object
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        if let Some(keys_value_array) = transition_map
            .remove_optional_inner_value_array::<Vec<_>>(property_names::PUBLIC_KEYS)
            .map_err(ProtocolError::ValueError)?
        {
            let keys = keys_value_array
                .into_iter()
                .map(|val| val.try_into().map_err(ProtocolError::ValueError))
                .collect::<Result<Vec<IdentityPublicKeyInCreationWithWitness>, ProtocolError>>()?;
            state_transition.set_public_keys(keys);
        }

        if let Some(proof) = transition_map.get(property_names::ASSET_LOCK_PROOF) {
            state_transition.set_asset_lock_proof(AssetLockProof::try_from(proof)?)?;
        }

        state_transition.protocol_version =
            transition_map.get_integer(property_names::PROTOCOL_VERSION)?;

        Ok(state_transition)
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
    pub fn get_public_keys(&self) -> &[IdentityPublicKeyInCreationWithWitness] {
        &self.public_keys
    }

    /// Replaces existing set of public keys with a new one
    pub fn set_public_keys(
        &mut self,
        public_keys: Vec<IdentityPublicKeyInCreationWithWitness>,
    ) -> &mut Self {
        self.public_keys = public_keys;

        self
    }

    /// Adds public keys to the existing public keys array
    pub fn add_public_keys(
        &mut self,
        public_keys: &mut Vec<IdentityPublicKeyInCreationWithWitness>,
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

    /// Get raw state transition
    pub fn to_json_object(
        &self,
        options: SerializationOptions,
    ) -> Result<JsonValue, ProtocolError> {
        if options.into_validating_json {
            self.to_object(options.skip_signature)?
                .try_into_validating_json()
                .map_err(ProtocolError::ValueError)
        } else {
            self.to_object(options.skip_signature)?
                .try_into()
                .map_err(ProtocolError::ValueError)
        }
    }

    pub fn set_protocol_version(&mut self, protocol_version: u32) {
        self.protocol_version = protocol_version;
    }

    pub fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
    }
}

impl StateTransitionConvert for IdentityCreateTransition {
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

    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;

        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        let mut public_keys: Vec<Value> = vec![];
        for key in self.public_keys.iter() {
            public_keys.push(key.to_raw_object(skip_signature)?);
        }

        value.insert(
            property_names::PUBLIC_KEYS.to_owned(),
            Value::Array(public_keys),
        )?;

        Ok(value)
    }

    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object(skip_signature)
            .and_then(|v| v.try_into().map_err(ProtocolError::ValueError))
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;

        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        let mut public_keys: Vec<Value> = vec![];
        for key in self.public_keys.iter() {
            public_keys.push(key.to_raw_cleaned_object(skip_signature)?);
        }

        value.insert(
            property_names::PUBLIC_KEYS.to_owned(),
            Value::Array(public_keys),
        )?;

        Ok(value)
    }
}

impl StateTransitionLike for IdentityCreateTransition {
    /// Returns ids of created identities
    fn get_modified_data_ids(&self) -> Vec<Identifier> {
        vec![*self.get_identity_id()]
    }

    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }
    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType {
        StateTransitionType::IdentityCreate
    }
    /// returns the signature as a byte-array
    fn get_signature(&self) -> &BinaryData {
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
