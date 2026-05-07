//! Generic serde helper for fixed-size byte arrays `[u8; N]`.
//!
//! Default serde serializes `[u8; N]` for N ≤ 32 as a tuple of u8 elements
//! (a sequence of numbers in JSON, opaque in non-self-describing formats).
//! For N > 32 there is no default impl at all.
//!
//! This module gives a single, length-agnostic shape:
//!
//! - **Human-readable** formats (JSON): base64-encoded string (matches
//!   `Bytes20` / `Bytes32` / `Bytes36` / `BinaryData` in `rs-platform-value`)
//! - **Binary** formats (bincode, CBOR, `platform_value`): raw byte sequence
//!   (which becomes `Uint8Array` through `serde_wasm_bindgen` with
//!   `serialize_bytes_as_arrays(false)`)
//!
//! Used via `#[serde(with = "crate::serialization::serde_bytes")]` on any
//! `[u8; N]` field. The `#[json_safe_fields]` proc-macro injects this for
//! every fixed-size byte field.

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt;

pub fn serialize<S: Serializer, const N: usize>(
    bytes: &[u8; N],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        serializer.serialize_str(&BASE64_STANDARD.encode(bytes))
    } else {
        serializer.serialize_bytes(bytes)
    }
}

pub fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
    deserializer: D,
) -> Result<[u8; N], D::Error> {
    // Accept all four input shapes — base64 string, byte buffer, byte slice,
    // and sequence of u8 — regardless of the deserializer's `is_human_readable`
    // flag. Required because serde's `ContentDeserializer` (used for internally
    // tagged enums like `#[serde(tag = "$formatVersion")]`) always reports
    // `is_human_readable: true`, so a value that started as bytes through a
    // non-HR deserializer (platform_value, bincode) can arrive at this visitor
    // through the string path and vice versa. Mirrors the pattern used by
    // `platform_value::types::{bytes_32,binary_data,identifier}`.

    struct AnyShapeVisitor<const N: usize>;

    impl<'de, const N: usize> Visitor<'de> for AnyShapeVisitor<N> {
        type Value = [u8; N];

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{} bytes (as a byte buffer, sequence of u8, or base64-encoded string)",
                N
            )
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            v.try_into()
                .map_err(|_| E::custom(format!("expected {} bytes, got {}", N, v.len())))
        }

        fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
            let len = v.len();
            v.try_into()
                .map_err(|_| E::custom(format!("expected {} bytes, got {}", N, len)))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            let vec = BASE64_STANDARD
                .decode(v)
                .map_err(|e| E::custom(format!("expected base64-encoded {} bytes: {}", N, e)))?;
            self.visit_byte_buf(vec)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut buf = Vec::with_capacity(N);
            while let Some(b) = seq.next_element::<u8>()? {
                buf.push(b);
            }
            let len = buf.len();
            buf.try_into()
                .map_err(|_| de::Error::custom(format!("expected {} bytes, got {}", N, len)))
        }
    }

    if deserializer.is_human_readable() {
        // `deserialize_any` covers both true human-readable deserializers
        // (serde_json sees a string → `visit_str`) AND serde's
        // `ContentDeserializer` (which falsely reports `is_human_readable=true`
        // and may wrap `Content::ByteBuf` from a non-HR source like
        // platform_value → dispatches to `visit_bytes`).
        deserializer.deserialize_any(AnyShapeVisitor::<N>)
    } else {
        // Non-HR (bincode, platform_value): bincode is non-self-describing and
        // requires an explicit shape hint; `deserialize_byte_buf` is what works
        // for both bincode (length-prefixed bytes) and platform_value (Value::Bytes).
        deserializer.deserialize_byte_buf(AnyShapeVisitor::<N>)
    }
}

