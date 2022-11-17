use std::convert::{TryFrom, TryInto};

use serde::de::Error as DeError;
use serde::ser::Error as SerError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::IdentityPublicKey;
use crate::prelude::Identifier;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::state_transition::{
    StateTransition, StateTransitionConvert, StateTransitionLike, StateTransitionType,
};
use crate::util::json_value::JsonValueExt;
use crate::util::string_encoding::Encoding;
use crate::{NonConsensusError, ProtocolError, SerdeParsingError};

mod property_names {
    pub const PUBLIC_KEYS: &str = "publicKeys";
    pub const ASSET_LOCK_PROOF: &str = "assetLockProof";
    pub const SIGNATURE: &str = "signature";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TRANSITION_TYPE: &str = "type";
    pub const IDENTITY_ID: &str = "identityId";
}

#[derive(Debug, Copy, Clone, Default)]
pub struct SerializationOptions {
    pub skip_signature: bool,
    pub skip_identifiers_conversion: bool,
}

#[derive(Debug, Clone)]
pub struct IdentityCreateTransition {
    // Own ST fields
    pub public_keys: Vec<IdentityPublicKey>,
    pub asset_lock_proof: AssetLockProof,
    pub identity_id: Identifier,
    // Generic identity ST fields
    pub protocol_version: u32,
    pub transition_type: StateTransitionType,
    pub signature: Vec<u8>,
    pub execution_context: StateTransitionExecutionContext,
}

impl Default for IdentityCreateTransition {
    fn default() -> Self {
        Self {
            transition_type: StateTransitionType::IdentityCreate,
            public_keys: Default::default(),
            asset_lock_proof: Default::default(),
            identity_id: Default::default(),
            protocol_version: Default::default(),
            signature: Default::default(),
            execution_context: Default::default(),
        }
    }
}

impl From<IdentityCreateTransition> for StateTransition {
    fn from(d: IdentityCreateTransition) -> Self {
        Self::IdentityCreate(d)
    }
}

impl Serialize for IdentityCreateTransition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let raw = self
            .to_json_object(Default::default())
            .map_err(|e| S::Error::custom(e.to_string()))?;

        raw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for IdentityCreateTransition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        Self::new(value).map_err(|e| D::Error::custom(e.to_string()))
    }
}

/// Main state transition functionality implementation
impl IdentityCreateTransition {
    pub fn new(raw_state_transition: serde_json::Value) -> Result<Self, NonConsensusError> {
        let mut state_transition = Self::default();

        let transition_map = raw_state_transition.as_object().ok_or_else(|| {
            SerdeParsingError::new("Expected raw identity transition to be a map")
        })?;
        if let Some(keys_value) = transition_map.get(property_names::PUBLIC_KEYS) {
            let keys_value_arr = keys_value
                .as_array()
                .ok_or_else(|| SerdeParsingError::new("Expected public keys to be an array"))?;
            let keys = keys_value_arr
                .iter()
                .map(|val| serde_json::from_value(val.clone()))
                .collect::<Result<Vec<IdentityPublicKey>, serde_json::Error>>()?;
            state_transition = state_transition.set_public_keys(keys);
        }

        if let Some(proof) = transition_map.get(property_names::ASSET_LOCK_PROOF) {
            state_transition.set_asset_lock_proof(AssetLockProof::try_from(proof)?)?;
        }

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
    pub fn get_public_keys(&self) -> &[IdentityPublicKey] {
        &self.public_keys
    }

    /// Replaces existing set of public keys with a new one
    pub fn set_public_keys(mut self, public_keys: Vec<IdentityPublicKey>) -> Self {
        self.public_keys = public_keys;

        self
    }

    /// Adds public keys to the existing public keys array
    pub fn add_public_keys(mut self, public_keys: &mut Vec<IdentityPublicKey>) -> Self {
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
    ) -> Result<JsonValue, SerdeParsingError> {
        let mut json_map = JsonValue::Object(Default::default());

        if !options.skip_signature {
            let sig = self.signature.iter().map(|num| JsonValue::from(*num));
            json_map.insert(
                property_names::SIGNATURE.to_string(),
                JsonValue::Array(sig.collect()),
            )?;
        }

        if !options.skip_identifiers_conversion {
            let bytes = self.signature.iter().map(|num| JsonValue::from(*num));
            json_map.insert(
                property_names::IDENTITY_ID.to_string(),
                JsonValue::Array(bytes.collect()),
            )?;
        } else {
            json_map.insert(
                property_names::IDENTITY_ID.to_string(),
                JsonValue::String(self.identity_id.to_string(Encoding::Base58)),
            )?;
        }

        let pk_values = self
            .public_keys
            .iter()
            .map(|pk| pk.to_raw_json_object(options.skip_signature))
            .collect::<Result<Vec<JsonValue>, SerdeParsingError>>()?;

        json_map.insert(
            property_names::PUBLIC_KEYS.to_string(),
            JsonValue::Array(pk_values),
        )?;

        json_map.insert(
            property_names::ASSET_LOCK_PROOF.to_string(),
            self.asset_lock_proof.as_ref().try_into()?,
        )?;

        // TODO ??
        json_map.insert(
            property_names::PROTOCOL_VERSION.to_string(),
            JsonValue::Number(self.get_protocol_version().into()),
        )?;

        Ok(json_map)
    }

    /// Returns ids of created identities
    pub fn get_modified_data_ids(&self) -> Vec<&Identifier> {
        vec![self.get_identity_id()]
    }
}

impl StateTransitionConvert for IdentityCreateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![property_names::SIGNATURE]
    }
    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![property_names::IDENTITY_ID]
    }
    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let mut json = serde_json::Value::Object(Default::default());

        // TODO: super.toJSON()

        json.insert(
            property_names::ASSET_LOCK_PROOF.to_string(),
            self.asset_lock_proof.as_ref().try_into()?,
        )?;

        let public_keys = self
            .public_keys
            .iter()
            .map(|pk| pk.to_json())
            .collect::<Result<Vec<JsonValue>, SerdeParsingError>>()?;

        json.insert(
            property_names::PUBLIC_KEYS.to_string(),
            serde_json::Value::Array(public_keys),
        )?;

        Ok(json)
    }
}

impl StateTransitionLike for IdentityCreateTransition {
    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }
    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType {
        StateTransitionType::IdentityTopUp
    }
    /// returns the signature as a byte-array
    fn get_signature(&self) -> &Vec<u8> {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = signature
    }
    fn get_execution_context(&self) -> &StateTransitionExecutionContext {
        &self.execution_context
    }

    fn get_execution_context_mut(&mut self) -> &mut StateTransitionExecutionContext {
        &mut self.execution_context
    }

    fn set_execution_context(&mut self, execution_context: StateTransitionExecutionContext) {
        self.execution_context = execution_context
    }
}

// @typedef {RawStateTransition & Object} RawIdentityCreateTransition
// @property {RawInstantAssetLockProof|RawChainAssetLockProof} assetLockProof
// @property {RawIdentityPublicKey[]} publicKeys
//
// @typedef {JsonStateTransition & Object} JsonIdentityCreateTransition
// @property {JsonInstantAssetLockProof|JsonChainAssetLockProof} assetLockProof
// @property {JsonIdentityPublicKey[]} publicKeys
