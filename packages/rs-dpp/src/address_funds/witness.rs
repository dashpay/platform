use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use dashcore::ed25519_dalek::VerifyingKey;
use dashcore::PublicKey;
use platform_value::BinaryData;
use serde::{Deserialize, Serialize};

/// Represents the type of witness data supplied for a transaction input.
///
/// A witness consists of a signature plus either a public key or a script,
/// depending on the spending condition. This enum indicates which variant
/// is present and how the witness should be interpreted.
#[derive(Debug, Clone, PartialEq)]
pub enum WitnessType {
    /// No witness data is present for the input.
    NoWitness,

    /// The witness includes an ECDSA secp256k1 public key.
    ///
    /// This corresponds to traditional Bitcoin- and Dash-style ECDSA spends.
    ECDSAPublicKey(PublicKey),

    /// The witness includes an Ed25519 public key.
    ///
    /// Used for EDDSA-based verification (e.g., Dash Platform identity keys).
    EDDSAPublicKey(VerifyingKey),

    /// The witness includes a raw script rather than a public key.
    ///
    /// This allows script-based verification similar to P2WSH or custom script
    /// logic, where the script itself appears in the witness.
    Script(BinaryData),
}

impl Encode for WitnessType {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            WitnessType::NoWitness => {
                0u8.encode(encoder)?;
            }
            WitnessType::ECDSAPublicKey(pubkey) => {
                1u8.encode(encoder)?;
                let bytes = pubkey.to_bytes();
                bytes.encode(encoder)?;
            }
            WitnessType::EDDSAPublicKey(pubkey) => {
                2u8.encode(encoder)?;
                let bytes = pubkey.to_bytes();
                bytes.encode(encoder)?;
            }
            WitnessType::Script(script) => {
                3u8.encode(encoder)?;
                script.encode(encoder)?;
            }
        }
        Ok(())
    }
}

impl Decode for WitnessType {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let discriminant = u8::decode(decoder)?;
        match discriminant {
            0 => Ok(WitnessType::NoWitness),
            1 => {
                let bytes: Vec<u8> = Vec::decode(decoder)?;
                let pubkey = PublicKey::from_slice(&bytes).map_err(|e| {
                    DecodeError::OtherString(format!("Invalid ECDSA public key: {}", e))
                })?;
                Ok(WitnessType::ECDSAPublicKey(pubkey))
            }
            2 => {
                let bytes: [u8; 32] = <[u8; 32]>::decode(decoder)?;
                let pubkey = VerifyingKey::from_bytes(&bytes).map_err(|e| {
                    DecodeError::OtherString(format!("Invalid Ed25519 public key: {}", e))
                })?;
                Ok(WitnessType::EDDSAPublicKey(pubkey))
            }
            3 => {
                let script = BinaryData::decode(decoder)?;
                Ok(WitnessType::Script(script))
            }
            _ => Err(DecodeError::OtherString(format!(
                "Invalid WitnessType discriminant: {}",
                discriminant
            ))),
        }
    }
}

impl<'de> bincode::BorrowDecode<'de> for WitnessType {
    fn borrow_decode<D: BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let discriminant = u8::borrow_decode(decoder)?;
        match discriminant {
            0 => Ok(WitnessType::NoWitness),
            1 => {
                let bytes: Vec<u8> = Vec::borrow_decode(decoder)?;
                let pubkey = PublicKey::from_slice(&bytes).map_err(|e| {
                    DecodeError::OtherString(format!("Invalid ECDSA public key: {}", e))
                })?;
                Ok(WitnessType::ECDSAPublicKey(pubkey))
            }
            2 => {
                let bytes: [u8; 32] = <[u8; 32]>::borrow_decode(decoder)?;
                let pubkey = VerifyingKey::from_bytes(&bytes).map_err(|e| {
                    DecodeError::OtherString(format!("Invalid Ed25519 public key: {}", e))
                })?;
                Ok(WitnessType::EDDSAPublicKey(pubkey))
            }
            3 => {
                let script = BinaryData::borrow_decode(decoder)?;
                Ok(WitnessType::Script(script))
            }
            _ => Err(DecodeError::OtherString(format!(
                "Invalid WitnessType discriminant: {}",
                discriminant
            ))),
        }
    }
}

