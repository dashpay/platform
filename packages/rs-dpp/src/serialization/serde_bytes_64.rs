//! Serde helper for `[u8; 64]` fields.
//!
//! Serde's built-in array support only covers arrays up to 32 elements.
//! This module provides custom serialize/deserialize for 64-byte arrays
//! (e.g. RedPallas binding signatures, spend auth signatures).
//!
//! - **Human-readable** formats (JSON): hex-encoded string
//! - **Binary** formats (bincode, CBOR): raw byte sequence

use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S: Serializer>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        serializer.serialize_str(&hex::encode(bytes))
    } else {
        serializer.serialize_bytes(bytes)
    }
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[u8; 64], D::Error> {
    if deserializer.is_human_readable() {
        let s = <String>::deserialize(deserializer)?;
        let vec = hex::decode(&s).map_err(serde::de::Error::custom)?;
        vec.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::custom(format!("expected 64 bytes, got {}", v.len()))
        })
    } else {
        let vec = <Vec<u8>>::deserialize(deserializer)?;
        vec.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::custom(format!("expected 64 bytes, got {}", v.len()))
        })
    }
}
