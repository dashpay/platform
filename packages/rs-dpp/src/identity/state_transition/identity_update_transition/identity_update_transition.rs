use bincode::{Decode, Encode};
use platform_value::{BinaryData, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::convert::{TryFrom, TryInto};

use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreationWithWitness;
use crate::prelude::Identity;
use crate::{
    identity::{KeyID, SecurityLevel},
    prelude::{Identifier, Revision, TimestampMillis},
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionType,
    },
    version::LATEST_VERSION,
    ProtocolError,
};

pub mod property_names {
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TYPE: &str = "type";
    pub const IDENTITY_ID: &str = "identityId";
    pub const REVISION: &str = "revision";
    pub const ADD_PUBLIC_KEYS_DATA: &str = "addPublicKeys[].data";
    pub const ADD_PUBLIC_KEYS_SIGNATURE: &str = "addPublicKeys[].signature";
    pub const ADD_PUBLIC_KEYS: &str = "addPublicKeys";
    pub const DISABLE_PUBLIC_KEYS: &str = "disablePublicKeys";
    pub const PUBLIC_KEYS_DISABLED_AT: &str = "publicKeysDisabledAt";
    pub const SIGNATURE: &str = "signature";
    pub const SIGNATURE_PUBLIC_KEY_ID: &str = "signaturePublicKeyId";
}

pub const IDENTIFIER_FIELDS: [&str; 1] = [property_names::IDENTITY_ID];
pub const BINARY_FIELDS: [&str; 3] = [
    property_names::ADD_PUBLIC_KEYS_DATA,
    property_names::ADD_PUBLIC_KEYS_SIGNATURE,
    property_names::SIGNATURE,
];

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityUpdateTransition {
    pub protocol_version: u32,
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,

    /// Cryptographic signature of the State Transition
    pub signature: BinaryData,

    /// The ID of the public key used to sing the State Transition
    pub signature_public_key_id: KeyID,

    /// Unique identifier of the identity to be updated
    pub identity_id: Identifier,

    /// Identity Update revision number
    pub revision: Revision,

    /// Public Keys to add to the Identity
    /// we want to skip serialization of transitions, as we does it manually in `to_object()`  and `to_json()`
    #[serde(default)]
    pub add_public_keys: Vec<IdentityPublicKeyInCreationWithWitness>,

    /// Identity Public Keys ID's to disable for the Identity
    pub disable_public_keys: Vec<KeyID>,

    /// Timestamp when keys were disabled
    pub public_keys_disabled_at: Option<TimestampMillis>,
}

impl Default for IdentityUpdateTransition {
    fn default() -> Self {
        Self {
            protocol_version: LATEST_VERSION,
            transition_type: StateTransitionType::IdentityUpdate,
            signature: Default::default(),
            signature_public_key_id: Default::default(),
            identity_id: Default::default(),
            revision: Default::default(),
            add_public_keys: Default::default(),
            disable_public_keys: Default::default(),
            public_keys_disabled_at: Default::default(),
        }
    }
}

impl IdentityUpdateTransition {
    pub fn new(raw_state_transition: Value) -> Result<Self, ProtocolError> {
        IdentityUpdateTransition::from_raw_object(raw_state_transition)
    }

