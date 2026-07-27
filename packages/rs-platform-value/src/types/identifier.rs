use bincode::enc::Encoder;
use bincode::error::EncodeError;
use bincode::{Decode, Encode};
use rand::distributions::Standard;
use rand::prelude::Distribution;
use rand::rngs::StdRng;
use rand::Rng;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};
#[cfg(feature = "json")]
use serde_json::Value as JsonValue;
use std::convert::{TryFrom, TryInto};
use std::fmt;

use crate::string_encoding::{Encoding, ALL_ENCODINGS};
use crate::types::encoding_string_to_encoding;
use crate::{string_encoding, Error, Value};

pub const IDENTIFIER_MEDIA_TYPE: &str = "application/x.dash.dpp.identifier";

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Copy, Encode, Decode)]
pub struct IdentifierBytes32(pub [u8; 32]);

#[derive(
    Default,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Ord,
    PartialOrd,
    Copy,
    Serialize,
    Deserialize,
    Encode,
    Decode,
)]
pub struct Identifier(pub IdentifierBytes32);

impl Distribution<Identifier> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Identifier {
        let bytes: [u8; 32] = rng.gen();
        Identifier::new(bytes)
    }
}

impl platform_serialization::PlatformVersionEncode for Identifier {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        _: &platform_version::version::PlatformVersion,
    ) -> Result<(), EncodeError> {
        self.0 .0.encode(encoder)
    }
}

impl platform_serialization::PlatformVersionedDecode for Identifier {
    fn platform_versioned_decode<D: bincode::de::Decoder>(
        decoder: &mut D,
        _platform_version: &platform_version::version::PlatformVersion,
    ) -> Result<Self, bincode::error::DecodeError> {
        let bytes = <[u8; 32]>::decode(decoder)?;
        Ok(Identifier::new(bytes))
    }
}

impl AsRef<[u8]> for Identifier {
    fn as_ref(&self) -> &[u8] {
        &(self.0 .0)
    }
}

impl From<Identifier> for [u8; 32] {
    fn from(id: Identifier) -> Self {
        id.into_buffer()
    }
}

impl Serialize for IdentifierBytes32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&bs58::encode(self.0).into_string())
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for IdentifierBytes32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Both visitors accept strings AND bytes because serde's ContentDeserializer
        // (used for internally tagged enums like `#[serde(tag = "$version")]`) defaults
        // `is_human_readable` to `true` regardless of the parent deserializer's setting.
        // This means bytes can arrive through the string path and vice versa.

        if deserializer.is_human_readable() {
            struct StringVisitor;

            impl Visitor<'_> for StringVisitor {
                type Value = IdentifierBytes32;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a base58-encoded string or 32-byte array")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let bytes = bs58::decode(v)
                        .into_vec()
                        .map_err(|e| E::custom(format!("expected base 58: {}", e)))?;
                    if bytes.len() != 32 {
                        return Err(E::invalid_length(bytes.len(), &self));
                    }
                    let mut array = [0u8; 32];
                    array.copy_from_slice(&bytes);
                    Ok(IdentifierBytes32(array))
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    if v.len() != 32 {
                        return Err(E::invalid_length(v.len(), &self));
                    }
                    let mut array = [0u8; 32];
                    array.copy_from_slice(v);
                    Ok(IdentifierBytes32(array))
                }
            }

            deserializer.deserialize_string(StringVisitor)
        } else {
            struct BytesVisitor;

            impl Visitor<'_> for BytesVisitor {
                type Value = IdentifierBytes32;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a 32-byte array or base58-encoded string")
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    if v.len() != 32 {
                        return Err(E::invalid_length(v.len(), &self));
                    }
                    let mut array = [0u8; 32];
                    array.copy_from_slice(v);
                    Ok(IdentifierBytes32(array))
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let bytes = bs58::decode(v)
                        .into_vec()
                        .map_err(|e| E::custom(format!("expected base 58: {}", e)))?;
                    if bytes.len() != 32 {
                        return Err(E::invalid_length(bytes.len(), &self));
                    }
                    let mut array = [0u8; 32];
                    array.copy_from_slice(&bytes);
                    Ok(IdentifierBytes32(array))
                }
            }

            deserializer.deserialize_bytes(BytesVisitor)
        }
    }
}