#[cfg(feature = "state-transition-serde-conversion")]
impl Serialize for WitnessType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("WitnessType", 2)?;
        match self {
            WitnessType::NoWitness => {
                state.serialize_field("type", "noWitness")?;
            }
            WitnessType::ECDSAPublicKey(pubkey) => {
                state.serialize_field("type", "ecdsaPublicKey")?;
                state.serialize_field("publicKey", &pubkey.to_bytes())?;
            }
            WitnessType::EDDSAPublicKey(pubkey) => {
                state.serialize_field("type", "eddsaPublicKey")?;
                state.serialize_field("publicKey", &pubkey.to_bytes())?;
            }
            WitnessType::Script(script) => {
                state.serialize_field("type", "script")?;
                state.serialize_field("script", script)?;
            }
        }
        state.end()
    }
}

#[cfg(feature = "state-transition-serde-conversion")]
impl<'de> Deserialize<'de> for WitnessType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct WitnessTypeVisitor;

        impl<'de> Visitor<'de> for WitnessTypeVisitor {
            type Value = WitnessType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a WitnessType struct")
            }

            fn visit_map<V>(self, mut map: V) -> Result<WitnessType, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut witness_type: Option<String> = None;
                let mut public_key: Option<Vec<u8>> = None;
                let mut script: Option<BinaryData> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => {
                            witness_type = Some(map.next_value()?);
                        }
                        "publicKey" => {
                            public_key = Some(map.next_value()?);
                        }
                        "script" => {
                            script = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let witness_type = witness_type.ok_or_else(|| de::Error::missing_field("type"))?;

                match witness_type.as_str() {
                    "noWitness" => Ok(WitnessType::NoWitness),
                    "ecdsaPublicKey" => {
                        let bytes =
                            public_key.ok_or_else(|| de::Error::missing_field("publicKey"))?;
                        let pubkey = PublicKey::from_slice(&bytes).map_err(|e| {
                            de::Error::custom(format!("Invalid ECDSA public key: {}", e))
                        })?;
                        Ok(WitnessType::ECDSAPublicKey(pubkey))
                    }
                    "eddsaPublicKey" => {
                        let bytes =
                            public_key.ok_or_else(|| de::Error::missing_field("publicKey"))?;
                        let bytes_array: [u8; 32] = bytes.try_into().map_err(|_| {
                            de::Error::custom("Ed25519 public key must be 32 bytes")
                        })?;
                        let pubkey = VerifyingKey::from_bytes(&bytes_array).map_err(|e| {
                            de::Error::custom(format!("Invalid Ed25519 public key: {}", e))
                        })?;
                        Ok(WitnessType::EDDSAPublicKey(pubkey))
                    }
                    "script" => {
                        let script_data =
                            script.ok_or_else(|| de::Error::missing_field("script"))?;
                        Ok(WitnessType::Script(script_data))
                    }
                    _ => Err(de::Error::unknown_variant(
                        &witness_type,
                        &["noWitness", "ecdsaPublicKey", "eddsaPublicKey", "script"],
                    )),
                }
            }
        }

        deserializer.deserialize_struct(
            "WitnessType",
            &["type", "publicKey", "script"],
            WitnessTypeVisitor,
        )
    }
}

/// A witness containing the cryptographic signature and associated data
/// required to validate a transaction input.
///
/// A witness always contains a signature. Depending on the `WitnessType`,
/// the witness may include an ECDSA public key, an Ed25519 public key, or
/// a raw script. The witness type indicates how the signature should be
/// interpreted by the verifier.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct AddressWitness {
    /// Describes how the witness should be interpreted (public key or script).
    pub witness_type: WitnessType,

    /// The signature authorizing the spend.
    ///
    /// The signature is interpreted according to the `WitnessType`. For
    /// example:
    /// - `ECDSAPublicKey` → ECDSA signature
    /// - `EDDSAPublicKey` → Ed25519 signature
    /// - `Script` → Signature is consumed by script execution
    pub signature: BinaryData,
}

impl Encode for AddressWitness {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.witness_type.encode(encoder)?;
        self.signature.encode(encoder)?;
        Ok(())
    }
}

impl Decode for AddressWitness {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let witness_type = WitnessType::decode(decoder)?;
        let signature = BinaryData::decode(decoder)?;
        Ok(AddressWitness {
            witness_type,
            signature,
        })
    }
}

impl<'de> bincode::BorrowDecode<'de> for AddressWitness {
    fn borrow_decode<D: BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let witness_type = WitnessType::borrow_decode(decoder)?;
        let signature = BinaryData::borrow_decode(decoder)?;
        Ok(AddressWitness {
            witness_type,
            signature,
        })
    }
}

