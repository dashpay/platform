#[cfg(feature = "json-conversion")]
mod json_conversion;
mod types;
#[cfg(feature = "value-conversion")]
mod value_conversion;
mod version;

use crate::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;

use std::convert::TryFrom;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use platform_value::{BinaryData, Value};

use crate::errors::ProtocolError;

use crate::identity::contract_bounds::ContractBounds;
use platform_serialization_derive::PlatformSignable;

use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;

use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;

use crate::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV0Setters,
};
use crate::state_transition::public_key_in_creation::methods::IdentityPublicKeyInCreationMethodsV0;

pub const BINARY_DATA_FIELDS: [&str; 2] = ["data", "signature"];

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(
    Default, Debug, Serialize, Deserialize, Encode, Decode, PlatformSignable, Clone, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPublicKeyInCreationV0 {
    pub id: KeyID,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    pub contract_bounds: Option<ContractBounds>,
    pub read_only: bool,
    pub data: BinaryData,
    /// The signature is needed for ECDSA_SECP256K1 Key type and BLS12_381 Key type
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl IdentityPublicKeyInCreationV0Getters for IdentityPublicKeyInCreationV0 {
    fn id(&self) -> KeyID {
        self.id
    }

    fn key_type(&self) -> KeyType {
        self.key_type
    }

    fn purpose(&self) -> Purpose {
        self.purpose
    }

    fn security_level(&self) -> SecurityLevel {
        self.security_level
    }

    fn read_only(&self) -> bool {
        self.read_only
    }

    fn data(&self) -> &BinaryData {
        &self.data
    }

    fn signature(&self) -> &BinaryData {
        &self.signature
    }

    fn contract_bounds(&self) -> Option<&ContractBounds> {
        self.contract_bounds.as_ref()
    }
}

impl IdentityPublicKeyInCreationV0Setters for IdentityPublicKeyInCreationV0 {
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn set_data(&mut self, data: BinaryData) {
        self.data = data
    }

    fn set_id(&mut self, id: KeyID) {
        self.id = id
    }

    fn set_type(&mut self, key_type: KeyType) {
        self.key_type = key_type;
    }

    fn set_security_level(&mut self, security_level: SecurityLevel) {
        self.security_level = security_level;
    }

    fn set_contract_bounds(&mut self, contract_bounds: Option<ContractBounds>) {
        self.contract_bounds = contract_bounds;
    }

    fn set_purpose(&mut self, purpose: Purpose) {
        self.purpose = purpose;
    }

    fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }
}

impl IdentityPublicKeyInCreationMethodsV0 for IdentityPublicKeyInCreationV0 {
    fn into_identity_public_key(self) -> IdentityPublicKey {
        let Self {
            id,
            purpose,
            security_level,
            contract_bounds,
            key_type,
            data,
            read_only,
            ..
        } = self;
        IdentityPublicKeyV0 {
            id,
            purpose,
            security_level,
            contract_bounds,
            key_type,
            data,
            read_only,
            disabled_at: None,
        }
        .into()
    }
}

//
//     #[cfg(feature = "value-conversion")]
//     pub fn from_object(mut raw_object: Value) -> Result<Self, ProtocolError> {
//         raw_object.try_into().map_err(ProtocolError::ValueError)
//     }
//
//
//
//
//     pub fn from_raw_json_object(raw_object: JsonValue) -> Result<Self, ProtocolError> {
//         let identity_public_key: Self = serde_json::from_value(raw_object)?;
//         Ok(identity_public_key)
//     }
//
//     pub fn from_json_object(raw_object: JsonValue) -> Result<Self, ProtocolError> {
//         let mut value: Value = raw_object.into();
//         value.replace_at_paths(BINARY_DATA_FIELDS, ReplacementType::BinaryBytes)?;
//         value.try_into().map_err(ProtocolError::ValueError)
//     }
//
//     /// Return raw data, with all binary fields represented as arrays
//     pub fn to_raw_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
//         let mut value = self.to_object()?;
//
//         if skip_signature || self.signature.is_empty() {
//             value
//                 .remove("signature")
//                 .map_err(ProtocolError::ValueError)?;
//         }
//
//         Ok(value)
//     }
//
//     /// Return raw data, with all binary fields represented as arrays
//     pub fn to_raw_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
//         let mut value = self.to_cleaned_object()?;
//
//         if skip_signature || self.signature.is_empty() {
//             value
//                 .remove("signature")
//                 .map_err(ProtocolError::ValueError)?;
//         }
//
//         Ok(value)
//     }
//
//     /// Return raw data, with all binary fields represented as arrays
//     pub fn to_raw_json_object(&self, skip_signature: bool) -> Result<JsonValue, SerdeParsingError> {
//         let mut value = serde_json::to_value(self)?;
//
//         if skip_signature {
//             if let JsonValue::Object(ref mut o) = value {
//                 o.remove("signature");
//             }
//         }
//
//         Ok(value)
//     }
//
//
//
//     pub fn to_ecdsa_array(&self) -> Result<[u8; 33], InvalidVectorSizeError> {
//         vec::vec_to_array::<33>(self.data.as_slice())
//     }
//
//
//
//
//
//     #[cfg(feature = "state-transition-cbor-conversion")]
//     pub fn from_cbor_value(cbor_value: &CborValue) -> Result<Self, ProtocolError> {
//         let key_value_map = cbor_value.as_map().ok_or_else(|| {
//             ProtocolError::DecodingError(String::from(
//                 "Expected identity public key to be a key value map",
//             ))
//         })?;
//
//         let id = key_value_map.as_u16("id", "A key must have an uint16 id")?;
//         let key_type = key_value_map.as_u8("type", "Identity public key must have a type")?;
//         let purpose = key_value_map.as_u8("purpose", "Identity public key must have a purpose")?;
//         let security_level = key_value_map.as_u8(
//             "securityLevel",
//             "Identity public key must have a securityLevel",
//         )?;
//         let readonly =
//             key_value_map.as_bool("readOnly", "Identity public key must have a readOnly")?;
//         let public_key_bytes =
//             key_value_map.as_bytes("data", "Identity public key must have a data")?;
//         let signature_bytes = key_value_map.as_bytes("signature", "").unwrap_or_default();
//
//         Ok(Self {
//             id: id.into(),
//             purpose: purpose.try_into()?,
//             security_level: security_level.try_into()?,
//             key_type: key_type.try_into()?,
//             data: BinaryData::from(public_key_bytes),
//             read_only: readonly,
//             signature: BinaryData::from(signature_bytes),
//         })
//     }
//
//     #[cfg(feature = "state-transition-cbor-conversion")]
//     pub fn to_cbor_value(&self) -> CborValue {
//         let mut pk_map = CborCanonicalMap::new();
//
//         pk_map.insert("id", self.id);
//         pk_map.insert("data", self.data.as_slice());
//         pk_map.insert("type", self.key_type);
//         pk_map.insert("purpose", self.purpose);
//         pk_map.insert("readOnly", self.read_only);
//         pk_map.insert("securityLevel", self.security_level);
//
//         if !self.signature.is_empty() {
//             pk_map.insert("signature", self.signature.as_slice())
//         }
//
//         pk_map.to_value_sorted()
//     }
// }

impl From<IdentityPublicKeyInCreationV0> for IdentityPublicKey {
    fn from(val: IdentityPublicKeyInCreationV0) -> Self {
        IdentityPublicKeyV0 {
            id: val.id,
            purpose: val.purpose,
            security_level: val.security_level,
            contract_bounds: val.contract_bounds,
            key_type: val.key_type,
            read_only: val.read_only,
            data: val.data,
            disabled_at: None,
        }
        .into()
    }
}

impl From<&IdentityPublicKeyInCreationV0> for IdentityPublicKey {
    fn from(val: &IdentityPublicKeyInCreationV0) -> Self {
        IdentityPublicKeyV0 {
            id: val.id,
            purpose: val.purpose,
            security_level: val.security_level,
            contract_bounds: val.contract_bounds.clone(),
            key_type: val.key_type,
            read_only: val.read_only,
            data: val.data.clone(),
            disabled_at: None,
        }
        .into()
    }
}

impl From<IdentityPublicKey> for IdentityPublicKeyInCreationV0 {
    fn from(val: IdentityPublicKey) -> Self {
        IdentityPublicKeyInCreationV0 {
            id: val.id(),
            purpose: val.purpose(),
            security_level: val.security_level(),
            contract_bounds: val.contract_bounds().cloned(),
            key_type: val.key_type(),
            read_only: val.read_only(),
            data: val.data_owned(),
            signature: Default::default(),
        }
    }
}

impl From<&IdentityPublicKey> for IdentityPublicKeyInCreationV0 {
    fn from(val: &IdentityPublicKey) -> Self {
        IdentityPublicKeyInCreationV0 {
            id: val.id(),
            purpose: val.purpose(),
            security_level: val.security_level(),
            contract_bounds: val.contract_bounds().cloned(),
            key_type: val.key_type(),
            read_only: val.read_only(),
            data: val.data().clone(),
            signature: Default::default(),
        }
    }
}

impl TryFrom<Value> for IdentityPublicKeyInCreationV0 {
    type Error = platform_value::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value)
    }
}

