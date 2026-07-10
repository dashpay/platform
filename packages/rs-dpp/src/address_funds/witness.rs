#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Maximum number of entries in a P2SH signatures vector.
/// This is 16 (max keys from OP_PUSHNUM_16) + 1 (CHECKMULTISIG dummy byte).
pub const MAX_P2SH_SIGNATURES: usize = 17;

/// The input witness data required to spend from a PlatformAddress.
///
/// This enum captures the different spending patterns for P2PKH and P2SH addresses.
///
/// Wire shape (internally tagged on `type`, camelCase variants/fields):
///   `{ "$type": "p2pkh", "signature": <BinaryData> }`
///   `{ "$type": "p2sh", "signatures": [<BinaryData>, ...], "redeemScript": <BinaryData> }`
///
/// Note: `MAX_P2SH_SIGNATURES` is enforced by the bincode `Decode` path (the
/// load-bearing wire format). The serde JSON/Value deserialize path does not
/// enforce it; downstream consumers must validate signature counts before
/// re-serializing for storage.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$type")
)]
pub enum AddressWitness {
    /// P2PKH witness: recoverable signature only
    ///
    /// Used for spending from a Pay-to-Public-Key-Hash address.
    /// The public key is recovered from the signature during verification,
    /// saving 33 bytes per witness compared to including the public key.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "p2pkh"))]
    P2pkh {
        /// The recoverable ECDSA signature (65 bytes with recovery byte prefix)
        signature: BinaryData, //todo change to [u8;65]
    },
    /// P2SH witness: signatures + redeem script
    ///
    /// Used for spending from a Pay-to-Script-Hash address (e.g., multisig).
    /// For a 2-of-3 multisig, signatures would be `[OP_0, sig1, sig2]` and
    /// redeem_script would be `OP_2 <pub1> <pub2> <pub3> OP_3 OP_CHECKMULTISIG`.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "p2sh"))]
    P2sh {
        /// The signatures (may include placeholder bytes like OP_0 for CHECKMULTISIG bug)
        signatures: Vec<BinaryData>,
        /// The redeem script that hashes to the address
        #[cfg_attr(feature = "serde-conversion", serde(rename = "redeemScript"))]
        redeem_script: BinaryData,
    },
}

impl Encode for AddressWitness {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            AddressWitness::P2pkh { signature } => {
                0u8.encode(encoder)?;
                signature.encode(encoder)?;
            }
            AddressWitness::P2sh {
                signatures,
                redeem_script,
            } => {
                1u8.encode(encoder)?;
                signatures.encode(encoder)?;
                redeem_script.encode(encoder)?;
            }
        }
        Ok(())
    }
}

impl<C> Decode<C> for AddressWitness {
    fn decode<D: Decoder<Context = C>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let discriminant = u8::decode(decoder)?;
        match discriminant {
            0 => {
                let signature = BinaryData::decode(decoder)?;
                Ok(AddressWitness::P2pkh { signature })
            }
            1 => {
                let signatures = Vec::<BinaryData>::decode(decoder)?;
                if signatures.len() > MAX_P2SH_SIGNATURES {
                    return Err(DecodeError::OtherString(format!(
                        "P2SH signatures count {} exceeds maximum {}",
                        signatures.len(),
                        MAX_P2SH_SIGNATURES,
                    )));
                }
                let redeem_script = BinaryData::decode(decoder)?;
                Ok(AddressWitness::P2sh {
                    signatures,
                    redeem_script,
                })
            }
            _ => Err(DecodeError::OtherString(format!(
                "Invalid AddressWitness discriminant: {}",
                discriminant
            ))),
        }
    }
}

