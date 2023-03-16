use rand::rngs::StdRng;
use rand::Rng;
use std::convert::{TryFrom, TryInto};
use std::fmt;

use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;

use crate::string_encoding::Encoding;
use crate::{string_encoding, Error, Value};

pub const IDENTIFIER_MEDIA_TYPE: &str = "application/x.dash.dpp.identifier";

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct Bytes(pub [u8; 32]);

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Copy, Serialize, Deserialize)]
pub struct Identifier(pub Bytes);

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BytesVisitor;

        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = Bytes;

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
                Ok(Bytes(bytes))
            }
        }

        deserializer.deserialize_bytes(BytesVisitor)
    }
}

// impl<'de> Deserialize<'de> for Identifier {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//         where
//             D: serde::Deserializer<'de>,
//     {
//         let data: DocumentValue = Deserialize::deserialize(deserializer)?;
//         if let DocumentValue::Bytes(bytes) = data {
//             return Ok(Identifier::from(bytes.0));
//         }
//         Err(serde::de::Error::custom(format!(
//             "expected bytes, got: {:?}",
//             data
//         )))
//     }
// }

fn encoding_string_to_encoding(encoding_string: Option<&str>) -> Encoding {
    match encoding_string {
        Some(str) => {
            //? should it be case-sensitive??
            if str == "base58" {
                Encoding::Base58
            } else {
                Encoding::Base64
            }
        }
        None => Encoding::Base58,
    }
}

impl Identifier {
    pub fn new(buffer: [u8; 32]) -> Identifier {
        Identifier(Bytes(buffer))
    }

    pub fn random(rng: &mut StdRng) -> Identifier {
        Identifier(Bytes(rng.gen()))
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
                "Identifier must be 32 bytes long",
            )));
        }

        // Since we checked that vector size is 32, we can use unwrap
        Ok(Identifier::new(bytes.try_into().unwrap()))
    }

    pub fn to_vec(&self) -> Vec<JsonValue> {
        self.to_buffer()
            .iter()
            .map(|v| JsonValue::from(*v))
            .collect()
    }

    pub fn len(&self) -> usize {
        32
    }

    // TODO - consider to change the name to 'asBuffer`
    pub fn to_buffer(&self) -> [u8; 32] {
        self.0.0
    }

    /// Convenience method to get underlying buffer as a vec
    pub fn to_buffer_vec(&self) -> Vec<u8> {
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

impl PartialEq<[u8; 32]> for Identifier {
    fn eq(&self, other: &[u8; 32]) -> bool {
        &self.0.0 == other
    }
}

impl PartialEq<[u8; 32]> for &Identifier {
    fn eq(&self, other: &[u8; 32]) -> bool {
        &self.0.0 == other
    }
}

impl PartialEq<Identifier> for [u8; 32] {
    fn eq(&self, other: &Identifier) -> bool {
        self == &other.0.0
    }
}

impl PartialEq<&Identifier> for [u8; 32] {
    fn eq(&self, other: &&Identifier) -> bool {
        self == &other.0.0
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
