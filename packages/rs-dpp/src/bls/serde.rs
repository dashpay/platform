//! Serde support for Platform BLS keys in hex, byte-sequence and tagged-enum inputs.

use crate::bls::PublicKey as BlsPublicKey;
use ::serde::de::Visitor;
use ::serde::ser::SerializeTuple;
use ::serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Size of a compressed BLS12-381 G1 public key.
const COMPRESSED_G1_LEN: usize = 48;

impl Serialize for BlsPublicKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(self.to_bytes()))
        } else {
            let mut tuple = serializer.serialize_tuple(COMPRESSED_G1_LEN)?;
            for byte in self.to_bytes() {
                tuple.serialize_element(&byte)?;
            }
            tuple.end()
        }
    }
}

impl<'de> Deserialize<'de> for BlsPublicKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(BlsPublicKeyVisitor)
        } else {
            // Bincode needs the fixed tuple length and cannot use deserialize_any.
            deserializer.deserialize_tuple(COMPRESSED_G1_LEN, BlsPublicKeyVisitor)
        }
    }
}

/// Serialize a public-key field with the DPP key's standard representation.
pub fn serialize<S: Serializer>(pk: &BlsPublicKey, serializer: S) -> Result<S::Ok, S::Error> {
    pk.serialize(serializer)
}

/// Decode a field from self-describing or buffered tagged-enum data.
///
/// For formats such as bincode, use the key's `Deserialize` implementation instead.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BlsPublicKey, D::Error> {
    // Tagged enum buffers may supply either representation regardless of is_human_readable().
    deserializer.deserialize_any(BlsPublicKeyVisitor)
}

fn from_compressed_g1_bytes<E: ::serde::de::Error>(bytes: &[u8]) -> Result<BlsPublicKey, E> {
    if bytes.len() != COMPRESSED_G1_LEN {
        return Err(E::custom(format!(
            "expected {COMPRESSED_G1_LEN} compressed-G1 bytes for public key, got {}",
            bytes.len()
        )));
    }
    BlsPublicKey::try_from(bytes).map_err(|_| E::custom("not a valid compressed G1 point"))
}

struct BlsPublicKeyVisitor;

impl<'de> Visitor<'de> for BlsPublicKeyVisitor {
    type Value = BlsPublicKey;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "a {}-char hex string or {} compressed-G1 bytes",
            COMPRESSED_G1_LEN * 2,
            COMPRESSED_G1_LEN
        )
    }

    fn visit_str<E: ::serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
        if s.len() != COMPRESSED_G1_LEN * 2 {
            return Err(E::custom(format!(
                "expected {} hex chars for compressed G1 public key, got {}",
                COMPRESSED_G1_LEN * 2,
                s.len()
            )));
        }
        let mut bytes = [0u8; COMPRESSED_G1_LEN];
        for (i, slot) in bytes.iter_mut().enumerate() {
            let hi = hex_nibble(s.as_bytes()[i * 2]).map_err(E::custom)?;
            let lo = hex_nibble(s.as_bytes()[i * 2 + 1]).map_err(E::custom)?;
            *slot = (hi << 4) | lo;
        }
        from_compressed_g1_bytes(&bytes)
    }

    fn visit_bytes<E: ::serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        from_compressed_g1_bytes(v)
    }

    fn visit_seq<A: ::serde::de::SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> Result<Self::Value, A::Error> {
        let mut bytes = Vec::with_capacity(COMPRESSED_G1_LEN);
        while let Some(b) = seq.next_element::<u8>()? {
            // A valid compressed-G1 public key is exactly COMPRESSED_G1_LEN
            // bytes; reject as soon as a hostile payload exceeds that rather
            // than allocating/parsing an arbitrarily long sequence first.
            if bytes.len() == COMPRESSED_G1_LEN {
                return Err(::serde::de::Error::invalid_length(bytes.len() + 1, &self));
            }
            bytes.push(b);
        }
        from_compressed_g1_bytes(&bytes)
    }
}

fn hex_nibble(c: u8) -> Result<u8, &'static str> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err("invalid hex character in compressed G1 public key"),
    }
}

/// `Option<BlsPublicKey>` variant for fields like
/// `Validator::public_key`.
pub mod option {
    use super::*;

    pub fn serialize<S: Serializer>(
        opt: &Option<BlsPublicKey>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        // Option<T> delegates to the DPP public key's Serialize implementation.
        opt.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<BlsPublicKey>, D::Error> {
        struct OptionVisitor;

        impl<'de> Visitor<'de> for OptionVisitor {
            type Value = Option<BlsPublicKey>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("Option<BlsPublicKey>")
            }

            fn visit_none<E: ::serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_unit<E: ::serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_some<D2: Deserializer<'de>>(
                self,
                inner: D2,
            ) -> Result<Self::Value, D2::Error> {
                super::deserialize(inner).map(Some)
            }
        }

