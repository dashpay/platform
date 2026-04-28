use crate::string_encoding::Encoding;
use crate::types::encoding_string_to_encoding;
use crate::{string_encoding, Error, Value};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bincode::{Decode, Encode};
use serde::de::Visitor;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Default, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Encode, Decode)]
pub struct BinaryData(pub Vec<u8>);

impl fmt::Debug for BinaryData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("BinaryData(0x{})", hex::encode(&self.0)))
    }
}

impl Serialize for BinaryData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&BASE64_STANDARD.encode(self.0.as_slice()))
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
        // Both visitors accept strings AND bytes regardless of the deserializer's
        // human-readable flag. Mirrors the same pattern used by `Identifier` /
        // `Bytes32` / etc.: serde's `ContentDeserializer` (used for internally
        // tagged enums like `#[serde(tag = "$version")]`) always reports
        // `is_human_readable: true`, so bytes can arrive through the string path
        // and vice versa. Without this, transitions whose Object form emits a
        // `Uint8Array` for a `BinaryData` field (e.g. `signature`) fail to
        // round-trip through `fromObject`.

        if deserializer.is_human_readable() {
            struct StringVisitor;

            impl Visitor<'_> for StringVisitor {
                type Value = BinaryData;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a base64-encoded string or byte array")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let bytes = BASE64_STANDARD.decode(v).map_err(|e| {
                        E::custom(format!("expected base64 for binary data: {}", e))
                    })?;
                    Ok(BinaryData(bytes))
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(BinaryData(v.to_vec()))
                }

                fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(BinaryData(v))
                }
            }

            deserializer.deserialize_string(StringVisitor)
        } else {
            struct BytesVisitor;

            impl Visitor<'_> for BytesVisitor {
                type Value = BinaryData;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a byte array or base64-encoded string")
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(BinaryData(v.to_vec()))
                }

                fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(BinaryData(v))
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let bytes = BASE64_STANDARD.decode(v).map_err(|e| {
                        E::custom(format!("expected base64 for binary data: {}", e))
                    })?;
                    Ok(BinaryData(bytes))
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

    /// Proves the **non-HR** path: bincode (`is_human_readable() == false`)
    /// dispatches `deserialize_bytes` → `visit_bytes`. Same path as
    /// `serde_wasm_bindgen::from_value` via the `dashpay/serde-wasm-bindgen` fork.
    #[test]
    fn binary_data_deserializes_bytes_through_non_human_readable_path() {
        let original = BinaryData::new(vec![0xde, 0xad, 0xbe, 0xef]);

        let bytes = bincode::serde::encode_to_vec(&original, bincode::config::standard())
            .expect("bincode encode");
        let (restored, _): (BinaryData, usize) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                .expect("bincode decode");
        assert_eq!(original, restored);
    }

    /// Proves the **nested-deserializer** path: when `BinaryData` lives inside an
    /// internally-tagged enum (`#[serde(tag = "type")]`), serde routes the inner
    /// field through `serde::__private::de::ContentDeserializer`, which reports
    /// `is_human_readable() == true` regardless of the outer format. So
    /// `BinaryData::deserialize` takes the HR branch (`deserialize_string`).
    /// The question: when the inner content is `Content::Bytes(...)` (because
    /// the source `Value` carries `Value::Bytes`), does `deserialize_string`
    /// dispatch to `visit_bytes` on `StringVisitor`?
    ///
    /// This is the path our wasm-dpp2 `fromObject` runs:
    /// `JsValue` → `platform_value::Value` (with `Value::Bytes` for byte fields)
    /// → `platform_value::from_value` → tagged enum `ContentDeserializer` →
    /// `BinaryData::deserialize` (HR=true) → `deserialize_string(StringVisitor)`.
    ///
    /// If CodeRabbit's concern is correct, this test fails.
    #[test]
    fn binary_data_deserializes_bytes_through_internally_tagged_enum() {
        use crate::Value;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        #[serde(tag = "type", rename_all = "camelCase")]
        enum Wrapper {
            Variant { signature: BinaryData },
        }

        // Build the tagged-enum shape by hand with a `Value::Bytes` for the
        // nested BinaryData field — exactly what platform_value_from_object
        // produces when a JS Object's byte field is a `Uint8Array`.
        let value = Value::Map(vec![
            (Value::Text("type".into()), Value::Text("variant".into())),
            (
                Value::Text("signature".into()),
                Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
            ),
        ]);

        let restored: Wrapper = crate::from_value(value).expect(
            "internally-tagged BinaryData should deserialize from Value::Bytes — \
             if this fails, CodeRabbit's `deserialize_string` blocks `visit_bytes` concern \
             is real for nested deserializers (ContentDeserializer reports HR=true).",
        );

        assert_eq!(
            restored,
            Wrapper::Variant {
                signature: BinaryData::new(vec![0xde, 0xad, 0xbe, 0xef]),
            }
        );
    }
}
