use std::convert::{TryFrom, TryInto};

use serde::de::Error as DeError;
use serde::ser::Error as SerError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::state_transition::identity_create_transition::SerializationOptions;
use crate::prelude::Identifier;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::state_transition::{
    StateTransition, StateTransitionConvert, StateTransitionLike, StateTransitionType,
};
use crate::util::json_value::JsonValueExt;
use crate::util::string_encoding::Encoding;
use crate::version::LATEST_VERSION;
use crate::{NonConsensusError, ProtocolError, SerdeParsingError};

mod property_names {
    pub const ASSET_LOCK_PROOF: &str = "assetLockProof";
    pub const SIGNATURE: &str = "signature";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TRANSITION_TYPE: &str = "type";
    pub const IDENTITY_ID: &str = "identityId";
}

#[derive(Debug, Clone)]
pub struct IdentityTopUpTransition {
    // Own ST fields
    pub asset_lock_proof: AssetLockProof,
    pub identity_id: Identifier,
    // Generic identity ST fields
    pub protocol_version: u32,
    pub transition_type: StateTransitionType,
    pub signature: Vec<u8>,
    pub execution_context: StateTransitionExecutionContext,
}

impl Default for IdentityTopUpTransition {
    fn default() -> Self {
        Self {
            transition_type: StateTransitionType::IdentityTopUp,
            asset_lock_proof: Default::default(),
            identity_id: Default::default(),
            protocol_version: Default::default(),
            signature: Default::default(),
            execution_context: Default::default(),
        }
    }
}

impl From<IdentityTopUpTransition> for StateTransition {
    fn from(d: IdentityTopUpTransition) -> Self {
        Self::IdentityTopUp(d)
    }
}

impl Serialize for IdentityTopUpTransition {
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

impl<'de> Deserialize<'de> for IdentityTopUpTransition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        Self::new(value).map_err(|e| D::Error::custom(e.to_string()))
    }
}

/// Main state transition functionality implementation
impl IdentityTopUpTransition {
    pub fn new(raw_state_transition: serde_json::Value) -> Result<Self, ProtocolError> {
        Self::from_raw_object(raw_state_transition)
    }

    pub fn from_raw_object(
        raw_object: JsonValue,
    ) -> Result<IdentityTopUpTransition, ProtocolError> {
        let protocol_version = raw_object
            .get_u64(property_names::PROTOCOL_VERSION)
            .unwrap_or(LATEST_VERSION as u64) as u32;
        let signature = raw_object
            .get_bytes(property_names::SIGNATURE)
            .unwrap_or_default();
        let identity_id =
            Identifier::from_bytes(&raw_object.get_bytes(property_names::IDENTITY_ID)?)?;

        let raw_asset_lock_proof = raw_object
            .get(property_names::ASSET_LOCK_PROOF)
            .ok_or_else(|| ProtocolError::Generic("Asset lock proof is missing".to_string()))?;
        let asset_lock_proof = AssetLockProof::try_from(raw_asset_lock_proof)?;

        Ok(IdentityTopUpTransition {
            protocol_version,
            signature,
            identity_id,
            asset_lock_proof,
            transition_type: StateTransitionType::IdentityTopUp,
            execution_context: Default::default(),
        })
    }

    /// Get State Transition type
    pub fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityTopUp
    }

    /// Set asset lock
    pub fn set_asset_lock_proof(
        &mut self,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(), NonConsensusError> {
        self.asset_lock_proof = asset_lock_proof;

        Ok(())
    }

    /// Get asset lock proof
    pub fn get_asset_lock_proof(&self) -> &AssetLockProof {
        &self.asset_lock_proof
    }

    /// Set identity id
    pub fn set_identity_id(&mut self, identity_id: Identifier) {
        self.identity_id = identity_id;
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
            let bytes = self
                .identity_id
                .buffer
                .iter()
                .map(|num| JsonValue::from(*num));
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

    pub fn set_protocol_version(&mut self, protocol_version: u32) {
        self.protocol_version = protocol_version;
    }

    /// Returns ids of created identities
    pub fn get_modified_data_ids(&self) -> Vec<&Identifier> {
        vec![self.get_identity_id()]
    }
}

impl StateTransitionConvert for IdentityTopUpTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![property_names::SIGNATURE]
    }
    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![property_names::IDENTITY_ID]
    }
    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn to_object(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(self)?;

        if skip_signature {
            if let JsonValue::Object(ref mut o) = json_value {
                for path in Self::signature_property_paths() {
                    o.remove(path);
                }
            }
        }

        Ok(json_value)
    }

    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let mut json = serde_json::Value::Object(Default::default());

        // TODO: super.toJSON()

        if skip_signature {
            if let JsonValue::Object(ref mut o) = json {
                for path in Self::signature_property_paths() {
                    o.remove(path);
                }
            }
        }

        json.insert(
            property_names::ASSET_LOCK_PROOF.to_string(),
            self.asset_lock_proof.as_ref().try_into()?,
        )?;

        Ok(json)
    }
}

impl StateTransitionLike for IdentityTopUpTransition {
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