impl TryFrom<IdentityPublicKeyInCreationV0> for Value {
    type Error = platform_value::Error;

    fn try_from(value: IdentityPublicKeyInCreationV0) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}

impl TryFrom<&IdentityPublicKeyInCreationV0> for Value {
    type Error = platform_value::Error;

    fn try_from(value: &IdentityPublicKeyInCreationV0) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::state_transition::public_key_in_creation::accessors::{
        IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV0Setters,
    };
    use crate::state_transition::public_key_in_creation::methods::IdentityPublicKeyInCreationMethodsV0;

    fn make_key_v0() -> IdentityPublicKeyInCreationV0 {
        IdentityPublicKeyInCreationV0 {
            id: 0,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            signature: BinaryData::new(vec![0u8; 65]),
        }
    }

    #[test]
    fn test_default() {
        let key = IdentityPublicKeyInCreationV0::default();
        assert_eq!(key.id, 0);
        assert!(key.data.is_empty());
        assert!(key.signature.is_empty());
    }

    #[test]
    fn test_getters() {
        let key = make_key_v0();
        assert_eq!(key.id(), 0);
        assert_eq!(key.key_type(), KeyType::ECDSA_SECP256K1);
        assert_eq!(key.purpose(), Purpose::AUTHENTICATION);
        assert_eq!(key.security_level(), SecurityLevel::MASTER);
        assert!(!key.read_only());
        assert_eq!(key.data().len(), 33);
        assert_eq!(key.signature().len(), 65);
        assert!(key.contract_bounds().is_none());
    }

