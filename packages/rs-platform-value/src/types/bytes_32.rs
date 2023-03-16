use std::fmt;
use std::fmt::Write;
use serde::{Deserialize, Serialize};
use serde::de::Visitor;

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct Bytes32(pub [u8; 32]);

impl Serialize for Bytes32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&base64::encode(self.0))
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
                    let bytes = base64::decode(v).map_err(|e| E::custom(format!("{}", e)))?;
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