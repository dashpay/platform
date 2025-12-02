use dash_sdk::dpp::platform_value::string_encoding::{decode, encode, Encoding};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serializer};
use std::fmt;

pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if serializer.is_human_readable() {
        let s = encode(bytes, Encoding::Base64);
        serializer.serialize_str(&s)
    } else {
        serializer.serialize_bytes(bytes)
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    if deserializer.is_human_readable() {
        let s = String::deserialize(deserializer)?;
        decode(&s, Encoding::Base64).map_err(de::Error::custom)
    } else {
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