/// Serde helper for `Option<[u8; N]>` — wraps the parent module's
/// const-generic `[u8; N]` codec in `Option`-aware visitors.
///
/// Use via `#[serde(with = "crate::serialization::serde_bytes::option")]`.
/// `None` round-trips as `null` in JSON / `unit` in binary formats; `Some`
/// values use the parent module's base64-vs-bytes shape.
pub mod option {
    use serde::de::{self, Visitor};
    use serde::{Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S: Serializer, const N: usize>(
        value: &Option<[u8; N]>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        // Wrap the inner `[u8; N]` so we can call `serialize_some` and let the
        // outer serializer write the Option tag (None / Some). Calling
        // `super::serialize` directly with the inner serializer would bypass
        // the Option variant tag in non-self-describing formats like bincode.
        struct Inner<'a, const N: usize>(&'a [u8; N]);
        impl<'a, const N: usize> serde::Serialize for Inner<'a, N> {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                super::serialize(self.0, s)
            }
        }
        match value {
            Some(bytes) => serializer.serialize_some(&Inner::<N>(bytes)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
        deserializer: D,
    ) -> Result<Option<[u8; N]>, D::Error> {
        struct OptionVisitor<const N: usize>;

        impl<'de, const N: usize> Visitor<'de> for OptionVisitor<N> {
            type Value = Option<[u8; N]>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "optional {} bytes", N)
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_some<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                super::deserialize::<D, N>(deserializer).map(Some)
            }
        }

        deserializer.deserialize_option(OptionVisitor::<N>)
    }
}

#[cfg(test)]
mod tests {
    use base64::prelude::BASE64_STANDARD;
    use base64::Engine;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Wrap32(#[serde(with = "super")] [u8; 32]);

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Wrap64(#[serde(with = "super")] [u8; 64]);

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Wrap20(#[serde(with = "super")] [u8; 20]);

    #[test]
    fn json_round_trip_32_bytes_uses_base64_string() {
        let original = Wrap32([0xab; 32]);
        let value = serde_json::to_value(&original).expect("serialize");
        assert_eq!(value, serde_json::json!(BASE64_STANDARD.encode([0xab; 32])));
        let restored: Wrap32 = serde_json::from_value(value).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn json_round_trip_64_bytes_uses_base64_string() {
        let original = Wrap64([0xcd; 64]);
        let value = serde_json::to_value(&original).expect("serialize");
        assert_eq!(value, serde_json::json!(BASE64_STANDARD.encode([0xcd; 64])));
        let restored: Wrap64 = serde_json::from_value(value).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn json_round_trip_20_bytes_works_with_const_generic() {
        let original = Wrap20([0x12; 20]);
        let value = serde_json::to_value(&original).expect("serialize");
        assert_eq!(value, serde_json::json!(BASE64_STANDARD.encode([0x12; 20])));
        let restored: Wrap20 = serde_json::from_value(value).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn rejects_wrong_length_base64() {
        let result: Result<Wrap32, _> =
            serde_json::from_value(serde_json::json!(BASE64_STANDARD.encode([0u8; 8])));
        assert!(result.is_err());
    }

    #[test]
    fn binary_round_trip_uses_raw_bytes() {
        let original = Wrap32([0x55; 32]);
        let bytes = bincode::serde::encode_to_vec(&original, bincode::config::standard())
            .expect("bincode encode");
        let (restored, _): (Wrap32, usize) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                .expect("bincode decode");
        assert_eq!(original, restored);
    }

    // --- option submodule --------------------------------------------------

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct OptWrap32(#[serde(with = "super::option")] Option<[u8; 32]>);

    #[test]
    fn option_some_json_round_trip() {
        let original = OptWrap32(Some([0xab; 32]));
        let value = serde_json::to_value(&original).expect("serialize");
        assert_eq!(value, serde_json::json!(BASE64_STANDARD.encode([0xab; 32])));
        let restored: OptWrap32 = serde_json::from_value(value).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn option_none_json_round_trip() {
        let original = OptWrap32(None);
        let value = serde_json::to_value(&original).expect("serialize");
        assert_eq!(value, serde_json::Value::Null);
        let restored: OptWrap32 = serde_json::from_value(value).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn option_some_binary_round_trip() {
        let original = OptWrap32(Some([0x77; 32]));
        let bytes = bincode::serde::encode_to_vec(&original, bincode::config::standard())
            .expect("bincode encode");
        let (restored, _): (OptWrap32, usize) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                .expect("bincode decode");
        assert_eq!(original, restored);
    }
}