impl AddressWitness {
    /// Generates a unique identifier for this witness based on its contents.
    ///
    /// This is used for deduplication purposes in unique_identifiers() implementations.
    pub fn unique_id(&self) -> String {
        use base64::prelude::BASE64_STANDARD;
        use base64::Engine;

        // Combine witness type discriminant and signature for unique ID
        let mut data = Vec::new();

        match &self.witness_type {
            WitnessType::NoWitness => {
                data.push(0u8);
            }
            WitnessType::ECDSAPublicKey(pubkey) => {
                data.push(1u8);
                data.extend_from_slice(&pubkey.to_bytes());
            }
            WitnessType::EDDSAPublicKey(pubkey) => {
                data.push(2u8);
                data.extend_from_slice(&pubkey.to_bytes());
            }
            WitnessType::Script(script) => {
                data.push(3u8);
                data.extend_from_slice(script.as_slice());
            }
        }

        data.extend_from_slice(self.signature.as_slice());
        BASE64_STANDARD.encode(&data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::config;

    #[test]
    fn test_witness_type_no_witness_encode_decode() {
        let witness_type = WitnessType::NoWitness;

        let encoded = bincode::encode_to_vec(&witness_type, config::standard()).unwrap();
        let decoded: WitnessType = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness_type, decoded);
    }

    #[test]
    fn test_witness_type_ecdsa_encode_decode() {
        // Create a valid ECDSA public key (33 bytes compressed)
        let mut pubkey_bytes = vec![0x02]; // compressed prefix
        pubkey_bytes.extend_from_slice(&[0x12; 32]); // x coordinate
        let pubkey = PublicKey::from_slice(&pubkey_bytes).unwrap();

        let witness_type = WitnessType::ECDSAPublicKey(pubkey);

        let encoded = bincode::encode_to_vec(&witness_type, config::standard()).unwrap();
        let decoded: WitnessType = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness_type, decoded);
    }

