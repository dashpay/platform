use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{BorrowDecode, Decode, Encode};
use dashcore::blockdata::opcodes;
use std::fmt;
use std::ops::Deref;

use dashcore::{ScriptBuf as DashcoreScript, ScriptBuf};
use platform_value::string_encoding::{self, Encoding};
use rand::rngs::StdRng;
use rand::Rng;

use serde::de::Visitor;
use serde::{Deserialize, Serialize};

use crate::ProtocolError;
use bincode::de::read::Reader;

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct CoreScript(DashcoreScript);

impl CoreScript {
    pub fn new(script: DashcoreScript) -> Self {
        CoreScript(script)
    }

    pub fn to_string(&self, encoding: Encoding) -> String {
        string_encoding::encode(&self.0.to_bytes(), encoding)
    }

    pub fn from_string(encoded_value: &str, encoding: Encoding) -> Result<Self, ProtocolError> {
        let vec = string_encoding::decode(encoded_value, encoding)?;

        Ok(Self(vec.into()))
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes.into())
    }

    pub fn new_p2pkh(key_hash: [u8; 20]) -> Self {
        let mut bytes: Vec<u8> = vec![
            opcodes::all::OP_DUP.to_u8(),
            opcodes::all::OP_HASH160.to_u8(),
            opcodes::all::OP_PUSHBYTES_20.to_u8(),
        ];
        bytes.extend_from_slice(&key_hash);
        bytes.push(opcodes::all::OP_EQUALVERIFY.to_u8());
        bytes.push(opcodes::all::OP_CHECKSIG.to_u8());
        Self::from_bytes(bytes)
    }

    pub fn new_p2sh(script_hash: [u8; 20]) -> Self {
        let mut bytes = vec![
            opcodes::all::OP_HASH160.to_u8(),
            opcodes::all::OP_PUSHBYTES_20.to_u8(),
        ];
        bytes.extend_from_slice(&script_hash);
        bytes.push(opcodes::all::OP_EQUAL.to_u8());
        Self::from_bytes(bytes)
    }

    pub fn random_p2sh(rng: &mut StdRng) -> Self {
        Self::new_p2sh(rng.gen())
    }

    pub fn random_p2pkh(rng: &mut StdRng) -> Self {
        Self::new_p2pkh(rng.gen())
    }
}

impl From<Vec<u8>> for CoreScript {
    fn from(value: Vec<u8>) -> Self {
        CoreScript::from_bytes(value)
    }
}

impl Deref for CoreScript {
    type Target = DashcoreScript;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Implement the bincode::Encode trait for CoreScript
impl Encode for CoreScript {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.0.as_bytes().encode(encoder)
    }
}

// Implement the bincode::Decode trait for CoreScript
impl<C> Decode<C> for CoreScript {
    fn decode<D: Decoder<Context = C>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let bytes = Vec::<u8>::decode(decoder)?;
        // Create a CoreScript instance using the decoded DashCoreScript
        Ok(CoreScript(ScriptBuf(bytes)))
    }
}

impl<'de, C> BorrowDecode<'de, C> for CoreScript {
    fn borrow_decode<D: BorrowDecoder<'de, Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        // Read the serialized bytes from the decoder into a Vec<u8>
        let mut bytes = Vec::new();
        loop {
            let buf_len = 1024; // Adjust the buffer size as needed
            let mut buf = vec![0; buf_len];

            match decoder.reader().read(&mut buf) {
                Ok(()) => {
                    let read_bytes = buf.iter().position(|&x| x == 0).unwrap_or(buf.len());
                    bytes.extend_from_slice(&buf[..read_bytes]);
                    if read_bytes < buf_len {
                        break;
                    }
                }
                Err(DecodeError::Io { inner, additional })
                    if inner.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    if additional > 0 {
                        return Err(DecodeError::Io { inner, additional });
                    } else {
                        break;
                    }
                }
                Err(e) => return Err(e),
            }
        }

        // Convert Vec<u8> to Box<[u8]> and create a DashCoreScript instance
        let dash_core_script = DashcoreScript(bytes);

        // Create a CoreScript instance using the decoded DashCoreScript
        Ok(CoreScript(dash_core_script))
    }
}

impl Serialize for CoreScript {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string(Encoding::Base64))
        } else {
            serializer.serialize_bytes(self.as_bytes())
        }
    }
}

impl<'de> Deserialize<'de> for CoreScript {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Both visitors accept strings AND bytes regardless of the deserializer's
        // human-readable flag — same pattern as `BinaryData` / `Identifier`. This
        // covers the case where serde-wasm-bindgen emits a `Uint8Array` for the
        // Object form (which the deserializer reports as human-readable), and the
        // case where a JSON consumer sends a base64 string through the binary path.

