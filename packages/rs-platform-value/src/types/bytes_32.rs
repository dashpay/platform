use crate::string_encoding::Encoding;
use crate::types::encoding_string_to_encoding;
use crate::{string_encoding, Error, Value};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bincode::{Decode, Encode};
use rand::rngs::StdRng;
use rand::Rng;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Default, Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Copy, Encode, Decode)]
pub struct Bytes32(pub [u8; 32]);

impl AsRef<[u8]> for Bytes32 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Bytes32 {
    pub fn new(buffer: [u8; 32]) -> Self {
        Bytes32(buffer)
    }

    pub fn from_vec(buffer: Vec<u8>) -> Result<Self, Error> {
        let buffer: [u8; 32] = buffer.try_into().map_err(|_| {
            Error::ByteLengthNot32BytesError("buffer was not 32 bytes long".to_string())
        })?;
        Ok(Bytes32::new(buffer))
    }

    pub fn random_with_rng(rng: &mut StdRng) -> Self {
        Bytes32(rng.gen())
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn to_buffer(&self) -> [u8; 32] {
        self.0
    }

    pub fn from_string(encoded_value: &str, encoding: Encoding) -> Result<Bytes32, Error> {
        let vec = string_encoding::decode(encoded_value, encoding)?;

        Bytes32::from_vec(vec)
    }

    pub fn from_string_with_encoding_string(
        encoded_value: &str,
        encoding_string: Option<&str>,
    ) -> Result<Bytes32, Error> {
        let encoding = encoding_string_to_encoding(encoding_string);

        Bytes32::from_string(encoded_value, encoding)
    }

    pub fn to_string(&self, encoding: Encoding) -> String {
        string_encoding::encode(&self.0, encoding)
    }

    pub fn to_string_with_encoding_string(&self, encoding_string: Option<&str>) -> String {
        let encoding = encoding_string_to_encoding(encoding_string);

        self.to_string(encoding)
    }
}

impl Serialize for Bytes32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&BASE64_STANDARD.encode(self.0))
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for Bytes32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Both visitors accept strings AND bytes because serde's ContentDeserializer
        // (used for internally tagged enums like `#[serde(tag = "$version")]`) defaults
        // `is_human_readable` to `true` regardless of the parent deserializer's setting.
        // This means bytes can arrive through the string path and vice versa. Mirrors
        // the pattern used by `BinaryData` and `Identifier`.

        if deserializer.is_human_readable() {
            struct StringVisitor;

            impl Visitor<'_> for StringVisitor {
                type Value = Bytes32;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a base64-encoded string with length 44 or 32-byte array")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let bytes = BASE64_STANDARD
                        .decode(v)
                        .map_err(|e| E::custom(format!("expected base 64 for bytes32: {}", e)))?;
                    if bytes.len() != 32 {
                        return Err(E::invalid_length(bytes.len(), &self));
                    }
                    let mut array = [0u8; 32];
                    array.copy_from_slice(&bytes);
                    Ok(Bytes32(array))
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    if v.len() != 32 {
                        return Err(E::invalid_length(v.len(), &self));
                    }
                    let mut array = [0u8; 32];
                    array.copy_from_slice(v);
                    Ok(Bytes32(array))
                }
            }

            deserializer.deserialize_string(StringVisitor)
        } else {
            struct BytesVisitor;

            impl Visitor<'_> for BytesVisitor {
                type Value = Bytes32;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a 32-byte array or base64-encoded string")
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    if v.len() != 32 {
                        return Err(E::invalid_length(v.len(), &self));
                    }
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(v);
                    Ok(Bytes32(bytes))
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let bytes = BASE64_STANDARD
                        .decode(v)
                        .map_err(|e| E::custom(format!("expected base 64 for bytes32: {}", e)))?;
                    if bytes.len() != 32 {
                        return Err(E::invalid_length(bytes.len(), &self));
                    }
                    let mut array = [0u8; 32];
                    array.copy_from_slice(&bytes);
                    Ok(Bytes32(array))
                }
            }

            deserializer.deserialize_bytes(BytesVisitor)
        }
    }
}

impl TryFrom<Value> for Bytes32 {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.into_bytes_32()
    }
}

impl TryFrom<&Value> for Bytes32 {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value.to_bytes_32()
    }
}

impl From<[u8; 32]> for Bytes32 {
    fn from(value: [u8; 32]) -> Self {
        Bytes32(value)
    }
}