    #[test]
    fn test_witness_type_eddsa_encode_decode() {
        // Create a valid Ed25519 public key (32 bytes)
        let pubkey_bytes = [0x42; 32];
        let pubkey = VerifyingKey::from_bytes(&pubkey_bytes).unwrap();

        let witness_type = WitnessType::EDDSAPublicKey(pubkey);

        let encoded = bincode::encode_to_vec(&witness_type, config::standard()).unwrap();
        let decoded: WitnessType = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness_type, decoded);
    }

    #[test]
    fn test_witness_type_script_encode_decode() {
        let script = BinaryData::new(vec![0x76, 0xa9, 0x14]); // OP_DUP OP_HASH160 OP_PUSHDATA
        let witness_type = WitnessType::Script(script);

        let encoded = bincode::encode_to_vec(&witness_type, config::standard()).unwrap();
        let decoded: WitnessType = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness_type, decoded);
    }

    #[test]
    fn test_address_witness_encode_decode() {
        let mut pubkey_bytes = vec![0x02];
        pubkey_bytes.extend_from_slice(&[0x12; 32]);
        let pubkey = PublicKey::from_slice(&pubkey_bytes).unwrap();

        let witness = AddressWitness {
            witness_type: WitnessType::ECDSAPublicKey(pubkey),
            signature: BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]), // DER signature prefix
        };

        let encoded = bincode::encode_to_vec(&witness, config::standard()).unwrap();
        let decoded: AddressWitness = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        assert_eq!(witness, decoded);
    }

    #[test]
    fn test_address_witness_unique_id_no_witness() {
        let witness = AddressWitness {
            witness_type: WitnessType::NoWitness,
            signature: BinaryData::new(vec![0x01, 0x02, 0x03]),
        };

        let id = witness.unique_id();
        assert!(!id.is_empty());

        // Same witness should produce same ID
        let witness2 = AddressWitness {
            witness_type: WitnessType::NoWitness,
            signature: BinaryData::new(vec![0x01, 0x02, 0x03]),
        };
        assert_eq!(id, witness2.unique_id());

        // Different signature should produce different ID
        let witness3 = AddressWitness {
            witness_type: WitnessType::NoWitness,
            signature: BinaryData::new(vec![0x04, 0x05, 0x06]),
        };
        assert_ne!(id, witness3.unique_id());
    }

    #[test]
    fn test_address_witness_unique_id_ecdsa() {
        let mut pubkey_bytes = vec![0x02];
        pubkey_bytes.extend_from_slice(&[0x12; 32]);
        let pubkey = PublicKey::from_slice(&pubkey_bytes).unwrap();

        let witness = AddressWitness {
            witness_type: WitnessType::ECDSAPublicKey(pubkey),
            signature: BinaryData::new(vec![0x30, 0x44]),
        };

        let id = witness.unique_id();
        assert!(!id.is_empty());

        // Different public key should produce different ID
        let mut pubkey_bytes2 = vec![0x03];
        pubkey_bytes2.extend_from_slice(&[0x13; 32]);
        let pubkey2 = PublicKey::from_slice(&pubkey_bytes2).unwrap();

        let witness2 = AddressWitness {
            witness_type: WitnessType::ECDSAPublicKey(pubkey2),
            signature: BinaryData::new(vec![0x30, 0x44]),
        };
        assert_ne!(id, witness2.unique_id());
    }

    #[test]
    fn test_address_witness_unique_id_eddsa() {
        let pubkey = VerifyingKey::from_bytes(&[0x42; 32]).unwrap();

        let witness = AddressWitness {
            witness_type: WitnessType::EDDSAPublicKey(pubkey),
            signature: BinaryData::new(vec![0x01; 64]),
        };

        let id = witness.unique_id();
        assert!(!id.is_empty());

        // Different public key should produce different ID
        let pubkey2 = VerifyingKey::from_bytes(&[0x43; 32]).unwrap();

        let witness2 = AddressWitness {
            witness_type: WitnessType::EDDSAPublicKey(pubkey2),
            signature: BinaryData::new(vec![0x01; 64]),
        };
        assert_ne!(id, witness2.unique_id());
    }

    #[test]
    fn test_address_witness_unique_id_script() {
        let script = BinaryData::new(vec![0x76, 0xa9, 0x14]);

        let witness = AddressWitness {
            witness_type: WitnessType::Script(script),
            signature: BinaryData::new(vec![0x00]),
        };

        let id = witness.unique_id();
        assert!(!id.is_empty());

        // Different script should produce different ID
        let script2 = BinaryData::new(vec![0x76, 0xa9, 0x15]);

        let witness2 = AddressWitness {
            witness_type: WitnessType::Script(script2),
            signature: BinaryData::new(vec![0x00]),
        };
        assert_ne!(id, witness2.unique_id());
    }

    #[cfg(feature = "state-transition-serde-conversion")]
    #[test]
    fn test_witness_type_serde_no_witness() {
        let witness_type = WitnessType::NoWitness;

        let json = serde_json::to_string(&witness_type).unwrap();
        let deserialized: WitnessType = serde_json::from_str(&json).unwrap();

        assert_eq!(witness_type, deserialized);
    }

    #[cfg(feature = "state-transition-serde-conversion")]
    #[test]
    fn test_witness_type_serde_ecdsa() {
        let mut pubkey_bytes = vec![0x02];
        pubkey_bytes.extend_from_slice(&[0x12; 32]);
        let pubkey = PublicKey::from_slice(&pubkey_bytes).unwrap();

        let witness_type = WitnessType::ECDSAPublicKey(pubkey);

        let json = serde_json::to_string(&witness_type).unwrap();
        let deserialized: WitnessType = serde_json::from_str(&json).unwrap();

        assert_eq!(witness_type, deserialized);
    }

    #[cfg(feature = "state-transition-serde-conversion")]
    #[test]
    fn test_witness_type_serde_eddsa() {
        let pubkey = VerifyingKey::from_bytes(&[0x42; 32]).unwrap();

        let witness_type = WitnessType::EDDSAPublicKey(pubkey);

        let json = serde_json::to_string(&witness_type).unwrap();
        let deserialized: WitnessType = serde_json::from_str(&json).unwrap();

        assert_eq!(witness_type, deserialized);
    }

    #[cfg(feature = "state-transition-serde-conversion")]
    #[test]
    fn test_witness_type_serde_script() {
        let script = BinaryData::new(vec![0x76, 0xa9, 0x14]);
        let witness_type = WitnessType::Script(script);

        let json = serde_json::to_string(&witness_type).unwrap();
        let deserialized: WitnessType = serde_json::from_str(&json).unwrap();

        assert_eq!(witness_type, deserialized);
    }

    #[cfg(feature = "state-transition-serde-conversion")]
    #[test]
    fn test_address_witness_serde() {
        let mut pubkey_bytes = vec![0x02];
        pubkey_bytes.extend_from_slice(&[0x12; 32]);
        let pubkey = PublicKey::from_slice(&pubkey_bytes).unwrap();

        let witness = AddressWitness {
            witness_type: WitnessType::ECDSAPublicKey(pubkey),
            signature: BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]),
        };

        let json = serde_json::to_string(&witness).unwrap();
        let deserialized: AddressWitness = serde_json::from_str(&json).unwrap();

        assert_eq!(witness, deserialized);
    }
}
