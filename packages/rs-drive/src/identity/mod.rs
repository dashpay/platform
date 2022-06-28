use std::collections::BTreeMap;

use ciborium::value::Value;
use serde::{Deserialize, Serialize};

use crate::common;
use crate::common::bytes_for_system_value_from_tree_map;
use crate::drive::Drive;
use crate::error::identity::IdentityError;
use crate::error::structure::StructureError;
use crate::error::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Identity {
    pub id: [u8; 32],
    pub revision: u64,
    pub balance: u64,
    pub keys: BTreeMap<u16, IdentityKey>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct IdentityKey {
    pub id: u16,
    pub key_type: u8,
    pub public_key_bytes: Vec<u8>,
    pub purpose: u8,
    pub security_level: u8,
    pub readonly: bool,
}

impl IdentityKey {
    pub fn from_cbor_value(key_value_map: &[(Value, Value)]) -> Result<Self, Error> {
        let id = match common::cbor_inner_u16_value(key_value_map, "id") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have an id",
                )))
            }
        };

        let key_type = match common::cbor_inner_u8_value(key_value_map, "type") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have a type",
                )))
            }
        };

        let purpose = match common::cbor_inner_u8_value(key_value_map, "purpose") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have a purpose",
                )))
            }
        };

        let security_level = match common::cbor_inner_u8_value(key_value_map, "securityLevel") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have a securityLevel",
                )))
            }
        };

        let readonly = match common::cbor_inner_bool_value(key_value_map, "readOnly") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have a readOnly value",
                )))
            }
        };

        let public_key_bytes = match common::cbor_inner_bytes_value(key_value_map, "data") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::Identity(IdentityError::IdentityKeyMissingField(
                    "a key must have a data value",
                )))
            }
        };

        Ok(IdentityKey {
            id,
            key_type,
            public_key_bytes,
            purpose,
            security_level,
            readonly,
        })
    }
}

impl Identity {
    pub fn from_cbor(identity_cbor: &[u8]) -> Result<Self, Error> {
        let (version, read_identity_cbor) = identity_cbor.split_at(4);
        if !Drive::check_protocol_version_bytes(version) {
            return Err(Error::Structure(StructureError::InvalidProtocolVersion(
                "invalid protocol version",
            )));
        }
        // Deserialize the contract
        let identity: BTreeMap<String, Value> = ciborium::de::from_reader(read_identity_cbor)
            .map_err(|_| {
                Error::Structure(StructureError::InvalidCBOR("unable to decode identity"))
            })?;

        // Get the contract id
        let identity_id: [u8; 32] = bytes_for_system_value_from_tree_map(&identity, "id")?
            .ok_or({
                Error::Identity(IdentityError::MissingRequiredKey(
                    "unable to get contract id",
                ))
            })?
            .try_into()
            .map_err(|_| {
                Error::Identity(IdentityError::FieldRequirementUnmet("id must be 32 bytes"))
            })?;

        let revision: u64 = identity
            .get("revision")
            .ok_or({
                Error::Identity(IdentityError::MissingRequiredKey("unable to get revision"))
            })?
            .as_integer()
            .ok_or({
                Error::Structure(StructureError::KeyWrongType("revision must be an integer"))
            })?
            .try_into()
            .map_err(|_| {
                Error::Structure(StructureError::KeyWrongBounds(
                    "revision must be in the range of a unsigned 64 bit integer",
                ))
            })?;

        let balance: u64 = identity
            .get("balance")
            .ok_or(Error::Identity(IdentityError::MissingRequiredKey(
                "unable to get balance",
            )))?
            .as_integer()
            .ok_or({
                Error::Structure(StructureError::KeyWrongType("balance must be an integer"))
            })?
            .try_into()
            .map_err(|_| {
                Error::Structure(StructureError::KeyWrongBounds(
                    "balance must be in the range of a unsigned 64 bit integer",
                ))
            })?;

        let keys_cbor_value = identity.get("publicKeys").ok_or(Error::Identity(
            IdentityError::MissingRequiredKey("unable to get keys"),
        ))?;
        let keys_cbor_value_raw = keys_cbor_value.as_array().ok_or({
            Error::Identity(IdentityError::InvalidIdentityStructure(
                "unable to get keys as map",
            ))
        })?;

        let mut keys: BTreeMap<u16, IdentityKey> = BTreeMap::new();

        // Build the document type hashmap
        for key in keys_cbor_value_raw {
            match key.as_map() {
                None => {
                    return Err(Error::Identity(IdentityError::InvalidIdentityStructure(
                        "key value is not a map as expected",
                    )));
                }
                Some(map) => {
                    let key = IdentityKey::from_cbor_value(map)?;
                    keys.insert(key.id, key);
                }
            }
        }

        Ok(Identity {
            id: identity_id,
            revision,
            balance,
            keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::identity::Identity;

    #[test]
    pub fn deserialize() {
        let identity_cbor = hex::decode("01000000a46269645820648d10ec3a16a37d2e62e8481820dbc2a853834625b065c036e3f998389e6a296762616c616e636500687265766973696f6e006a7075626c69634b65797382a6626964006464617461582102eaf222e32d46b97f56f890bb22c3d65e279b18bda203f30bd2d3eed769a3476264747970650067707572706f73650068726561644f6e6c79f46d73656375726974794c6576656c00a6626964016464617461582103c00af793d83155f95502b33a17154110946dcf69ca0dd188bee3b6d10c0d4f8b64747970650067707572706f73650168726561644f6e6c79f46d73656375726974794c6576656c03").unwrap();
        let identity = Identity::from_cbor(identity_cbor.as_slice())
            .expect("expected to deserialize an identity");
    }
}
