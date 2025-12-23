use dpp::platform_value::string_encoding::{Encoding, decode, encode};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serializer};
use std::fmt;

pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if serializer.is_human_readable() {
        // JSON, YAML, etc. → Base64 string
        let s = encode(bytes, Encoding::Base64);
        serializer.serialize_str(&s)
    } else {
        // Binary / wasm / serde_wasm_bindgen → real bytes
        serializer.serialize_bytes(bytes)
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    if deserializer.is_human_readable() {
        // Expect Base64 string in JSON
        let s = String::deserialize(deserializer)?;
        decode(&s, Encoding::Base64).map_err(de::Error::custom)
    } else {
        // Expect bytes for binary formats / serde_wasm_bindgen
        struct BytesVisitor;

        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(v.to_vec())
            }
        }

        deserializer.deserialize_bytes(BytesVisitor)
    }
}

/// Generic serde helper for `Option<[u8; N]>` as base64
pub mod option {
    use super::*;

    pub fn serialize<S, const N: usize>(
        value: &Option<[u8; N]>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(bytes) => super::serialize(bytes.as_slice(), serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D, const N: usize>(
        deserializer: D,
    ) -> Result<Option<[u8; N]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OptionVisitor<const N: usize>;

        impl<'de, const N: usize> Visitor<'de> for OptionVisitor<N> {
            type Value = Option<[u8; N]>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "optional {} bytes", N)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(None)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                let bytes = super::deserialize(deserializer)?;
                if bytes.len() == N {
                    let mut arr = [0u8; N];
                    arr.copy_from_slice(&bytes);
                    Ok(Some(arr))
                } else {
                    Err(de::Error::custom(format!(
                        "expected {} bytes, got {}",
                        N,
                        bytes.len()
                    )))
                }
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(None)
            }
        }

        deserializer.deserialize_option(OptionVisitor::<N>)
    }
}
