use std::fmt;
use serde::{Deserialize, Serialize};
use serde::de::Visitor;

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BinaryData(pub Vec<u8>);

impl Serialize for BinaryData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&base64::encode(self.0.as_slice()))
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for BinaryData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {

            struct StringVisitor;

            impl<'de> Visitor<'de> for StringVisitor {
                type Value = BinaryData;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a base64-encoded string")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                {
                    let bytes = base64::decode(v).map_err(|e| E::custom(format!("{}", e)))?;
                    Ok(BinaryData(bytes))
                }
            }

            deserializer.deserialize_string(StringVisitor)
        } else {
            struct BytesVisitor;

            impl<'de> Visitor<'de> for BytesVisitor {
                type Value = BinaryData;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a byte array with length 32")
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                {
                    Ok(BinaryData(v.to_vec()))
                }
            }

            deserializer.deserialize_bytes(BytesVisitor)
        }
    }
}

impl BinaryData {
    pub fn new(buffer: Vec<u8>) -> BinaryData {
        BinaryData(buffer)
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{from_value, Identifier, to_value, Value};
    use serde::{Deserialize, Serialize};

    use super::*;

    #[test]
    fn test_binary_data_serialization() {
        let id = BinaryData::new([2; 34].to_vec());
        let value = to_value(id.clone()).unwrap();
        assert_eq!(value, Value::Bytes(id.to_vec()));
    }

    #[test]
    fn test_identifier_value_deserialization() {
        let id = Identifier::new([3; 32]);
        let value = Value::Identifier(id.to_buffer());
        let new_id: Identifier = from_value(value).unwrap();
        assert_eq!(id, new_id);
    }
}