impl Identifier {
    pub const fn new(buffer: [u8; 32]) -> Identifier {
        Identifier(IdentifierBytes32(buffer))
    }

    pub fn random() -> Identifier {
        Identifier(IdentifierBytes32(rand::random::<[u8; 32]>()))
    }

    pub fn random_with_rng(rng: &mut StdRng) -> Identifier {
        Identifier(IdentifierBytes32(rng.gen()))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0 .0
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0 .0.as_slice()
    }

    pub fn from_string(encoded_value: &str, encoding: Encoding) -> Result<Identifier, Error> {
        let vec = string_encoding::decode(encoded_value, encoding)?;

        Identifier::from_bytes(&vec)
    }

    pub fn from_string_try_encodings(
        encoded_value: &str,
        encodings: &[Encoding],
    ) -> Result<Identifier, Error> {
        let mut tried = vec![];
        for encoding in encodings {
            if let Ok(vec) = string_encoding::decode(encoded_value, *encoding) {
                if vec.len() == 32 {
                    return Identifier::from_bytes(&vec);
                }
            }
            tried.push(encoding.to_string());
        }
        Err(Error::StringDecodingError(format!(
            "Failed to decode string with encodings [{}]",
            tried.join(", ")
        )))
    }

    pub fn from_string_unknown_encoding(encoded_value: &str) -> Result<Identifier, Error> {
        Identifier::from_string_try_encodings(encoded_value, &ALL_ENCODINGS)
    }

    pub fn from_string_with_encoding_string(
        encoded_value: &str,
        encoding_string: Option<&str>,
    ) -> Result<Identifier, Error> {
        let encoding = encoding_string_to_encoding(encoding_string);

        Identifier::from_string(encoded_value, encoding)
    }

    // TODO the constructor "From" shouldn't use the reference to collection
    pub fn from_bytes(bytes: &[u8]) -> Result<Identifier, Error> {
        if bytes.len() != 32 {
            return Err(Error::ByteLengthNot32BytesError(String::from(
                "Identifier must be 32 bytes long from bytes",
            )));
        }

        // Since we checked that vector size is 32, we can use unwrap
        Ok(Identifier::new(bytes.try_into().unwrap()))
    }

    pub fn from_vec(vec: Vec<u8>) -> Result<Identifier, Error> {
        if vec.len() != 32 {
            return Err(Error::ByteLengthNot32BytesError(String::from(
                "Identifier must be 32 bytes long from vec",
            )));
        }

        // Since we checked that vector size is 32, we can use unwrap
        Ok(Identifier::new(vec.try_into().unwrap()))
    }

    #[cfg(feature = "json")]
    pub fn to_json_value_vec(&self) -> Vec<JsonValue> {
        self.to_buffer()
            .iter()
            .map(|v| JsonValue::from(*v))
            .collect()
    }

    pub fn len(&self) -> usize {
        32
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    // TODO - consider to change the name to 'asBuffer`
    pub fn to_buffer(&self) -> [u8; 32] {
        self.0 .0
    }

    pub fn into_buffer(self) -> [u8; 32] {
        self.0 .0
    }

    /// Convenience method to get underlying buffer as a vec
    pub fn to_vec(&self) -> Vec<u8> {
        self.0 .0.to_vec()
    }

    pub fn to_string(&self, encoding: Encoding) -> String {
        string_encoding::encode(&self.0 .0, encoding)
    }

    pub fn to_string_with_encoding_string(&self, encoding_string: Option<&str>) -> String {
        let encoding = encoding_string_to_encoding(encoding_string);

        self.to_string(encoding)
    }
}

impl TryFrom<&[u8]> for Identifier {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl TryFrom<&Vec<u8>> for Identifier {
    type Error = Error;

