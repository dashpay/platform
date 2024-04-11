use crate::string_encoding::Encoding;
use crate::types::encoding_string_to_encoding;
use crate::{string_encoding, Error, Value};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bincode::{Decode, Encode};
use rand::rngs::StdRng;
use rand::Rng;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Default, Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Copy, Encode, Decode)]
pub struct Bytes32(pub [u8; 32]);

impl AsRef<[u8]> for Bytes32 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Bytes32 {
    pub fn new(buffer: [u8; 32]) -> Self {
        Bytes32(buffer)
    }

    pub fn from_vec(buffer: Vec<u8>) -> Result<Self, Error> {
        let buffer: [u8; 32] = buffer.try_into().map_err(|_| {
            Error::ByteLengthNot32BytesError("buffer was not 32 bytes long".to_string())
        })?;
        Ok(Bytes32::new(buffer))
    }

    pub fn random_with_rng(rng: &mut StdRng) -> Self {
        Bytes32(rng.gen())
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn to_buffer(&self) -> [u8; 32] {
        self.0
    }

    pub fn from_string(encoded_value: &str, encoding: Encoding) -> Result<Bytes32, Error> {
        let vec = string_encoding::decode(encoded_value, encoding)?;

        Bytes32::from_vec(vec)
    }

    pub fn from_string_with_encoding_string(
        encoded_value: &str,
        encoding_string: Option<&str>,
    ) -> Result<Bytes32, Error> {
        let encoding = encoding_string_to_encoding(encoding_string);

        Bytes32::from_string(encoded_value, encoding)
    }

    pub fn to_string(&self, encoding: Encoding) -> String {
        string_encoding::encode(&self.0, encoding)
    }

    pub fn to_string_with_encoding_string(&self, encoding_string: Option<&str>) -> String {
        let encoding = encoding_string_to_encoding(encoding_string);

        self.to_string(encoding)
    }
}

impl Serialize for Bytes32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&BASE64_STANDARD.encode(self.0))
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for Bytes32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            struct StringVisitor;

            impl<'de> Visitor<'de> for StringVisitor {
                type Value = Bytes32;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a base64-encoded string with length 44")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let bytes = BASE64_STANDARD
                        .decode(v)
                        .map_err(|e| E::custom(format!("{}", e)))?;
                    if bytes.len() != 32 {
                        return Err(E::invalid_length(bytes.len(), &self));
                    }
                    let mut array = [0u8; 32];
                    array.copy_from_slice(&bytes);
                    Ok(Bytes32(array))
                }
            }

            deserializer.deserialize_string(StringVisitor)
        } else {
            struct BytesVisitor;

            impl<'de> Visitor<'de> for BytesVisitor {
                type Value = Bytes32;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a byte array with length 32")
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let mut bytes = [0u8; 32];
                    if v.len() != 32 {
                        return Err(E::invalid_length(v.len(), &self));
                    }
                    bytes.copy_from_slice(v);
                    Ok(Bytes32(bytes))
                }
            }

            deserializer.deserialize_bytes(BytesVisitor)
        }
    }
}

impl TryFrom<Value> for Bytes32 {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.into_bytes_32()
    }
}

impl TryFrom<&Value> for Bytes32 {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value.to_bytes_32()
    }
}

impl From<[u8; 32]> for Bytes32 {
    fn from(value: [u8; 32]) -> Self {
        Bytes32(value)
    }
}

impl From<&[u8; 32]> for Bytes32 {
    fn from(value: &[u8; 32]) -> Self {
        Bytes32(*value)
    }
}

impl From<Bytes32> for Value {
    fn from(value: Bytes32) -> Self {
        Value::Bytes32(value.0)
    }
}

impl From<&Bytes32> for Value {
    fn from(value: &Bytes32) -> Self {
        Value::Bytes32(value.0)
    }
}

impl TryFrom<String> for Bytes32 {
    type Error = Error;

    fn try_from(data: String) -> Result<Self, Self::Error> {
        Self::from_string(&data, Encoding::Base64)
    }
}

impl From<Bytes32> for String {
    fn from(val: Bytes32) -> Self {
        val.to_string(Encoding::Base64)
    }
}

impl From<&Bytes32> for String {
    fn from(val: &Bytes32) -> Self {
        val.to_string(Encoding::Base64)
    }
}