impl From<&[u8; 32]> for Bytes32 {
    fn from(value: &[u8; 32]) -> Self {
        Bytes32(*value)
    }
}

impl From<Bytes32> for Value {
    fn from(value: Bytes32) -> Self {
        Value::Bytes32(value.0)
    }
}

impl From<&Bytes32> for Value {
    fn from(value: &Bytes32) -> Self {
        Value::Bytes32(value.0)
    }
}

impl TryFrom<String> for Bytes32 {
    type Error = Error;

    fn try_from(data: String) -> Result<Self, Self::Error> {
        Self::from_string(&data, Encoding::Base64)
    }
}

impl From<Bytes32> for String {
    fn from(val: Bytes32) -> Self {
        val.to_string(Encoding::Base64)
    }
}

impl From<&Bytes32> for String {
    fn from(val: &Bytes32) -> Self {
        val.to_string(Encoding::Base64)
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
#[allow(clippy::clone_on_copy)]
#[allow(clippy::needless_borrows_for_generic_args)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn compute_hash<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    // ---------------------------------------------------------------
    // From<[u8; 32]> and Into<[u8; 32]> round-trip
    // ---------------------------------------------------------------

    #[test]
    fn from_array_round_trip() {
        let arr = [0xABu8; 32];
        let b = Bytes32::from(arr);
        assert_eq!(b.0, arr);
        let back: [u8; 32] = b.to_buffer();
        assert_eq!(back, arr);
    }

    #[test]
    fn from_ref_array() {
        let arr = [7u8; 32];
        let b = Bytes32::from(&arr);
        assert_eq!(b.0, arr);
    }

    // ---------------------------------------------------------------
    // from_vec — correct and wrong sizes
    // ---------------------------------------------------------------

    #[test]
    fn from_vec_correct_size() {
        let v = vec![1u8; 32];
        let b = Bytes32::from_vec(v).unwrap();
        assert_eq!(b.0, [1u8; 32]);
    }

    #[test]
    fn from_vec_too_short() {
        let v = vec![1u8; 31];
        let err = Bytes32::from_vec(v).unwrap_err();
        match err {
            Error::ByteLengthNot32BytesError(_) => {}
            other => panic!("expected ByteLengthNot32BytesError, got {:?}", other),
        }
    }

    #[test]
    fn from_vec_too_long() {
        let v = vec![1u8; 33];
        let err = Bytes32::from_vec(v).unwrap_err();
        match err {
            Error::ByteLengthNot32BytesError(_) => {}
            other => panic!("expected ByteLengthNot32BytesError, got {:?}", other),
        }
    }

    #[test]
    fn from_vec_empty() {
        let v = vec![];
        assert!(Bytes32::from_vec(v).is_err());
    }

    // ---------------------------------------------------------------
    // TryFrom<Value> for owned Value
    // ---------------------------------------------------------------

    #[test]
    fn try_from_value_bytes32() {
        let arr = [9u8; 32];
        let val = Value::Bytes32(arr);
        let b = Bytes32::try_from(val).unwrap();
        assert_eq!(b.0, arr);
    }

    #[test]
    fn try_from_value_bytes_correct_len() {
        let v = vec![3u8; 32];
        let val = Value::Bytes(v);
        let b = Bytes32::try_from(val).unwrap();
        assert_eq!(b.0, [3u8; 32]);
    }

    #[test]
    fn try_from_value_bytes_wrong_len() {
        let val = Value::Bytes(vec![1, 2, 3]);
        assert!(Bytes32::try_from(val).is_err());
    }

    #[test]
    fn try_from_value_identifier() {
        let arr = [0xCCu8; 32];
        let val = Value::Identifier(arr);
        let b = Bytes32::try_from(val).unwrap();
        assert_eq!(b.0, arr);
    }

    #[test]
    fn try_from_value_unsupported_variant() {
        let val = Value::Bool(false);
        assert!(Bytes32::try_from(val).is_err());
    }

    #[test]
    fn try_from_value_null_errors() {
        let val = Value::Null;
        assert!(Bytes32::try_from(val).is_err());
    }

    // ---------------------------------------------------------------
    // TryFrom<&Value> for borrowed Value
    // ---------------------------------------------------------------

    #[test]
    fn try_from_ref_value_bytes32() {
        let arr = [10u8; 32];
        let val = Value::Bytes32(arr);
        let b = Bytes32::try_from(&val).unwrap();
        assert_eq!(b.0, arr);
    }

    #[test]
    fn try_from_ref_value_bytes_correct_len() {
        let val = Value::Bytes(vec![4u8; 32]);
        let b = Bytes32::try_from(&val).unwrap();
        assert_eq!(b.0, [4u8; 32]);
    }

    #[test]
    fn try_from_ref_value_bytes_wrong_len() {
        let val = Value::Bytes(vec![1, 2]);
        assert!(Bytes32::try_from(&val).is_err());
    }

    #[test]
    fn try_from_ref_value_identifier() {
        let arr = [0xDDu8; 32];
        let val = Value::Identifier(arr);
        let b = Bytes32::try_from(&val).unwrap();
        assert_eq!(b.0, arr);
    }

    #[test]
    fn try_from_ref_value_unsupported() {
        let val = Value::Float(3.14);
        assert!(Bytes32::try_from(&val).is_err());
    }

    // ---------------------------------------------------------------
    // as_slice(), to_vec()
    // ---------------------------------------------------------------

    #[test]
    fn as_slice_returns_inner_bytes() {
        let arr = [0xFFu8; 32];
        let b = Bytes32::new(arr);
        assert_eq!(b.as_slice(), &arr[..]);
        assert_eq!(b.as_slice().len(), 32);
    }

    #[test]
    fn to_vec_returns_copy() {
        let arr = [5u8; 32];
        let b = Bytes32::new(arr);
        let v = b.to_vec();
        assert_eq!(v.len(), 32);
        assert_eq!(v, arr.to_vec());
    }

    #[test]
    fn as_ref_returns_inner_bytes() {
        let arr = [11u8; 32];
        let b = Bytes32::new(arr);
        let r: &[u8] = b.as_ref();
        assert_eq!(r, &arr[..]);
    }

    // ---------------------------------------------------------------
    // Hash impl: equal values hash equally, different values differ
    // ---------------------------------------------------------------

    #[test]
    fn hash_equal_values() {
        let a = Bytes32::new([1u8; 32]);
        let b = Bytes32::new([1u8; 32]);
        assert_eq!(compute_hash(&a), compute_hash(&b));
    }

    #[test]
    fn hash_different_values() {
        let a = Bytes32::new([1u8; 32]);
        let b = Bytes32::new([2u8; 32]);
        assert_ne!(compute_hash(&a), compute_hash(&b));
    }

    // ---------------------------------------------------------------
    // PartialOrd / Ord: ordering matches byte ordering
    // ---------------------------------------------------------------

    #[test]
    fn ordering_matches_byte_ordering() {
        let mut low = [0u8; 32];
        low[0] = 1;
        let mut high = [0u8; 32];
        high[0] = 2;
        let a = Bytes32::new(low);
        let b = Bytes32::new(high);
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn ordering_equal() {
        let a = Bytes32::new([5u8; 32]);
        let b = Bytes32::new([5u8; 32]);
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
    }

    #[test]
    fn ordering_last_byte_differs() {
        let mut low = [0u8; 32];
        low[31] = 1;
        let mut high = [0u8; 32];
        high[31] = 2;
        assert!(Bytes32::new(low) < Bytes32::new(high));
    }

    // ---------------------------------------------------------------
    // Default is all zeros
    // ---------------------------------------------------------------

    #[test]
    fn default_is_all_zeros() {
        let b = Bytes32::default();
        assert_eq!(b.0, [0u8; 32]);
    }

    // ---------------------------------------------------------------
    // Value round-trips
    // ---------------------------------------------------------------

    #[test]
    fn into_value_and_back() {
        let arr = [42u8; 32];
        let b = Bytes32::new(arr);
        let val: Value = b.into();
        assert_eq!(val, Value::Bytes32(arr));
        let back = Bytes32::try_from(val).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn ref_into_value() {
        let b = Bytes32::new([7u8; 32]);
        let val: Value = (&b).into();
        assert_eq!(val, Value::Bytes32([7u8; 32]));
    }

    // ---------------------------------------------------------------
    // String conversions
    // ---------------------------------------------------------------

    #[test]
    fn try_from_string_base64_round_trip() {
        let arr = [99u8; 32];
        let b = Bytes32::new(arr);
        let s: String = b.into();
        let recovered = Bytes32::try_from(s).unwrap();
        assert_eq!(recovered, Bytes32::new(arr));
    }

    #[test]
    fn try_from_string_invalid_base64() {
        let s = "not-valid-base64!!!".to_string();
        assert!(Bytes32::try_from(s).is_err());
    }

    #[test]
    fn ref_to_string() {
        let b = Bytes32::new([0u8; 32]);
        let s: String = (&b).into();
        let decoded = BASE64_STANDARD.decode(&s).unwrap();
        assert_eq!(decoded.len(), 32);
    }

    // ---------------------------------------------------------------
    // from_string with different encodings
    // ---------------------------------------------------------------

    #[test]
    fn from_string_base58_round_trip() {
        let arr = [0xABu8; 32];
        let b = Bytes32::new(arr);
        let encoded = b.to_string(Encoding::Base58);
        let recovered = Bytes32::from_string(&encoded, Encoding::Base58).unwrap();
        assert_eq!(recovered, b);
    }

    #[test]
    fn from_string_hex_round_trip() {
        let arr = [0xCDu8; 32];
        let b = Bytes32::new(arr);
        let encoded = b.to_string(Encoding::Hex);
        let recovered = Bytes32::from_string(&encoded, Encoding::Hex).unwrap();
        assert_eq!(recovered, b);
    }

    #[test]
    fn from_string_with_encoding_string_none_defaults_to_base58() {
        let arr = [0x01u8; 32];
        let b = Bytes32::new(arr);
        let encoded = b.to_string_with_encoding_string(None);
        let recovered = Bytes32::from_string_with_encoding_string(&encoded, None).unwrap();
        assert_eq!(recovered, b);
    }

    // ---------------------------------------------------------------
    // Serde round-trips
    // ---------------------------------------------------------------

    #[test]
    #[cfg(feature = "json")]
    fn serde_json_round_trip() {
        let arr = [0x12u8; 32];
        let b = Bytes32::new(arr);
        let json = serde_json::to_string(&b).unwrap();
        let recovered: Bytes32 = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, b);
    }

    #[test]
    fn serde_bincode_round_trip() {
        let arr = [0x34u8; 32];
        let b = Bytes32::new(arr);
        let config = bincode::config::standard();
        let encoded = bincode::encode_to_vec(&b, config).unwrap();
        let (decoded, _): (Bytes32, _) = bincode::decode_from_slice(&encoded, config).unwrap();
        assert_eq!(decoded, b);
    }

    // ---------------------------------------------------------------
    // Clone and Copy semantics
    // ---------------------------------------------------------------

    #[test]
    fn clone_is_equal() {
        let b = Bytes32::new([77u8; 32]);
        let c = b.clone();
        assert_eq!(b, c);
    }

    #[test]
    fn copy_semantics() {
        let b = Bytes32::new([88u8; 32]);
        let c = b; // Copy
        assert_eq!(b, c); // b is still valid because Bytes32 is Copy
    }

    // ---------------------------------------------------------------
    // Equality
    // ---------------------------------------------------------------

    #[test]
    fn equality_same_bytes() {
        let a = Bytes32::new([1u8; 32]);
        let b = Bytes32::new([1u8; 32]);
        assert_eq!(a, b);
    }

    #[test]
    fn inequality_different_bytes() {
        let a = Bytes32::new([1u8; 32]);
        let b = Bytes32::new([2u8; 32]);
        assert_ne!(a, b);
    }

    #[test]
    fn inequality_single_byte_diff() {
        let mut arr = [0u8; 32];
        let a = Bytes32::new(arr);
        arr[16] = 1;
        let b = Bytes32::new(arr);
        assert_ne!(a, b);
    }

    // ---------------------------------------------------------------
    // random_with_rng
    // ---------------------------------------------------------------

    #[test]
    fn random_with_rng_produces_non_zero() {
        let mut rng = StdRng::seed_from_u64(12345);
        let b = Bytes32::random_with_rng(&mut rng);
        // Extremely unlikely for 32 random bytes to all be zero
        assert_ne!(b.0, [0u8; 32]);
    }

    #[test]
    fn random_with_rng_deterministic_with_same_seed() {
        let mut rng1 = StdRng::seed_from_u64(42);
        let mut rng2 = StdRng::seed_from_u64(42);
        let a = Bytes32::random_with_rng(&mut rng1);
        let b = Bytes32::random_with_rng(&mut rng2);
        assert_eq!(a, b);
    }

    #[test]
    fn random_with_rng_different_seeds_differ() {
        let mut rng1 = StdRng::seed_from_u64(1);
        let mut rng2 = StdRng::seed_from_u64(2);
        let a = Bytes32::random_with_rng(&mut rng1);
        let b = Bytes32::random_with_rng(&mut rng2);
        assert_ne!(a, b);
    }
}
