use crate::string_encoding::Encoding;
use crate::types::encoding_string_to_encoding;
use crate::{string_encoding, Error, Value};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bincode::{Decode, Encode};
use serde::de::Visitor;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Default, Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Copy, Encode, Decode)]
pub struct Bytes20(pub [u8; 20]);

impl AsRef<[u8]> for Bytes20 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Bytes20 {
    pub fn new(buffer: [u8; 20]) -> Self {
        Bytes20(buffer)
    }

    pub fn from_vec(buffer: Vec<u8>) -> Result<Self, Error> {
        let buffer: [u8; 20] = buffer.try_into().map_err(|_| {
            Error::ByteLengthNot20BytesError("buffer was not 20 bytes long".to_string())
        })?;
        Ok(Bytes20::new(buffer))
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn to_buffer(&self) -> [u8; 20] {
        self.0
    }

    pub fn into_buffer(self) -> [u8; 20] {
        self.0
    }

    pub fn from_string(encoded_value: &str, encoding: Encoding) -> Result<Bytes20, Error> {
        let vec = string_encoding::decode(encoded_value, encoding)?;

        Bytes20::from_vec(vec)
    }

    pub fn from_string_with_encoding_string(
        encoded_value: &str,
        encoding_string: Option<&str>,
    ) -> Result<Bytes20, Error> {
        let encoding = encoding_string_to_encoding(encoding_string);

        Bytes20::from_string(encoded_value, encoding)
    }

    pub fn to_string(&self, encoding: Encoding) -> String {
        string_encoding::encode(&self.0, encoding)
    }

    pub fn to_string_with_encoding_string(&self, encoding_string: Option<&str>) -> String {
        let encoding = encoding_string_to_encoding(encoding_string);

        self.to_string(encoding)
    }
}

impl Serialize for Bytes20 {
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

impl<'de> Deserialize<'de> for Bytes20 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            struct StringVisitor;

            impl<'de> Visitor<'de> for StringVisitor {
                type Value = Bytes20;

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
                    if bytes.len() != 20 {
                        return Err(E::invalid_length(bytes.len(), &self));
                    }
                    let mut array = [0u8; 20];
                    array.copy_from_slice(&bytes);
                    Ok(Bytes20(array))
                }
            }

            deserializer.deserialize_string(StringVisitor)
        } else {
            struct BytesVisitor;

            impl<'de> Visitor<'de> for BytesVisitor {
                type Value = Bytes20;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a byte array with length 20")
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let mut bytes = [0u8; 20];
                    if v.len() != 20 {
                        return Err(E::invalid_length(v.len(), &self));
                    }
                    bytes.copy_from_slice(v);
                    Ok(Bytes20(bytes))
                }
            }

            deserializer.deserialize_bytes(BytesVisitor)
        }
    }
}

impl TryFrom<Value> for Bytes20 {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.into_bytes_20()
    }
}

impl TryFrom<&Value> for Bytes20 {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value.to_bytes_20()
    }
}

impl From<[u8; 20]> for Bytes20 {
    fn from(value: [u8; 20]) -> Self {
        Bytes20(value)
    }
}

impl From<&[u8; 20]> for Bytes20 {
    fn from(value: &[u8; 20]) -> Self {
        Bytes20(*value)
    }
}

impl From<Bytes20> for Value {
    fn from(value: Bytes20) -> Self {
        Value::Bytes20(value.0)
    }
}

impl From<&Bytes20> for Value {
    fn from(value: &Bytes20) -> Self {
        Value::Bytes20(value.0)
    }
}

impl TryFrom<String> for Bytes20 {
    type Error = Error;

    fn try_from(data: String) -> Result<Self, Self::Error> {
        Self::from_string(&data, Encoding::Base64)
    }
}

impl From<Bytes20> for String {
    fn from(val: Bytes20) -> Self {
        val.to_string(Encoding::Base64)
    }
}

impl From<&Bytes20> for String {
    fn from(val: &Bytes20) -> Self {
        val.to_string(Encoding::Base64)
    }
}
