//! Serde helper for variable-length `Vec<u8>` byte fields.
//!
//! Default serde serializes `Vec<u8>` as a sequence of u8 elements. For JS / wasm
//! consumers this is verbose and not ergonomic; we want bytes in binary formats
//! and base64 strings in human-readable formats — matching `BinaryData` (the
//! widely-used opaque-bytes wrapper in `rs-platform-value`) and the const-generic
//! `serde_bytes` helper.
//!
//! - **Human-readable** formats (JSON): base64-encoded string
//! - **Binary** formats (bincode / `platform_value`): raw byte sequence (which
//!   becomes `Uint8Array` through `serde_wasm_bindgen` with
//!   `serialize_bytes_as_arrays(false)`)

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt;

pub fn serialize<S: Serializer>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        serializer.serialize_str(&BASE64_STANDARD.encode(bytes))
    } else {
        serializer.serialize_bytes(bytes)
    }
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    // Accept all four input shapes — base64 string, byte buffer, byte slice,
    // and sequence of u8 — regardless of the deserializer's `is_human_readable`
    // flag. Required because serde's `ContentDeserializer` (used for internally
    // tagged enums like `#[serde(tag = "$formatVersion")]`) always reports
    // `is_human_readable: true`, so a value that started as bytes through a
    // non-HR deserializer can arrive at this visitor through any path.

    struct AnyShapeVisitor;

    impl<'de> Visitor<'de> for AnyShapeVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("bytes, sequence of u8, or base64-encoded string")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            Ok(v.to_vec())
        }

        fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
            Ok(v)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            BASE64_STANDARD
                .decode(v)
                .map_err(|e| E::custom(format!("expected base64 for bytes: {}", e)))
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(b) = seq.next_element::<u8>()? {
                bytes.push(b);
            }
            Ok(bytes)
        }
    }

    if deserializer.is_human_readable() {
        // `deserialize_any` covers true HR (serde_json string) AND
        // ContentDeserializer (which reports HR but may wrap bytes from a
        // non-HR source like platform_value).
        deserializer.deserialize_any(AnyShapeVisitor)
    } else {
        // Non-HR (bincode, platform_value): explicit shape hint.
        deserializer.deserialize_byte_buf(AnyShapeVisitor)
    }
}

#[cfg(test)]
mod tests {
    use base64::prelude::BASE64_STANDARD;
    use base64::Engine;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Wrap(#[serde(with = "super")] Vec<u8>);

    #[test]
    fn json_emits_base64_string() {
        let original = Wrap(vec![0xde, 0xad, 0xbe, 0xef]);
        let value = serde_json::to_value(&original).expect("serialize");
        assert_eq!(
            value,
            serde_json::json!(BASE64_STANDARD.encode([0xde, 0xad, 0xbe, 0xef]))
        );

        let restored: Wrap = serde_json::from_value(value).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn empty_vec_round_trips() {
        let original = Wrap(Vec::new());
        let value = serde_json::to_value(&original).expect("serialize empty");
        assert_eq!(value, serde_json::json!(""));
        let restored: Wrap = serde_json::from_value(value).expect("deserialize empty");
        assert_eq!(original, restored);
    }

    #[test]
    fn binary_round_trip_uses_raw_bytes() {
        let original = Wrap(vec![1, 2, 3, 4, 5]);
        let bytes = bincode::serde::encode_to_vec(&original, bincode::config::standard())
            .expect("bincode encode");
        let (restored, _): (Wrap, usize) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                .expect("bincode decode");
        assert_eq!(original, restored);
    }
}