    fn try_from(bytes: &Vec<u8>) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes.as_slice())
    }
}

impl TryFrom<Vec<u8>> for Identifier {
    type Error = Error;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::from_bytes(&bytes)
    }
}

impl TryFrom<String> for Identifier {
    type Error = Error;

    fn try_from(data: String) -> Result<Self, Self::Error> {
        Self::from_string(&data, Encoding::Base58)
    }
}

impl From<[u8; 32]> for Identifier {
    fn from(bytes: [u8; 32]) -> Self {
        Self::new(bytes)
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string(Encoding::Base58))
    }
}

impl PartialEq<&Identifier> for Identifier {
    fn eq(&self, other: &&Identifier) -> bool {
        self.0 .0 == other.0 .0
    }
}

impl PartialEq<[u8; 32]> for Identifier {
    fn eq(&self, other: &[u8; 32]) -> bool {
        &self.0 .0 == other
    }
}

impl PartialEq<[u8; 32]> for &Identifier {
    fn eq(&self, other: &[u8; 32]) -> bool {
        &self.0 .0 == other
    }
}

impl PartialEq<Identifier> for [u8; 32] {
    fn eq(&self, other: &Identifier) -> bool {
        self == &other.0 .0
    }
}

impl PartialEq<&Identifier> for [u8; 32] {
    fn eq(&self, other: &&Identifier) -> bool {
        self == &other.0 .0
    }
}

impl TryFrom<Value> for Identifier {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.into_identifier()
    }
}

impl TryFrom<&Value> for Identifier {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value.to_identifier()
    }
}

impl From<Identifier> for Value {
    fn from(value: Identifier) -> Self {
        Value::Identifier(value.0 .0)
    }
}

impl From<&Identifier> for Value {
    fn from(value: &Identifier) -> Self {
        Value::Identifier(value.0 .0)
    }
}

impl From<Identifier> for String {
    fn from(val: Identifier) -> Self {
        val.to_string(Encoding::Base58)
    }
}

impl From<&Identifier> for String {
    fn from(val: &Identifier) -> Self {
        val.to_string(Encoding::Base58)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{from_value, to_value, Identifier};

    #[test]
    fn test_identifier_value_serialization() {
        let id = Identifier::new([2; 32]);
        let value = to_value(id).unwrap();
        assert_eq!(value, Value::Identifier(id.to_buffer()));
    }

    #[test]
    fn test_identifier_value_deserialization() {
        let id = Identifier::new([3; 32]);
        let value = Value::Identifier(id.to_buffer());
        let new_id: Identifier = from_value(value).unwrap();
        assert_eq!(id, new_id);
    }

    /// Serde's ContentDeserializer (used for internally tagged enums) always
    /// reports `is_human_readable() = true`, even when the parent serializer
    /// was non-HR (like platform_value). This means bytes serialized by
    /// platform_value arrive through the "human-readable" deserialization path.
    /// Without `visit_bytes` on the StringVisitor, this would fail with
    /// "invalid type: byte array, expected a base58-encoded string".
    #[test]
    fn test_identifier_in_tagged_enum_platform_value_roundtrip() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        #[serde(tag = "$version")]
        enum Tagged {
            #[serde(rename = "0")]
            V0 { owner_id: IdentifierBytes32 },
        }

        let id = IdentifierBytes32([7u8; 32]);
        let original = Tagged::V0 { owner_id: id };

        // platform_value is non-HR, so IdentifierBytes32 serializes as bytes.
        // But on deserialization, serde's ContentDeserializer (needed for the
        // tag) reports is_human_readable()=true, sending bytes through the
        // string visitor path.
        let value = to_value(&original).expect("serialize to platform_value");
        let restored: Tagged = from_value(value).expect("deserialize from platform_value");
        assert_eq!(original, restored);
    }
}
