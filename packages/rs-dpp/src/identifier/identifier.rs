use std::convert::{TryFrom, TryInto};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;

use crate::errors::ProtocolError;
use crate::util::string_encoding;
use crate::util::string_encoding::Encoding;

pub const MEDIA_TYPE: &str = "application/x.dash.dpp.identifier";

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub buffer: [u8; 32],
}

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
        Identifier { buffer }
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.buffer
    }

    pub fn from_string(
        encoded_value: &str,
        encoding: Encoding,
    ) -> Result<Identifier, ProtocolError> {
        let vec = string_encoding::decode(encoded_value, encoding)?;

        Identifier::from_bytes(&vec)
    }

    pub fn from_string_with_encoding_string(
        encoded_value: &str,
        encoding_string: Option<&str>,
    ) -> Result<Identifier, ProtocolError> {
        let encoding = encoding_string_to_encoding(encoding_string);

        Identifier::from_string(encoded_value, encoding)
    }

    // TODO the constructor "From" shouldn't use the reference to collection
    pub fn from_bytes(bytes: &[u8]) -> Result<Identifier, ProtocolError> {
        if bytes.len() != 32 {
            return Err(ProtocolError::IdentifierError(String::from(
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

    // TODO - consider to change the name to 'asBuffer`
    pub fn to_buffer(&self) -> [u8; 32] {
        self.buffer
    }

    pub fn to_string(&self, encoding: Encoding) -> String {
        string_encoding::encode(&self.buffer, encoding)
    }

    pub fn to_string_with_encoding_string(&self, encoding_string: Option<&str>) -> String {
        let encoding = encoding_string_to_encoding(encoding_string);

        self.to_string(encoding)
    }
}

impl TryFrom<&[u8]> for Identifier {
    type Error = ProtocolError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl From<[u8; 32]> for Identifier {
    fn from(bytes: [u8; 32]) -> Self {
        Self::new(bytes)
    }
}

// TODO change default serialization to bytes
impl Serialize for Identifier {
    fn serialize<S>(self: &Identifier, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // by default we use base58 as Identifier type should be encoded in that way
        serializer.serialize_str(&self.to_string(Encoding::Base58))
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Identifier, D::Error> {
        let data: String = Deserialize::deserialize(d)?;

        // by default we use base58 as Identifier type should be encoded in that way
        Identifier::from_string_with_encoding_string(&data, Some("base58"))
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

// impl Serialize for Identifier {
//     fn serialize<S>(self: &Identifier, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         // by default we use base58 as Identifier type should be encoded in that way
//         serializer.serialize_bytes(&self.to_buffer())
//     }
// }
//
// //#[serde(bound = "T: MyTrait")]
// impl<'de> Deserialize<'de> for Identifier {
//     fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Identifier, D::Error> {
//         println!("id 1");
//         let bytes: CborValue = Deserialize::deserialize(d)?;
//
//         let bytes = bytes
//             .as_bytes()
//             .ok_or_else(|| serde::de::Error::custom("Expected Identifier to be bytes"))?;
//
//         println!("id 2");
//         let id = Identifier::from_bytes(bytes).map_err(|e| serde::de::Error::custom(e.to_string()));
//         println!("id 3");
//
//         id
//     }
// }

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string(Encoding::Base58))
    }
}
