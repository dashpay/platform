use crate::Error;
use base64;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use bs58;
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub enum Encoding {
    Base58,
    Base64,
    Hex,
}

pub const ALL_ENCODINGS: [Encoding; 3] = [Encoding::Hex, Encoding::Base58, Encoding::Base64];

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Encoding::Base58 => write!(f, "Base58"),
            Encoding::Base64 => write!(f, "Base64"),
            Encoding::Hex => write!(f, "Hex"),
        }
    }
}

pub fn decode(encoded_value: &str, encoding: Encoding) -> Result<Vec<u8>, Error> {
    match encoding {
        Encoding::Base58 => Ok(bs58::decode(encoded_value)
            .into_vec()
            .map_err(|e| Error::StringDecodingError(e.to_string()))?),
        Encoding::Base64 => Ok(BASE64_STANDARD
            .decode(encoded_value)
            .map_err(|e| Error::StringDecodingError(e.to_string()))?),
        Encoding::Hex => Ok(
            hex::decode(encoded_value).map_err(|e| Error::StringDecodingError(e.to_string()))?
        ),
    }
}

pub fn encode(value: &[u8], encoding: Encoding) -> String {
    match encoding {
        Encoding::Base58 => bs58::encode(value).into_string(),
        Encoding::Base64 => BASE64_STANDARD.encode(value),
        Encoding::Hex => hex::encode(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- encode/decode round-trips ----

    #[test]
    fn round_trip_base58_empty() {
        let data: &[u8] = &[];
        let encoded = encode(data, Encoding::Base58);
        let decoded = decode(&encoded, Encoding::Base58).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn round_trip_base58_non_empty() {
        let data = b"hello world";
        let encoded = encode(data, Encoding::Base58);
        let decoded = decode(&encoded, Encoding::Base58).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn round_trip_base58_binary() {
        let data: Vec<u8> = (0u8..=255).collect();
        let encoded = encode(&data, Encoding::Base58);
        let decoded = decode(&encoded, Encoding::Base58).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn round_trip_base64_empty() {
        let data: &[u8] = &[];
        let encoded = encode(data, Encoding::Base64);
        let decoded = decode(&encoded, Encoding::Base64).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn round_trip_base64_non_empty() {
        let data = b"hello world";
        let encoded = encode(data, Encoding::Base64);
        let decoded = decode(&encoded, Encoding::Base64).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn round_trip_base64_binary() {
        let data: Vec<u8> = (0u8..=255).collect();
        let encoded = encode(&data, Encoding::Base64);
        let decoded = decode(&encoded, Encoding::Base64).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn round_trip_hex_empty() {
        let data: &[u8] = &[];
        let encoded = encode(data, Encoding::Hex);
        let decoded = decode(&encoded, Encoding::Hex).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn round_trip_hex_non_empty() {
        let data = b"hello world";
        let encoded = encode(data, Encoding::Hex);
        let decoded = decode(&encoded, Encoding::Hex).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn round_trip_hex_binary() {
        let data: Vec<u8> = (0u8..=255).collect();
        let encoded = encode(&data, Encoding::Hex);
        let decoded = decode(&encoded, Encoding::Hex).unwrap();
        assert_eq!(decoded, data);
    }

    // ---- encode produces expected output ----

    #[test]
    fn encode_hex_known_value() {
        assert_eq!(encode(&[0xde, 0xad, 0xbe, 0xef], Encoding::Hex), "deadbeef");
    }

    #[test]
    fn encode_base64_known_value() {
        // "hello" -> "aGVsbG8="
        assert_eq!(encode(b"hello", Encoding::Base64), "aGVsbG8=");
    }

    #[test]
    fn encode_base58_known_value() {
        // bs58 of "Hello" is "9Ajdvzr"
        let encoded = encode(b"Hello", Encoding::Base58);
        assert_eq!(encoded, bs58::encode(b"Hello").into_string());
    }

    // ---- Display for Encoding ----

    #[test]
    fn display_encoding_base58() {
        assert_eq!(format!("{}", Encoding::Base58), "Base58");
    }

    #[test]
    fn display_encoding_base64() {
        assert_eq!(format!("{}", Encoding::Base64), "Base64");
    }

    #[test]
    fn display_encoding_hex() {
        assert_eq!(format!("{}", Encoding::Hex), "Hex");
    }

    // ---- ALL_ENCODINGS covers all variants ----

    #[test]
    fn all_encodings_has_three_entries() {
        assert_eq!(ALL_ENCODINGS.len(), 3);
    }

    // ---- error cases for invalid encoded strings ----

    #[test]
    fn decode_invalid_hex_returns_error() {
        // 'zz' is not valid hex
        let result = decode("zzzz", Encoding::Hex);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::StringDecodingError(msg) => {
                assert!(!msg.is_empty());
            }
            other => panic!("Expected StringDecodingError, got {:?}", other),
        }
    }

    #[test]
    fn decode_invalid_hex_odd_length_returns_error() {
        let result = decode("abc", Encoding::Hex);
        assert!(result.is_err());
    }

    #[test]
    fn decode_invalid_base64_returns_error() {
        // '!!!' is not valid base64
        let result = decode("!!!", Encoding::Base64);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::StringDecodingError(msg) => {
                assert!(!msg.is_empty());
            }
            other => panic!("Expected StringDecodingError, got {:?}", other),
        }
    }

    #[test]
    fn decode_invalid_base58_returns_error() {
        // '0OIl' contains characters invalid in base58
        let result = decode("0OIl", Encoding::Base58);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::StringDecodingError(msg) => {
                assert!(!msg.is_empty());
            }
            other => panic!("Expected StringDecodingError, got {:?}", other),
        }
    }

    // ---- round-trip through all encodings systematically ----

    #[test]
    fn round_trip_all_encodings_same_data() {
        let data = b"The quick brown fox jumps over the lazy dog";
        for encoding in &ALL_ENCODINGS {
            let encoded = encode(data, *encoding);
            let decoded = decode(&encoded, *encoding).unwrap();
            assert_eq!(decoded, data, "Round-trip failed for encoding {}", encoding);
        }
    }
}