        struct CoreScriptVisitor;

        impl Visitor<'_> for CoreScriptVisitor {
            type Value = CoreScript;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a byte array or base64-encoded string")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                CoreScript::from_string(v, Encoding::Base64).map_err(|e| {
                    E::custom(format!(
                        "expected to be able to deserialize core script from string: {}",
                        e
                    ))
                })
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                self.visit_str(&v)
            }

            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(CoreScript::from_bytes(v.to_vec()))
            }

            fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                Ok(CoreScript::from_bytes(v))
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_string(CoreScriptVisitor)
        } else {
            deserializer.deserialize_bytes(CoreScriptVisitor)
        }
    }
}

impl std::fmt::Display for CoreScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string(Encoding::Base64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::blockdata::opcodes;
    use platform_value::string_encoding::Encoding;

    mod construction {
        use super::*;

        #[test]
        fn from_bytes_creates_script() {
            let bytes = vec![1, 2, 3, 4, 5];
            let script = CoreScript::from_bytes(bytes.clone());
            assert_eq!(script.as_bytes(), &bytes);
        }

        #[test]
        fn new_wraps_dashcore_script() {
            let dashcore_script = DashcoreScript::from(vec![10, 20, 30]);
            let script = CoreScript::new(dashcore_script.clone());
            assert_eq!(script.as_bytes(), dashcore_script.as_bytes());
        }

        #[test]
        fn default_is_empty() {
            let script = CoreScript::default();
            assert!(script.as_bytes().is_empty());
        }

        #[test]
        fn from_vec_u8() {
            let bytes = vec![0xAA, 0xBB, 0xCC];
            let script: CoreScript = bytes.clone().into();
            assert_eq!(script.as_bytes(), &bytes);
        }
    }

    mod p2pkh {
        use super::*;

        #[test]
        fn new_p2pkh_has_correct_structure() {
            let key_hash = [0u8; 20];
            let script = CoreScript::new_p2pkh(key_hash);
            let bytes = script.as_bytes();

            // P2PKH script: OP_DUP OP_HASH160 OP_PUSHBYTES_20 <20 bytes> OP_EQUALVERIFY OP_CHECKSIG
            assert_eq!(bytes.len(), 25); // 3 + 20 + 2
            assert_eq!(bytes[0], opcodes::all::OP_DUP.to_u8());
            assert_eq!(bytes[1], opcodes::all::OP_HASH160.to_u8());
            assert_eq!(bytes[2], opcodes::all::OP_PUSHBYTES_20.to_u8());
            assert_eq!(&bytes[3..23], &key_hash);
            assert_eq!(bytes[23], opcodes::all::OP_EQUALVERIFY.to_u8());
            assert_eq!(bytes[24], opcodes::all::OP_CHECKSIG.to_u8());
        }

        #[test]
        fn new_p2pkh_with_nonzero_hash() {
            let key_hash: [u8; 20] = [
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
                0x0F, 0x10, 0x11, 0x12, 0x13, 0x14,
            ];
            let script = CoreScript::new_p2pkh(key_hash);
            let bytes = script.as_bytes();
            assert_eq!(&bytes[3..23], &key_hash);
        }

        #[test]
        fn two_different_key_hashes_produce_different_scripts() {
            let hash_a = [0xAA; 20];
            let hash_b = [0xBB; 20];
            let script_a = CoreScript::new_p2pkh(hash_a);
            let script_b = CoreScript::new_p2pkh(hash_b);
            assert_ne!(script_a, script_b);
        }
    }

    mod p2sh {
        use super::*;

        #[test]
        fn new_p2sh_has_correct_structure() {
            let script_hash = [0u8; 20];
            let script = CoreScript::new_p2sh(script_hash);
            let bytes = script.as_bytes();

            // P2SH script: OP_HASH160 OP_PUSHBYTES_20 <20 bytes> OP_EQUAL
            assert_eq!(bytes.len(), 23); // 2 + 20 + 1
            assert_eq!(bytes[0], opcodes::all::OP_HASH160.to_u8());
            assert_eq!(bytes[1], opcodes::all::OP_PUSHBYTES_20.to_u8());
            assert_eq!(&bytes[2..22], &script_hash);
            assert_eq!(bytes[22], opcodes::all::OP_EQUAL.to_u8());
        }

        #[test]
        fn new_p2sh_with_nonzero_hash() {
            let script_hash: [u8; 20] = [0xFF; 20];
            let script = CoreScript::new_p2sh(script_hash);
            let bytes = script.as_bytes();
            assert_eq!(&bytes[2..22], &script_hash);
        }

        #[test]
        fn p2pkh_and_p2sh_differ_for_same_hash() {
            let hash = [0x42; 20];
            let p2pkh = CoreScript::new_p2pkh(hash);
            let p2sh = CoreScript::new_p2sh(hash);
            assert_ne!(p2pkh, p2sh);
            // P2PKH is 25 bytes, P2SH is 23 bytes
            assert_eq!(p2pkh.as_bytes().len(), 25);
            assert_eq!(p2sh.as_bytes().len(), 23);
        }
    }

    mod string_encoding_round_trip {
        use super::*;

        #[test]
        fn base64_round_trip() {
            let original = CoreScript::new_p2pkh([0xAB; 20]);
            let encoded = original.to_string(Encoding::Base64);
            let decoded =
                CoreScript::from_string(&encoded, Encoding::Base64).expect("should decode base64");
            assert_eq!(original, decoded);
        }

        #[test]
        fn hex_round_trip() {
            let original = CoreScript::new_p2sh([0xCD; 20]);
            let encoded = original.to_string(Encoding::Hex);
            let decoded =
                CoreScript::from_string(&encoded, Encoding::Hex).expect("should decode hex");
            assert_eq!(original, decoded);
        }

        #[test]
        fn from_string_invalid_base64_fails() {
            let result = CoreScript::from_string("not-valid-base64!!!", Encoding::Base64);
            assert!(result.is_err());
        }

        #[test]
        fn display_uses_base64() {
            let script = CoreScript::new_p2pkh([0x00; 20]);
            let display_str = format!("{}", script);
            let encoded = script.to_string(Encoding::Base64);
            assert_eq!(display_str, encoded);
        }
    }

    mod from_bytes_round_trip {
        use super::*;

        #[test]
        fn bytes_round_trip() {
            let original_bytes = vec![1, 2, 3, 4, 5, 6, 7, 8];
            let script = CoreScript::from_bytes(original_bytes.clone());
            assert_eq!(script.as_bytes(), &original_bytes);
        }

        #[test]
        fn empty_bytes() {
            let script = CoreScript::from_bytes(vec![]);
            assert!(script.as_bytes().is_empty());
        }
    }

    mod deref {
        use super::*;

        #[test]
        fn deref_returns_inner_script() {
            let bytes = vec![1, 2, 3];
            let script = CoreScript::from_bytes(bytes.clone());
            // Deref gives us access to DashcoreScript methods
            let inner: &DashcoreScript = &script;
            assert_eq!(inner.as_bytes(), &bytes);
        }
    }

    mod equality_and_clone {
        use super::*;

        #[test]
        fn equal_scripts_are_equal() {
            let a = CoreScript::new_p2pkh([0x11; 20]);
            let b = CoreScript::new_p2pkh([0x11; 20]);
            assert_eq!(a, b);
        }

        #[test]
        fn different_scripts_are_not_equal() {
            let a = CoreScript::new_p2pkh([0x11; 20]);
            let b = CoreScript::new_p2pkh([0x22; 20]);
            assert_ne!(a, b);
        }

        #[test]
        fn clone_produces_equal_script() {
            let original = CoreScript::new_p2sh([0x33; 20]);
            let cloned = original.clone();
            assert_eq!(original, cloned);
        }
    }

    mod random_scripts {
        use super::*;
        use rand::SeedableRng;

        #[test]
        fn random_p2pkh_produces_valid_script() {
            let mut rng = StdRng::seed_from_u64(42);
            let script = CoreScript::random_p2pkh(&mut rng);
            let bytes = script.as_bytes();
            assert_eq!(bytes.len(), 25);
            assert_eq!(bytes[0], opcodes::all::OP_DUP.to_u8());
        }

        #[test]
        fn random_p2sh_produces_valid_script() {
            let mut rng = StdRng::seed_from_u64(42);
            let script = CoreScript::random_p2sh(&mut rng);
            let bytes = script.as_bytes();
            assert_eq!(bytes.len(), 23);
            assert_eq!(bytes[0], opcodes::all::OP_HASH160.to_u8());
        }

        #[test]
        fn two_random_scripts_differ() {
            let mut rng = StdRng::seed_from_u64(42);
            let a = CoreScript::random_p2pkh(&mut rng);
            let b = CoreScript::random_p2pkh(&mut rng);
            assert_ne!(a, b);
        }
    }
}