    pub fn from_raw_object(
        mut raw_object: Value,
    ) -> Result<IdentityUpdateTransition, ProtocolError> {
        let protocol_version = raw_object
            .get_optional_integer(property_names::PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or(LATEST_VERSION);
        let signature = raw_object
            .get_binary_data(property_names::SIGNATURE)
            .map_err(ProtocolError::ValueError)?;
        let signature_public_key_id = raw_object
            .get_integer(property_names::SIGNATURE_PUBLIC_KEY_ID)
            .map_err(ProtocolError::ValueError)?;
        let identity_id = raw_object
            .get_identifier(property_names::IDENTITY_ID)
            .map_err(ProtocolError::ValueError)?;

        let revision = raw_object
            .get_integer(property_names::REVISION)
            .map_err(ProtocolError::ValueError)?;
        let add_public_keys = get_list(&mut raw_object, property_names::ADD_PUBLIC_KEYS)?;
        let disable_public_keys =
            remove_integer_list_or_default(&mut raw_object, property_names::DISABLE_PUBLIC_KEYS)?;
        let public_keys_disabled_at = raw_object
            .remove_optional_integer(property_names::PUBLIC_KEYS_DISABLED_AT)
            .map_err(ProtocolError::ValueError)?;

        Ok(IdentityUpdateTransition {
            protocol_version,
            signature,
            signature_public_key_id,
            identity_id,
            revision,
            add_public_keys,
            disable_public_keys,
            public_keys_disabled_at,
            transition_type: StateTransitionType::IdentityUpdate,
        })
    }

    /// Get State Transition Type
    pub fn get_type(&self) -> StateTransitionType {
        self.transition_type
    }

    pub fn set_identity_id(&mut self, id: Identifier) {
        self.identity_id = id;
    }

    pub fn get_identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

    pub fn get_revision(&self) -> Revision {
        self.revision
    }

    pub fn set_public_keys_to_add(
        &mut self,
        add_public_keys: Vec<IdentityPublicKeyInCreationWithWitness>,
    ) {
        self.add_public_keys = add_public_keys;
    }

    pub fn get_public_keys_to_add(&self) -> &[IdentityPublicKeyInCreationWithWitness] {
        &self.add_public_keys
    }

    pub fn get_public_keys_to_add_mut(&mut self) -> &mut [IdentityPublicKeyInCreationWithWitness] {
        &mut self.add_public_keys
    }

    pub fn set_public_key_ids_to_disable(&mut self, disable_public_keys: Vec<KeyID>) {
        self.disable_public_keys = disable_public_keys;
    }

    pub fn get_public_key_ids_to_disable(&self) -> &[KeyID] {
        &self.disable_public_keys
    }

    pub fn set_public_keys_disabled_at(
        &mut self,
        public_keys_disabled_at: Option<TimestampMillis>,
    ) {
        self.public_keys_disabled_at = public_keys_disabled_at;
    }

    pub fn get_public_keys_disabled_at(&self) -> Option<TimestampMillis> {
        self.public_keys_disabled_at
    }

    pub fn set_protocol_version(&mut self, protocol_version: u32) {
        self.protocol_version = protocol_version;
    }

    pub fn clean_value(value: &mut Value) -> Result<(), platform_value::Error> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        Ok(())
    }
}

/// if the property isn't present the empty list is returned. If property is defined, the function
/// might return some serialization-related errors
fn get_list<T: TryFrom<Value, Error = platform_value::Error>>(
    value: &mut Value,
    property_name: &str,
) -> Result<Vec<T>, ProtocolError> {
    value
        .remove_optional_array(property_name)
        .map_err(ProtocolError::ValueError)?
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.try_into().map_err(ProtocolError::ValueError))
        .collect()
}

/// if the property isn't present the empty list is returned. If property is defined, the function
/// might return some serialization-related errors
fn remove_integer_list_or_default<T>(
    value: &mut Value,
    property_name: &str,
) -> Result<Vec<T>, ProtocolError>
where
    T: TryFrom<i128>
        + TryFrom<u128>
        + TryFrom<u64>
        + TryFrom<i64>
        + TryFrom<u32>
        + TryFrom<i32>
        + TryFrom<u16>
        + TryFrom<i16>
        + TryFrom<u8>
        + TryFrom<i8>,
{
    value
        .remove_optional_array(property_name)
        .map_err(ProtocolError::ValueError)?
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.to_integer().map_err(ProtocolError::ValueError))
        .collect()
}