        deserializer.deserialize_option(OptionVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::serde::{Deserialize, Serialize};
    use serde_json::json;

    // A known-valid compressed-G1 BLS public key (deterministic — the
    // ValidatorSet fixture's seeded StdRng(42) threshold key).
    const PK_HEX: &str =
        "969c5d5873f49aa994c5f6a850924ca1840c4ad1791aaaecd90093d4a5c0c3799f2d98540f5366cfa0a33f143fd69263";

    // Newtypes that drive the `with` module(s) through serde.
    #[derive(Serialize, Deserialize)]
    struct Wrap(#[serde(with = "super")] BlsPublicKey);

    #[derive(Serialize, Deserialize)]
    struct OptWrap(#[serde(with = "super::option")] Option<BlsPublicKey>);

    #[test]
    fn should_accept_the_same_json_representations_directly_and_through_the_adapter() {
        let bytes = hex::decode(PK_HEX).unwrap();
        for value in [json!(PK_HEX), json!(PK_HEX.to_uppercase()), json!(bytes)] {
            let direct: BlsPublicKey = serde_json::from_value(value.clone()).unwrap();
            let adapted: Wrap = serde_json::from_value(value).unwrap();
            assert_eq!(direct, adapted.0);
            assert_eq!(direct.to_bytes().as_slice(), bytes);
        }
    }

    #[test]
    fn should_reject_malformed_keys_directly_and_through_the_adapter() {
        for value in [
            json!("ab"),
            json!("z".repeat(96)),
            json!(vec![0u8; 47]),
            json!(vec![0u8; 49]),
            json!(vec![0u8; 48]),
        ] {
            assert!(serde_json::from_value::<BlsPublicKey>(value.clone()).is_err());
            assert!(serde_json::from_value::<Wrap>(value).is_err());
        }
    }

    #[test]
    fn should_decode_bincode_keys_without_consuming_the_following_fields() {
        let key: BlsPublicKey = serde_json::from_value(json!(PK_HEX)).unwrap();
        let record = (key, 0x21u8, Some(key));
        let config = bincode::config::standard().with_big_endian();
        let encoded = bincode::serde::encode_to_vec(record, config).unwrap();
        let mut expected = key.to_bytes().to_vec();
        expected.extend([0x21, 1]);
        expected.extend(key.to_bytes());
        assert_eq!(encoded, expected);
        let (decoded, consumed): ((BlsPublicKey, u8, Option<BlsPublicKey>), usize) =
            bincode::serde::decode_from_slice(&encoded, config).unwrap();
        assert_eq!(decoded, record);
        assert_eq!(consumed, encoded.len());
        assert!(
            bincode::serde::decode_from_slice::<BlsPublicKey, _>(&encoded[..47], config).is_err()
        );
    }

    #[test]
    fn should_keep_legacy_field_adapter_paths_working() {
        #[derive(Serialize, Deserialize)]
        struct LegacyKey(
            #[serde(with = "crate::serialization::dashcore::bls_pubkey")] BlsPublicKey,
        );
        #[derive(Serialize, Deserialize)]
        struct LegacyOption(
            #[serde(with = "crate::serialization::dashcore::bls_pubkey::option")]
            Option<BlsPublicKey>,
        );

        let key: LegacyKey = serde_json::from_value(json!(PK_HEX)).unwrap();
        assert_eq!(serde_json::to_value(key).unwrap(), json!(PK_HEX));
        for value in [json!(PK_HEX), json!(null)] {
            let key: LegacyOption = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(key).unwrap(), value);
        }
    }

    #[test]
    fn json_hr_round_trip_is_the_hex_string() {
        // Human-readable (serde_json::Value): hex in, identical hex out (visit_str).
        let pk: Wrap = serde_json::from_value(json!(PK_HEX)).expect("from hex");
        assert_eq!(serde_json::to_value(&pk).expect("to json"), json!(PK_HEX));
    }

    #[test]
    fn json_borrowed_str_path_works() {
        // The shared visitor accepts borrowed strings as well as owned JSON values.
        let pk: Wrap = serde_json::from_str(&format!("\"{PK_HEX}\"")).expect("from_str");
        assert_eq!(serde_json::to_value(&pk).expect("to json"), json!(PK_HEX));
    }

    #[test]
    fn value_non_hr_byte_seq_round_trip() {
        // Non-HR `platform_value` serializes the key as a 48-byte sequence; the
        // shared visit_seq path must reconstruct it. Re-serialize to JSON to
        // confirm the same key.
        let pk: Wrap = serde_json::from_value(json!(PK_HEX)).expect("from hex");
        let value = platform_value::to_value(&pk).expect("to value");
        let pk2: Wrap = platform_value::from_value(value).expect("from value seq");
        assert_eq!(serde_json::to_value(&pk2).expect("to json"), json!(PK_HEX));
    }

    #[test]
    fn option_some_and_none_round_trip() {
        let some: OptWrap = serde_json::from_value(json!(PK_HEX)).expect("some");
        assert_eq!(serde_json::to_value(&some).expect("to json"), json!(PK_HEX));
        let none: OptWrap = serde_json::from_value(json!(null)).expect("none");
        assert_eq!(serde_json::to_value(&none).expect("to json"), json!(null));
    }
}