    #[test]
    fn test_setters() {
        let mut key = make_key_v0();
        key.set_id(5);
        assert_eq!(key.id(), 5);
        key.set_type(KeyType::BLS12_381);
        assert_eq!(key.key_type(), KeyType::BLS12_381);
        key.set_purpose(Purpose::TRANSFER);
        assert_eq!(key.purpose(), Purpose::TRANSFER);
        key.set_security_level(SecurityLevel::CRITICAL);
        assert_eq!(key.security_level(), SecurityLevel::CRITICAL);
        key.set_read_only(true);
        assert!(key.read_only());
        key.set_data(BinaryData::new(vec![1, 2, 3]));
        assert_eq!(key.data().as_slice(), &[1, 2, 3]);
        key.set_signature(BinaryData::new(vec![4, 5]));
        assert_eq!(key.signature().as_slice(), &[4, 5]);
        key.set_contract_bounds(None);
        assert!(key.contract_bounds().is_none());
    }

    #[test]
    fn test_into_identity_public_key() {
        let key = make_key_v0();
        let pk = key.clone().into_identity_public_key();
        assert_eq!(pk.id(), key.id);
        assert_eq!(pk.purpose(), key.purpose);
        assert_eq!(pk.security_level(), key.security_level);
        assert_eq!(pk.key_type(), key.key_type);
    }

    #[test]
    fn test_from_identity_public_key() {
        let key = make_key_v0();
        let pk: IdentityPublicKey = key.clone().into();
        let back: IdentityPublicKeyInCreationV0 = pk.into();
        assert_eq!(back.id, key.id);
        assert_eq!(back.purpose, key.purpose);
        assert_eq!(back.security_level, key.security_level);
        assert_eq!(back.key_type, key.key_type);
        assert_eq!(back.data, key.data);
        assert!(back.signature.is_empty()); // signature is cleared on conversion
    }

    #[test]
    fn test_from_identity_public_key_ref() {
        let key = make_key_v0();
        let pk: IdentityPublicKey = key.clone().into();
        let back: IdentityPublicKeyInCreationV0 = (&pk).into();
        assert_eq!(back.id, key.id);
    }

    #[test]
    fn test_from_ref_into_identity_public_key() {
        let key = make_key_v0();
        let pk: IdentityPublicKey = (&key).into();
        assert_eq!(pk.id(), key.id);
    }

    #[test]
    fn test_try_from_value_roundtrip() {
        let key = make_key_v0();
        let value: Value = (&key).try_into().expect("should convert to value");
        let restored: IdentityPublicKeyInCreationV0 =
            value.try_into().expect("should convert from value");
        assert_eq!(key, restored);
    }

    #[test]
    fn test_try_from_value_owned() {
        let key = make_key_v0();
        let value: Value = key.clone().try_into().expect("should convert to value");
        let restored: IdentityPublicKeyInCreationV0 =
            value.try_into().expect("should convert from value");
        assert_eq!(key, restored);
    }

    #[test]
    fn test_is_master() {
        let key = make_key_v0();
        assert!(key.is_master());
        let non_master = IdentityPublicKeyInCreationV0 {
            security_level: SecurityLevel::HIGH,
            ..make_key_v0()
        };
        assert!(!non_master.is_master());
    }
}
