use ::serde::{Deserialize, Serialize};
use platform_value::Value;
use std::convert::TryFrom;

use crate::util::hash::hash_double;
use crate::{identifier::Identifier, ProtocolError};
use dashcore::OutPoint;

/// Serde module for OutPoint:
/// - Human-readable (JSON): serializes as "txid:vout" string
/// - Non-human-readable (binary/platform_value): serializes as 36 bytes
mod outpoint_serde {
    use dashcore::OutPoint;
    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serializer};
    use std::str::FromStr;

    pub fn serialize<S>(out_point: &OutPoint, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            // JSON: serialize as "txid:vout" string
            serializer.serialize_str(&out_point.to_string())
        } else {
            // Binary: serialize as 36 bytes
            let bytes: [u8; 36] = (*out_point).into();
            serializer.serialize_bytes(&bytes)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OutPoint, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            // JSON: deserialize from "txid:vout" string
            let s = String::deserialize(deserializer)?;
            OutPoint::from_str(&s)
                .map_err(|e| D::Error::custom(format!("invalid outpoint: {}", e)))
        } else {
            // Binary: deserialize from 36 bytes
            struct OutPointVisitor;

            impl<'de> Visitor<'de> for OutPointVisitor {
                type Value = OutPoint;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("36 bytes for OutPoint")
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    if v.len() != 36 {
                        return Err(E::custom(format!(
                            "invalid outpoint length: expected 36, got {}",
                            v.len()
                        )));
                    }
                    let mut arr = [0u8; 36];
                    arr.copy_from_slice(v);
                    Ok(OutPoint::from(arr))
                }

                fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    self.visit_bytes(&v)
                }
            }

            deserializer.deserialize_bytes(OutPointVisitor)
        }
    }
}

/// Instant Asset Lock Proof is a part of Identity Create and Identity Topup
/// transitions. It is a proof that specific output of dash is locked in credits
/// pull and the transitions can mint credits and populate identity's balance.
/// To prove that the output is locked, a height where transaction was chain locked is provided.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainAssetLockProof {
    /// Core height on which the asset lock transaction was chain locked or higher
    pub core_chain_locked_height: u32,
    /// A reference to Asset Lock Special Transaction ID and output index in the payload
    #[serde(with = "outpoint_serde")]
    pub out_point: OutPoint,
}

impl TryFrom<Value> for ChainAssetLockProof {
    type Error = platform_value::Error;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value)
    }
}

impl ChainAssetLockProof {
    pub fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }
    pub fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        self.to_object()
    }

    pub fn new(core_chain_locked_height: u32, out_point: [u8; 36]) -> Self {
        Self {
            core_chain_locked_height,
            out_point: OutPoint::from(out_point),
        }
    }

    /// Create identifier
    pub fn create_identifier(&self) -> Identifier {
        let outpoint_bytes: [u8; 36] = self.out_point.into();

        let hash = hash_double(outpoint_bytes.as_slice());

        Identifier::new(hash)
    }
}
