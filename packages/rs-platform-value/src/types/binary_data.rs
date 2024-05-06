use crate::string_encoding::Encoding;
use crate::types::encoding_string_to_encoding;
use crate::{string_encoding, Error, Value};
use bincode::{Decode, Encode};
use serde::de::Visitor;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Default, Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Encode, Decode)]
#[ferment_macro::export]
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

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn from_string(encoded_value: &str, encoding: Encoding) -> Result<BinaryData, Error> {
        let vec = string_encoding::decode(encoded_value, encoding)?;

        Ok(BinaryData::new(vec))
    }

    pub fn from_string_with_encoding_string(
        encoded_value: &str,
        encoding_string: Option<&str>,
    ) -> Result<BinaryData, Error> {
        let encoding = encoding_string_to_encoding(encoding_string);

        BinaryData::from_string(encoded_value, encoding)
    }

    pub fn to_string(&self, encoding: Encoding) -> String {
        string_encoding::encode(&self.0, encoding)
    }

    pub fn to_string_with_encoding_string(&self, encoding_string: Option<&str>) -> String {
        let encoding = encoding_string_to_encoding(encoding_string);

        self.to_string(encoding)
    }
}

impl From<Vec<u8>> for BinaryData {
    fn from(value: Vec<u8>) -> Self {
        BinaryData::new(value)
    }
}

impl TryFrom<Value> for BinaryData {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.into_binary_data()
    }
}

impl TryFrom<&Value> for BinaryData {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value.to_binary_data()
    }
}

impl From<BinaryData> for Value {
    fn from(value: BinaryData) -> Self {
        Value::Bytes(value.0)
    }
}

impl From<&BinaryData> for Value {
    fn from(value: &BinaryData) -> Self {
        Value::Bytes(value.to_vec())
    }
}

impl TryFrom<String> for BinaryData {
    type Error = Error;

    fn try_from(data: String) -> Result<Self, Self::Error> {
        Self::from_string(&data, Encoding::Base64)
    }
}

impl From<BinaryData> for String {
    fn from(val: BinaryData) -> Self {
        val.to_string(Encoding::Base64)
    }
}

impl From<&BinaryData> for String {
    fn from(val: &BinaryData) -> Self {
        val.to_string(Encoding::Base64)
    }
}

impl PartialEq<&[u8; 20]> for BinaryData {
    fn eq(&self, other: &&[u8; 20]) -> bool {
        self.as_slice() == *other
    }
}

impl PartialEq<[u8; 20]> for BinaryData {
    fn eq(&self, other: &[u8; 20]) -> bool {
        self.as_slice() == *other
    }
}

impl PartialEq<&[u8]> for BinaryData {
    fn eq(&self, other: &&[u8]) -> bool {
        self.as_slice() == *other
    }
}

impl PartialEq<[u8]> for BinaryData {
    fn eq(&self, other: &[u8]) -> bool {
        self.as_slice() == other
    }
}

impl PartialEq<Vec<u8>> for BinaryData {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_slice() == other
    }
}

impl PartialEq<BinaryData> for Vec<u8> {
    fn eq(&self, other: &BinaryData) -> bool {
        other.as_slice() == self
    }
}

#[cfg(test)]
mod tests {
    use crate::{from_value, to_value, Identifier, Value};

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
