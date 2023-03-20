use std::convert::{TryFrom, TryInto};

use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::{BinaryData, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyWithWitness;
use crate::prelude::Identifier;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::state_transition::{
    StateTransition, StateTransitionConvert, StateTransitionLike, StateTransitionType,
};
use crate::{NonConsensusError, ProtocolError};
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreateTransition {
    // Own ST fields
    pub public_keys: Vec<IdentityPublicKeyWithWitness>,
    pub asset_lock_proof: AssetLockProof,
    pub identity_id: Identifier,
    // Generic identity ST fields
    pub protocol_version: u32,
    pub transition_type: StateTransitionType,
    pub signature: BinaryData,
    #[serde(skip)]
    pub execution_context: StateTransitionExecutionContext,
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
            execution_context: Default::default(),
        }
    }
}

impl From<IdentityCreateTransition> for StateTransition {
    fn from(d: IdentityCreateTransition) -> Self {
        Self::IdentityCreate(d)
    }
}

// impl Serialize for IdentityCreateTransition {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let raw = self
//             .to_json_object(Default::default())
//             .map_err(|e| S::Error::custom(e.to_string()))?;
//
//         raw.serialize(serializer)
//     }
// }
//
// impl<'de> Deserialize<'de> for IdentityCreateTransition {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let value = platform_value::Value::deserialize(deserializer)?;
//
//         Self::new(value).map_err(|e| D::Error::custom(e.to_string()))
//     }
// }

/// Main state transition functionality implementation
impl IdentityCreateTransition {
    pub fn new(raw_state_transition: Value) -> Result<Self, ProtocolError> {
        let mut state_transition = Self::default();

        let mut transition_map = raw_state_transition
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        if let Some(keys_value_array) = transition_map
            .remove_optional_inner_value_array::<Vec<_>>(property_names::PUBLIC_KEYS)
            .map_err(ProtocolError::ValueError)?
        {
            let keys = keys_value_array
                .into_iter()
                .map(|val| val.try_into().map_err(ProtocolError::ValueError))
                .collect::<Result<Vec<IdentityPublicKeyWithWitness>, ProtocolError>>()?;
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
    pub fn get_public_keys(&self) -> &[IdentityPublicKeyWithWitness] {
        &self.public_keys
    }

    /// Replaces existing set of public keys with a new one
    pub fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyWithWitness>) -> &mut Self {
        self.public_keys = public_keys;

        self
    }

    /// Adds public keys to the existing public keys array
    pub fn add_public_keys(
        &mut self,
        public_keys: &mut Vec<IdentityPublicKeyWithWitness>,
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
        vec![property_names::SIGNATURE]
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
                .remove_values_at_paths(Self::signature_property_paths())
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
                .remove_values_at_paths(Self::signature_property_paths())
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