impl<'de, C> bincode::BorrowDecode<'de, C> for AddressWitness {
    fn borrow_decode<D: BorrowDecoder<'de, Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        let discriminant = u8::borrow_decode(decoder)?;
        match discriminant {
            0 => {
                let signature = BinaryData::borrow_decode(decoder)?;
                Ok(AddressWitness::P2pkh { signature })
            }
            1 => {
                let signatures = Vec::<BinaryData>::borrow_decode(decoder)?;
                if signatures.len() > MAX_P2SH_SIGNATURES {
                    return Err(DecodeError::OtherString(format!(
                        "P2SH signatures count {} exceeds maximum {}",
                        signatures.len(),
                        MAX_P2SH_SIGNATURES,
                    )));
                }
                let redeem_script = BinaryData::borrow_decode(decoder)?;
                Ok(AddressWitness::P2sh {
                    signatures,
                    redeem_script,
                })
            }
            _ => Err(DecodeError::OtherString(format!(
                "Invalid AddressWitness discriminant: {}",
                discriminant
            ))),
        }
    }
}

impl AddressWitness {
    /// Generates a unique identifier for this witness based on its contents.
    ///
    /// This is used for deduplication purposes in unique_identifiers() implementations.
    pub fn unique_id(&self) -> String {
        use base64::prelude::BASE64_STANDARD;
        use base64::Engine;

        let mut data = Vec::new();

        match self {
            AddressWitness::P2pkh { signature } => {
                data.push(0u8);
                data.extend_from_slice(signature.as_slice());
            }
            AddressWitness::P2sh {
                signatures,
                redeem_script,
            } => {
                data.push(1u8);
                data.extend_from_slice(redeem_script.as_slice());
                for sig in signatures {
                    data.extend_from_slice(sig.as_slice());
                }
            }
        }

        BASE64_STANDARD.encode(&data)
    }

    /// Returns the redeem script if this is a P2SH witness
    pub fn redeem_script(&self) -> Option<&BinaryData> {
        match self {
            AddressWitness::P2pkh { .. } => None,
            AddressWitness::P2sh { redeem_script, .. } => Some(redeem_script),
        }
    }

    /// Returns true if this is a P2PKH witness
    pub fn is_p2pkh(&self) -> bool {
        matches!(self, AddressWitness::P2pkh { .. })
    }

    /// Returns true if this is a P2SH witness
    pub fn is_p2sh(&self) -> bool {
        matches!(self, AddressWitness::P2sh { .. })
    }
}

#[cfg(test)]
#[allow(clippy::needless_borrows_for_generic_args)]
mod tests {
    use super::*;
    use bincode::config;

