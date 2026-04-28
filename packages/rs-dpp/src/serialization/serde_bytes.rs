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
use serde::{Deserialize, Deserializer, Serializer};

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
    if deserializer.is_human_readable() {
        let s = <String>::deserialize(deserializer)?;
        let vec = BASE64_STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)?;
        vec.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::custom(format!("expected {} bytes, got {}", N, v.len()))
        })
    } else {
        let vec = <Vec<u8>>::deserialize(deserializer)?;
        vec.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::custom(format!("expected {} bytes, got {}", N, v.len()))
        })
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
}
