use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use dashcore::PublicKey;
use platform_value::BinaryData;
use serde::{Deserialize, Serialize};

/// The input witness data required to spend from a PlatformAddress.
///
/// This enum captures the different spending patterns for P2PKH and P2SH addresses.
#[derive(Debug, Clone, PartialEq)]
pub enum AddressWitness {
    /// P2PKH witness: signature + public key
    ///
    /// Used for spending from a Pay-to-Public-Key-Hash address.
    P2pkh {
        /// The ECDSA signature authorizing the spend
        signature: BinaryData,
        /// The public key whose hash matches the address
        public_key: PublicKey,
    },
    /// P2SH witness: signatures + redeem script
    ///
    /// Used for spending from a Pay-to-Script-Hash address (e.g., multisig).
    /// For a 2-of-3 multisig, signatures would be `[OP_0, sig1, sig2]` and
    /// redeem_script would be `OP_2 <pub1> <pub2> <pub3> OP_3 OP_CHECKMULTISIG`.
    P2sh {
        /// The signatures (may include placeholder bytes like OP_0 for CHECKMULTISIG bug)
        signatures: Vec<BinaryData>,
        /// The redeem script that hashes to the address
        redeem_script: BinaryData,
    },
}

impl Encode for AddressWitness {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            AddressWitness::P2pkh {
                signature,
                public_key,
            } => {
                0u8.encode(encoder)?;
                signature.encode(encoder)?;
                let pubkey_bytes = public_key.to_bytes();
                pubkey_bytes.encode(encoder)?;
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

impl Decode for AddressWitness {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let discriminant = u8::decode(decoder)?;
        match discriminant {
            0 => {
                let signature = BinaryData::decode(decoder)?;
                let pubkey_bytes: Vec<u8> = Vec::decode(decoder)?;
                let public_key = PublicKey::from_slice(&pubkey_bytes).map_err(|e| {
                    DecodeError::OtherString(format!("Invalid ECDSA public key: {}", e))
                })?;
                Ok(AddressWitness::P2pkh {
                    signature,
                    public_key,
                })
            }
            1 => {
                let signatures = Vec::<BinaryData>::decode(decoder)?;
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

impl<'de> bincode::BorrowDecode<'de> for AddressWitness {
    fn borrow_decode<D: BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let discriminant = u8::borrow_decode(decoder)?;
        match discriminant {
            0 => {
                let signature = BinaryData::borrow_decode(decoder)?;
                let pubkey_bytes: Vec<u8> = Vec::borrow_decode(decoder)?;
                let public_key = PublicKey::from_slice(&pubkey_bytes).map_err(|e| {
                    DecodeError::OtherString(format!("Invalid ECDSA public key: {}", e))
                })?;
                Ok(AddressWitness::P2pkh {
                    signature,
                    public_key,
                })
            }
            1 => {
                let signatures = Vec::<BinaryData>::borrow_decode(decoder)?;
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

#[cfg(feature = "state-transition-serde-conversion")]
impl Serialize for AddressWitness {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        match self {
            AddressWitness::P2pkh {
                signature,
                public_key,
            } => {
                let mut state = serializer.serialize_struct("AddressWitness", 3)?;
                state.serialize_field("type", "p2pkh")?;
                state.serialize_field("signature", signature)?;
                state.serialize_field("publicKey", &public_key.to_bytes())?;
                state.end()
            }
            AddressWitness::P2sh {
                signatures,
                redeem_script,
            } => {
                let mut state = serializer.serialize_struct("AddressWitness", 3)?;
                state.serialize_field("type", "p2sh")?;
                state.serialize_field("signatures", signatures)?;
                state.serialize_field("redeemScript", redeem_script)?;
                state.end()
            }
        }
    }
}

#[cfg(feature = "state-transition-serde-conversion")]
impl<'de> Deserialize<'de> for AddressWitness {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct AddressWitnessVisitor;

        impl<'de> Visitor<'de> for AddressWitnessVisitor {
            type Value = AddressWitness;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an AddressWitness struct")
            }

            fn visit_map<V>(self, mut map: V) -> Result<AddressWitness, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut witness_type: Option<String> = None;
                let mut signature: Option<BinaryData> = None;
                let mut public_key: Option<Vec<u8>> = None;
                let mut signatures: Option<Vec<BinaryData>> = None;
                let mut redeem_script: Option<BinaryData> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => {
                            witness_type = Some(map.next_value()?);
                        }
                        "signature" => {
                            signature = Some(map.next_value()?);
                        }
                        "publicKey" => {
                            public_key = Some(map.next_value()?);
                        }
                        "signatures" => {
                            signatures = Some(map.next_value()?);
                        }
                        "redeemScript" => {
                            redeem_script = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let witness_type = witness_type.ok_or_else(|| de::Error::missing_field("type"))?;

                match witness_type.as_str() {
                    "p2pkh" => {
                        let signature =
                            signature.ok_or_else(|| de::Error::missing_field("signature"))?;
                        let pubkey_bytes =
                            public_key.ok_or_else(|| de::Error::missing_field("publicKey"))?;
                        let public_key = PublicKey::from_slice(&pubkey_bytes).map_err(|e| {
                            de::Error::custom(format!("Invalid ECDSA public key: {}", e))
                        })?;
                        Ok(AddressWitness::P2pkh {
                            signature,
                            public_key,
                        })
                    }
                    "p2sh" => {
                        let signatures =
                            signatures.ok_or_else(|| de::Error::missing_field("signatures"))?;
                        let redeem_script =
                            redeem_script.ok_or_else(|| de::Error::missing_field("redeemScript"))?;
                        Ok(AddressWitness::P2sh {
                            signatures,
                            redeem_script,
                        })
                    }
                    _ => Err(de::Error::unknown_variant(
                        &witness_type,
                        &["p2pkh", "p2sh"],
                    )),
                }
            }
        }

        deserializer.deserialize_struct(
            "AddressWitness",
            &["type", "signature", "publicKey", "signatures", "redeemScript"],
            AddressWitnessVisitor,
        )
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
            AddressWitness::P2pkh {
                signature,
                public_key,
            } => {
                data.push(0u8);
                data.extend_from_slice(&public_key.to_bytes());
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

    /// Returns the public key if this is a P2PKH witness
    pub fn public_key(&self) -> Option<&PublicKey> {
        match self {
            AddressWitness::P2pkh { public_key, .. } => Some(public_key),
            AddressWitness::P2sh { .. } => None,
        }
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
mod tests {
    use super::*;
    use bincode::config;

    #[test]
    fn test_p2pkh_witness_encode_decode() {
        // Create a valid ECDSA public key (33 bytes compressed)
        let mut pubkey_bytes = vec![0x02]; // compressed prefix
        pubkey_bytes.extend_from_slice(&[0x12; 32]); // x coordinate
        let public_key = PublicKey::from_slice(&pubkey_bytes).unwrap();

        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]),
            public_key,
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
                BinaryData::new(vec![0x00]), // OP_0 placeholder
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
        let mut pubkey_bytes = vec![0x02];
        pubkey_bytes.extend_from_slice(&[0x12; 32]);
        let public_key = PublicKey::from_slice(&pubkey_bytes).unwrap();

        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x44]),
            public_key,
        };

        let id = witness.unique_id();
        assert!(!id.is_empty());

        // Different public key should produce different ID
        let mut pubkey_bytes2 = vec![0x03];
        pubkey_bytes2.extend_from_slice(&[0x12; 32]);
        let public_key2 = PublicKey::from_slice(&pubkey_bytes2).unwrap();

        let witness2 = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x44]),
            public_key: public_key2,
        };
        assert_ne!(id, witness2.unique_id());
    }

    #[test]
    fn test_unique_id_p2sh() {
        let witness = AddressWitness::P2sh {
            signatures: vec![BinaryData::new(vec![0x00]), BinaryData::new(vec![0x30, 0x44])],
            redeem_script: BinaryData::new(vec![0x52, 0xae]),
        };

        let id = witness.unique_id();
        assert!(!id.is_empty());

        // Different redeem script should produce different ID
        let witness2 = AddressWitness::P2sh {
            signatures: vec![BinaryData::new(vec![0x00]), BinaryData::new(vec![0x30, 0x44])],
            redeem_script: BinaryData::new(vec![0x53, 0xae]),
        };
        assert_ne!(id, witness2.unique_id());
    }

    #[cfg(feature = "state-transition-serde-conversion")]
    #[test]
    fn test_p2pkh_serde() {
        let mut pubkey_bytes = vec![0x02];
        pubkey_bytes.extend_from_slice(&[0x12; 32]);
        let public_key = PublicKey::from_slice(&pubkey_bytes).unwrap();

        let witness = AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]),
            public_key,
        };

        let json = serde_json::to_string(&witness).unwrap();
        let deserialized: AddressWitness = serde_json::from_str(&json).unwrap();

        assert_eq!(witness, deserialized);
    }

    #[cfg(feature = "state-transition-serde-conversion")]
    #[test]
    fn test_p2sh_serde() {
        let witness = AddressWitness::P2sh {
            signatures: vec![BinaryData::new(vec![0x00]), BinaryData::new(vec![0x30, 0x44])],
            redeem_script: BinaryData::new(vec![0x52, 0xae]),
        };

        let json = serde_json::to_string(&witness).unwrap();
        let deserialized: AddressWitness = serde_json::from_str(&json).unwrap();

        assert_eq!(witness, deserialized);
    }
}