impl StateTransitionConvert for IdentityUpdateTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![
            property_names::SIGNATURE,
            property_names::ADD_PUBLIC_KEYS_SIGNATURE,
        ]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![property_names::IDENTITY_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![
            property_names::SIGNATURE,
            property_names::SIGNATURE_PUBLIC_KEY_ID,
            property_names::ADD_PUBLIC_KEYS_SIGNATURE,
        ]
    }

    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;

        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        let mut add_public_keys: Vec<Value> = vec![];
        for key in self.add_public_keys.iter() {
            add_public_keys.push(key.to_raw_object(skip_signature)?);
        }

        value.insert(
            property_names::ADD_PUBLIC_KEYS.to_owned(),
            Value::Array(add_public_keys),
        )?;

        Ok(value)
    }

    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object(skip_signature)
            .and_then(|value| value.try_into().map_err(ProtocolError::ValueError))
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;

        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        let mut add_public_keys: Vec<Value> = vec![];
        for key in self.add_public_keys.iter() {
            add_public_keys.push(key.to_raw_cleaned_object(skip_signature)?);
        }

        value.insert(
            property_names::ADD_PUBLIC_KEYS.to_owned(),
            Value::Array(add_public_keys),
        )?;

        Ok(value)
    }
}

impl StateTransitionLike for IdentityUpdateTransition {
    /// Returns ids of created identities
    fn get_modified_data_ids(&self) -> Vec<Identifier> {
        vec![*self.get_identity_id()]
    }

    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }

    fn get_signature(&self) -> &BinaryData {
        &self.signature
    }

    fn get_type(&self) -> StateTransitionType {
        self.transition_type
    }

    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature;
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }
}

impl StateTransitionIdentitySigned for IdentityUpdateTransition {
    fn get_owner_id(&self) -> &Identifier {
        &self.identity_id
    }

    fn get_security_level_requirement(&self) -> SecurityLevel {
        SecurityLevel::MASTER
    }

    fn get_signature_public_key_id(&self) -> Option<KeyID> {
        Some(self.signature_public_key_id)
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }
}

#[cfg(test)]
mod test {
    use crate::tests::{fixtures::identity_fixture, utils::generate_random_identifier_struct};
    use getrandom::getrandom;

    use super::*;

    #[test]
    fn conversion_to_json_object() {
        let public_key = identity_fixture().get_public_keys()[&0].to_owned();
        let mut buffer = [0u8; 33];
        let _ = getrandom(&mut buffer);
        let transition = IdentityUpdateTransition {
            identity_id: generate_random_identifier_struct(),
            add_public_keys: vec![(&public_key).into()],
            signature: BinaryData::new(buffer.to_vec()),
            ..Default::default()
        };

        let result = transition
            .to_json(false)
            .expect("conversion to json shouldn't fail");

        assert!(matches!(
            result[property_names::IDENTITY_ID],
            JsonValue::String(_)
        ));
        assert!(matches!(
            result[property_names::SIGNATURE],
            JsonValue::String(_)
        ));
        assert!(matches!(
            result[property_names::ADD_PUBLIC_KEYS][0]["data"],
            JsonValue::String(_)
        ));
    }

    #[test]
    fn conversion_to_raw_object() {
        let public_key = identity_fixture().get_public_keys()[&0].to_owned();
        let mut buffer = [0u8; 33];
        let _ = getrandom(&mut buffer);
        let transition = IdentityUpdateTransition {
            identity_id: generate_random_identifier_struct(),
            add_public_keys: vec![(&public_key).into()],
            signature: BinaryData::new(buffer.to_vec()),

            ..Default::default()
        };

        let result = transition
            .to_object(false)
            .expect("conversion to raw object shouldn't fail");

        assert!(matches!(
            result[property_names::IDENTITY_ID],
            Value::Identifier(_)
        ));
        assert!(matches!(result[property_names::SIGNATURE], Value::Bytes(_)));
        assert!(matches!(
            result[property_names::ADD_PUBLIC_KEYS][0]["data"],
            Value::Bytes(_)
        ));
    }
}