    #[test]
    fn test_p2pkh_witness_encode_decode() {
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let decoded: AddressWitness = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness, decoded);
        assert!(decoded.is_p2pkh());
        assert!(!decoded.is_p2sh());
    }

    #[test]
    fn test_p2sh_witness_encode_decode() {
        let witness = AddressWitness::P2sh {
            signatures: vec![
                BinaryData::new(vec![0x00]),                   // OP_0 placeholder
                BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]), // sig1
                BinaryData::new(vec![0x30, 0x45, 0x02, 0x21]), // sig2
            ],
            redeem_script: BinaryData::new(vec![
                0x52, // OP_2
                0x21, // push 33 bytes (pubkey1)
                0x02, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12,
                0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12,
                0x12, 0x12, 0x12, 0x12, 0x12, 0x53, // OP_3
                0xae, // OP_CHECKMULTISIG
            ]),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let decoded: AddressWitness = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness, decoded);
        assert!(!decoded.is_p2pkh());
        assert!(decoded.is_p2sh());
    }

    #[test]
    fn test_unique_id_p2pkh() {
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x44]),
        };

        let id = witness.unique_id();
        assert!(!id.is_empty());

        // Different signature should produce different ID
        let witness2 = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x45]),
        };
        assert_ne!(id, witness2.unique_id());
    }

    #[test]
    fn test_unique_id_p2sh() {
        let witness = AddressWitness::P2sh {
            signatures: vec![
                BinaryData::new(vec![0x00]),
                BinaryData::new(vec![0x30, 0x44]),
            ],
            redeem_script: BinaryData::new(vec![0x52, 0xae]),
        };

        let id = witness.unique_id();
        assert!(!id.is_empty());

        // Different redeem script should produce different ID
        let witness2 = AddressWitness::P2sh {
            signatures: vec![
                BinaryData::new(vec![0x00]),
                BinaryData::new(vec![0x30, 0x44]),
            ],
            redeem_script: BinaryData::new(vec![0x53, 0xae]),
        };
        assert_ne!(id, witness2.unique_id());
    }

    #[cfg(feature = "serde-conversion")]
    #[test]
    fn test_p2pkh_serde() {
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]),
        };

        let json = serde_json::to_string(&witness).unwrap();
        let deserialized: AddressWitness = serde_json::from_str(&json).unwrap();

        assert_eq!(witness, deserialized);
    }

    #[cfg(feature = "serde-conversion")]
    #[test]
    fn test_p2sh_serde() {
        let witness = AddressWitness::P2sh {
            signatures: vec![
                BinaryData::new(vec![0x00]),
                BinaryData::new(vec![0x30, 0x44]),
            ],
            redeem_script: BinaryData::new(vec![0x52, 0xae]),
        };

        let json = serde_json::to_string(&witness).unwrap();
        let deserialized: AddressWitness = serde_json::from_str(&json).unwrap();

        assert_eq!(witness, deserialized);
    }

    /// AUDIT L1: Unbounded P2SH witness size during deserialization.
    ///
    /// The `Decode` impl for `AddressWitness::P2sh` now enforces
    /// `MAX_P2SH_SIGNATURES` during deserialization. A payload with more
    /// signatures than the limit is rejected with a decode error.
    ///
    /// Location: rs-dpp/src/address_funds/witness.rs
    #[test]
    fn test_p2sh_witness_rejects_excessive_signatures() {
        // Create a P2SH witness with 1000 signatures — far above MAX_P2SH_SIGNATURES
        let num_signatures = 1000;
        let signatures: Vec<BinaryData> = (0..num_signatures)
            .map(|i| BinaryData::new(vec![0x30, 0x44, i as u8]))
            .collect();

        let witness = AddressWitness::P2sh {
            signatures,
            redeem_script: BinaryData::new(vec![0x52, 0xae]),
        };

        // Encode succeeds (encoding has no limit), but decode must reject
        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let result: Result<(AddressWitness, usize), _> =
            bincode::decode_from_slice(&encoded, config::standard());

        assert!(
            result.is_err(),
            "AUDIT L1: P2SH witness with {} signatures should be rejected during \
            deserialization. MAX_P2SH_SIGNATURES = {}.",
            num_signatures,
            MAX_P2SH_SIGNATURES,
        );
    }

    /// AUDIT L3: No maximum length check on P2SH signatures vector.
    ///
    /// The deserialization now enforces `MAX_P2SH_SIGNATURES` (17). Signature
    /// counts above this limit are rejected during decode. The boundary value
    /// (17) is accepted, and 18+ is rejected.
    ///
    /// Location: rs-dpp/src/address_funds/witness.rs
    #[test]
    fn test_p2sh_witness_max_signatures_boundary() {
        // Counts above MAX_P2SH_SIGNATURES should be rejected during decode
        for count in [50, 100, 500] {
            let signatures: Vec<BinaryData> = (0..count)
                .map(|_| BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]))
                .collect();

            let witness = AddressWitness::P2sh {
                signatures,
                redeem_script: BinaryData::new(vec![0x52, 0xae]),
            };

            let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
            let result: Result<(AddressWitness, usize), _> =
                bincode::decode_from_slice(&encoded, config::standard());

            assert!(
                result.is_err(),
                "AUDIT L3: P2SH witness with {} signatures should be rejected during \
                deserialization. MAX_P2SH_SIGNATURES = {}.",
                count,
                MAX_P2SH_SIGNATURES,
            );
        }

        // MAX_P2SH_SIGNATURES (17) should be accepted
        let signatures: Vec<BinaryData> = (0..MAX_P2SH_SIGNATURES)
            .map(|_| BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]))
            .collect();

        let witness = AddressWitness::P2sh {
            signatures,
            redeem_script: BinaryData::new(vec![0x52, 0xae]),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let decoded: AddressWitness = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness, decoded);

        // MAX_P2SH_SIGNATURES + 1 should be rejected
        let signatures: Vec<BinaryData> = (0..MAX_P2SH_SIGNATURES + 1)
            .map(|_| BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]))
            .collect();

        let witness = AddressWitness::P2sh {
            signatures,
            redeem_script: BinaryData::new(vec![0x52, 0xae]),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let result: Result<(AddressWitness, usize), _> =
            bincode::decode_from_slice(&encoded, config::standard());

        assert!(
            result.is_err(),
            "P2SH witness with {} signatures (MAX + 1) should be rejected",
            MAX_P2SH_SIGNATURES + 1,
        );
    }

    // --- Additional encode/decode round-trip tests ---

    #[test]
    fn test_p2pkh_empty_signature_round_trip() {
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![]),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let decoded: AddressWitness = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness, decoded);
        assert!(decoded.is_p2pkh());
    }

    #[test]
    fn test_p2pkh_65_byte_signature_round_trip() {
        // Typical recoverable ECDSA signature is 65 bytes
        let signature_data: Vec<u8> = (0..65).collect();
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(signature_data),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let decoded: AddressWitness = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness, decoded);
    }

    #[test]
    fn test_p2sh_single_signature_round_trip() {
        let witness = AddressWitness::P2sh {
            signatures: vec![BinaryData::new(vec![0x30, 0x44, 0x02, 0x20])],
            redeem_script: BinaryData::new(vec![0x51, 0xae]),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let decoded: AddressWitness = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness, decoded);
        assert!(decoded.is_p2sh());
        assert_eq!(
            decoded.redeem_script(),
            Some(&BinaryData::new(vec![0x51, 0xae]))
        );
    }

    #[test]
    fn test_p2sh_empty_signatures_vec_round_trip() {
        let witness = AddressWitness::P2sh {
            signatures: vec![],
            redeem_script: BinaryData::new(vec![0x52, 0xae]),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let decoded: AddressWitness = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness, decoded);
    }

    #[test]
    fn test_p2sh_empty_redeem_script_round_trip() {
        let witness = AddressWitness::P2sh {
            signatures: vec![BinaryData::new(vec![0x00])],
            redeem_script: BinaryData::new(vec![]),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let decoded: AddressWitness = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness, decoded);
    }

    // --- Error path tests ---

    #[test]
    fn test_invalid_discriminant_decode_fails() {
        // Manually craft a payload with discriminant 2 (invalid)
        let mut data = vec![];
        bincode::encode_into_std_write(&2u8, &mut data, config::standard()).unwrap();
        // Add some dummy data
        data.extend_from_slice(&[0x00, 0x00, 0x00]);

        let result: Result<(AddressWitness, usize), _> =
            bincode::decode_from_slice(&data, config::standard());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Invalid AddressWitness discriminant"));
    }

    #[test]
    fn test_invalid_discriminant_255_decode_fails() {
        let mut data = vec![];
        bincode::encode_into_std_write(&255u8, &mut data, config::standard()).unwrap();

        let result: Result<(AddressWitness, usize), _> =
            bincode::decode_from_slice(&data, config::standard());
        assert!(result.is_err());
    }

    #[test]
    fn test_truncated_p2pkh_payload_fails() {
        // Encode only the discriminant, no signature data
        let data = vec![0u8]; // discriminant for P2pkh
        let result: Result<(AddressWitness, usize), _> =
            bincode::decode_from_slice(&data, config::standard());
        assert!(result.is_err());
    }

    #[test]
    fn test_truncated_p2sh_payload_fails() {
        // Encode discriminant for P2sh but no signatures/redeem_script
        let data = vec![1u8]; // discriminant for P2sh
        let result: Result<(AddressWitness, usize), _> =
            bincode::decode_from_slice(&data, config::standard());
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_payload_fails() {
        let data: Vec<u8> = vec![];
        let result: Result<(AddressWitness, usize), _> =
            bincode::decode_from_slice(&data, config::standard());
        assert!(result.is_err());
    }

    // --- Accessor tests ---

    #[test]
    fn test_redeem_script_returns_none_for_p2pkh() {
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30]),
        };
        assert!(witness.redeem_script().is_none());
    }

    #[test]
    fn test_redeem_script_returns_some_for_p2sh() {
        let script = BinaryData::new(vec![0x52, 0xae]);
        let witness = AddressWitness::P2sh {
            signatures: vec![],
            redeem_script: script.clone(),
        };
        assert_eq!(witness.redeem_script(), Some(&script));
    }

    // --- BorrowDecode path tests ---

    #[test]
    fn test_borrow_decode_p2pkh_round_trip() {
        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0xAB, 0xCD, 0xEF]),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        // borrow_decode is exercised through decode_from_slice
        let decoded: AddressWitness = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;
        assert_eq!(witness, decoded);
    }

    #[test]
    fn test_borrow_decode_p2sh_round_trip() {
        let witness = AddressWitness::P2sh {
            signatures: vec![
                BinaryData::new(vec![0x00]),
                BinaryData::new(vec![0x30, 0x44]),
                BinaryData::new(vec![0x30, 0x45]),
            ],
            redeem_script: BinaryData::new(vec![0x52, 0x53, 0xae]),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let decoded: AddressWitness = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;
        assert_eq!(witness, decoded);
    }

    #[test]
    fn test_borrow_decode_rejects_excessive_signatures() {
        // Ensure BorrowDecode also rejects > MAX_P2SH_SIGNATURES
        let signatures: Vec<BinaryData> = (0..MAX_P2SH_SIGNATURES + 1)
            .map(|_| BinaryData::new(vec![0x30]))
            .collect();

        let witness = AddressWitness::P2sh {
            signatures,
            redeem_script: BinaryData::new(vec![0xae]),
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let result: Result<(AddressWitness, usize), _> =
            bincode::decode_from_slice(&encoded, config::standard());
        assert!(result.is_err());
    }

    #[test]
    fn test_borrow_decode_invalid_discriminant_fails() {
        let mut data = vec![];
        bincode::encode_into_std_write(&3u8, &mut data, config::standard()).unwrap();
        data.extend_from_slice(&[0x00; 10]);

        let result: Result<(AddressWitness, usize), _> =
            bincode::decode_from_slice(&data, config::standard());
        assert!(result.is_err());
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for AddressWitness {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for AddressWitness {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::{platform_value, BinaryData};
    use serde_json::json;

    // `AddressWitness` has a manual Serialize/Deserialize that emits a
    // `{ "$type": "p2pkh"|"p2sh", ... }` discriminator shape. `BinaryData` is
    // base64-encoded in JSON (HR), and stored as `Value::Bytes` in non-HR.

    #[test]
    fn json_round_trip_p2pkh_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0xa1; 65]),
        };
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$type": "p2pkh",
                "signature": "oaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaE=",
            })
        );
        let recovered = AddressWitness::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_p2sh_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = AddressWitness::P2sh {
            redeem_script: BinaryData::new(vec![0xb2; 30]),
            signatures: vec![BinaryData::new(vec![0xc3; 65])],
        };
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$type": "p2sh",
                "signatures": [
                    "w8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8PDw8M=",
                ],
                "redeemScript": "srKysrKysrKysrKysrKysrKysrKysrKysrKysrKy",
            })
        );
        let recovered = AddressWitness::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_p2pkh_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::Value;
        let original = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0xa1; 65]),
        };
        let value = original.to_object().expect("to_object");
        // `BinaryData` serializes as `Value::Bytes(Vec<u8>)` in non-HR mode.
        assert_eq!(
            value,
            platform_value!({
                "$type": "p2pkh",
                "signature": Value::Bytes(vec![0xa1; 65]),
            })
        );
        let recovered = AddressWitness::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_p2sh_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::Value;
        let original = AddressWitness::P2sh {
            redeem_script: BinaryData::new(vec![0xb2; 30]),
            signatures: vec![BinaryData::new(vec![0xc3; 65])],
        };
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$type": "p2sh",
                "signatures": [Value::Bytes(vec![0xc3; 65])],
                "redeemScript": Value::Bytes(vec![0xb2; 30]),
            })
        );
        let recovered = AddressWitness::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
