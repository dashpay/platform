use std::convert::{TryFrom, TryInto};

use dashcore::{OutPoint, Transaction};

use serde::{Deserialize, Deserializer, Serialize};

use bincode::{Decode, Encode};

pub use instant::*;
use platform_value::Value;
#[cfg(feature = "validation")]
use platform_version::version::PlatformVersion;
use serde::de::Error;

use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::prelude::Identifier;
#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
use crate::{ProtocolError, SerdeParsingError};

pub mod chain;
pub mod instant;
pub mod validate_asset_lock_transaction_structure;

// TODO: Serialization with bincode
// TODO: Consider use Box for InstantAssetLockProof
//
// Wire-shape note: this is an *internally-tagged* enum (`#[serde(tag = "type")]`
// with no `content`). serde's internal tagging works on newtype variants whose
// inner is a struct — both `InstantAssetLockProof` and `ChainAssetLockProof`
// qualify — so the inner struct's fields are flattened next to the `type`
// discriminator: `{"type": "instant", "instantLock": ..., "transaction": ...,
// "outputIndex": ...}`. This matches the convention applied to other tagged
// unions exposed to JS (see `AddressWitness`, `AddressFundsFeeStrategyStep`).
// Bincode `Encode`/`Decode` derives are independent of serde, so consensus
// binary format is unaffected.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Encode, Decode)]
#[serde(tag = "type", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum AssetLockProof {
    Instant(#[bincode(with_serde)] InstantAssetLockProof),
    Chain(#[bincode(with_serde)] ChainAssetLockProof),
}

/// Wire-shape Deserialize uses the same internal-tag layout the Serialize derive
/// produces, but routes the instant variant through `RawInstantLockProof` so the
/// dashcore `InstantLock` can be reconstructed from its raw bytes form.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum RawAssetLockProof {
    Instant(RawInstantLockProof),
    Chain(ChainAssetLockProof),
}

impl TryFrom<RawAssetLockProof> for AssetLockProof {
    type Error = ProtocolError;

    fn try_from(value: RawAssetLockProof) -> Result<Self, Self::Error> {
        match value {
            RawAssetLockProof::Instant(raw_instant_lock) => {
                let instant_lock = raw_instant_lock.try_into()?;

                Ok(AssetLockProof::Instant(instant_lock))
            }
            RawAssetLockProof::Chain(chain) => Ok(AssetLockProof::Chain(chain)),
        }
    }
}

impl<'de> Deserialize<'de> for AssetLockProof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawAssetLockProof::deserialize(deserializer)?;
        raw.try_into().map_err(|e: ProtocolError| {
            D::Error::custom(format!(
                "expected to be able to deserialize asset lock proof: {}",
                e
            ))
        })
    }
}

impl Default for AssetLockProof {
    fn default() -> Self {
        Self::Instant(InstantAssetLockProof::default())
    }
}

impl AsRef<AssetLockProof> for AssetLockProof {
    fn as_ref(&self) -> &AssetLockProof {
        self
    }
}
pub enum AssetLockProofType {
    Instant = 0,
    Chain = 1,
}

impl TryFrom<u8> for AssetLockProofType {
    type Error = SerdeParsingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Instant),
            1 => Ok(Self::Chain),
            _ => Err(SerdeParsingError::new("Unexpected asset lock proof type")),
        }
    }
}

impl TryFrom<u64> for AssetLockProofType {
    type Error = SerdeParsingError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Instant),
            1 => Ok(Self::Chain),
            _ => Err(SerdeParsingError::new("Unexpected asset lock proof type")),
        }
    }
}

// TODO: Versioning
impl AssetLockProof {
    pub fn type_from_raw_value(value: &Value) -> Option<AssetLockProofType> {
        let proof_type_res = value.get_integer::<u8>("type");

        match proof_type_res {
            Ok(proof_type_int) => {
                let proof_type = AssetLockProofType::try_from(proof_type_int);
                proof_type.ok()
            }
            Err(_) => None,
        }
    }

    pub fn create_identifier(&self) -> Result<Identifier, ProtocolError> {
        match self {
            AssetLockProof::Instant(instant_proof) => instant_proof.create_identifier(),
            AssetLockProof::Chain(chain_proof) => Ok(chain_proof.create_identifier()),
        }
    }

    pub fn output_index(&self) -> u32 {
        match self {
            AssetLockProof::Instant(proof) => proof.output_index(),
            AssetLockProof::Chain(proof) => proof.out_point.vout,
        }
    }

    pub fn out_point(&self) -> Option<OutPoint> {
        match self {
            AssetLockProof::Instant(proof) => proof.out_point(),
            AssetLockProof::Chain(proof) => Some(proof.out_point),
        }
    }

    pub fn transaction(&self) -> Option<&Transaction> {
        match self {
            AssetLockProof::Instant(is_lock) => Some(is_lock.transaction()),
            AssetLockProof::Chain(_chain_lock) => None,
        }
    }

    pub fn to_raw_object(&self) -> Result<Value, ProtocolError> {
        match self {
            AssetLockProof::Instant(is) => {
                platform_value::to_value(is).map_err(ProtocolError::ValueError)
            }
            AssetLockProof::Chain(cl) => {
                platform_value::to_value(cl).map_err(ProtocolError::ValueError)
            }
        }
    }

    /// Validate the structure of the asset lock proof
    #[cfg(feature = "validation")]
    pub fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match self {
            AssetLockProof::Instant(proof) => proof.validate_structure(platform_version),
            AssetLockProof::Chain(_) => Ok(SimpleConsensusValidationResult::default()),
        }
    }
}

impl TryFrom<&Value> for AssetLockProof {
    type Error = ProtocolError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        //this is a complete hack for the moment
        //todo: replace with
        //  from_value(value.clone()).map_err(ProtocolError::ValueError)
        let proof_type_int: Option<u8> = value
            .get_optional_integer("type")
            .map_err(ProtocolError::ValueError)?;
        if let Some(proof_type_int) = proof_type_int {
            let proof_type = AssetLockProofType::try_from(proof_type_int)?;

            match proof_type {
                AssetLockProofType::Instant => Ok(Self::Instant(value.clone().try_into()?)),
                AssetLockProofType::Chain => Ok(Self::Chain(value.clone().try_into()?)),
            }
        } else {
            let map = value.as_map().ok_or(ProtocolError::DecodingError(
                "error decoding asset lock proof".to_string(),
            ))?;
            let (key, asset_lock_value) = map.first().ok_or(ProtocolError::DecodingError(
                "error decoding asset lock proof as it was empty".to_string(),
            ))?;
            match key.as_str().ok_or(ProtocolError::DecodingError(
                "error decoding asset lock proof".to_string(),
            ))? {
                "Instant" => Ok(Self::Instant(asset_lock_value.clone().try_into()?)),
                "Chain" => Ok(Self::Chain(asset_lock_value.clone().try_into()?)),
                _ => Err(ProtocolError::DecodingError(
                    "error decoding asset lock proof".to_string(),
                )),
            }
        }
    }
}

impl TryFrom<Value> for AssetLockProof {
    type Error = ProtocolError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let proof_type_int: Option<u8> = value
            .get_optional_integer("type")
            .map_err(ProtocolError::ValueError)?;
        if let Some(proof_type_int) = proof_type_int {
            let proof_type = AssetLockProofType::try_from(proof_type_int)?;

            match proof_type {
                AssetLockProofType::Instant => Ok(Self::Instant(value.try_into()?)),
                AssetLockProofType::Chain => Ok(Self::Chain(value.try_into()?)),
            }
        } else {
            let map = value.as_map().ok_or(ProtocolError::DecodingError(
                "error decoding asset lock proof".to_string(),
            ))?;
            let (key, asset_lock_value) = map.first().ok_or(ProtocolError::DecodingError(
                "error decoding asset lock proof as it was empty".to_string(),
            ))?;
            match key.as_str().ok_or(ProtocolError::DecodingError(
                "error decoding asset lock proof".to_string(),
            ))? {
                "Instant" => Ok(Self::Instant(asset_lock_value.clone().try_into()?)),
                "Chain" => Ok(Self::Chain(asset_lock_value.clone().try_into()?)),
                _ => Err(ProtocolError::DecodingError(
                    "error decoding asset lock proof".to_string(),
                )),
            }
        }
    }
}

impl TryInto<Value> for AssetLockProof {
    type Error = ProtocolError;

    fn try_into(self) -> Result<Value, Self::Error> {
        match self {
            AssetLockProof::Instant(instant_proof) => {
                platform_value::to_value(instant_proof).map_err(ProtocolError::ValueError)
            }
            AssetLockProof::Chain(chain_proof) => {
                platform_value::to_value(chain_proof).map_err(ProtocolError::ValueError)
            }
        }
    }
}

impl TryInto<Value> for &AssetLockProof {
    type Error = ProtocolError;

    fn try_into(self) -> Result<Value, Self::Error> {
        match self {
            AssetLockProof::Instant(instant_proof) => {
                platform_value::to_value(instant_proof).map_err(ProtocolError::ValueError)
            }
            AssetLockProof::Chain(chain_proof) => {
                platform_value::to_value(chain_proof).map_err(ProtocolError::ValueError)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::{OutPoint, Txid};
    use std::str::FromStr;

    /// JSON wire shape is internally tagged: `{type, ...flattened inner fields}`,
    /// no `data` wrapper. This guards against accidental reintroduction of the
    /// old adjacent-tagged `{type, data: {...}}` shape and against the divergence
    /// from the `AddressWitness` / `AddressFundsFeeStrategyStep` precedent.
    #[test]
    fn chain_variant_serializes_with_internal_tag() {
        let txid =
            Txid::from_str("e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d")
                .unwrap();
        let proof = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 11,
            out_point: OutPoint { txid, vout: 1 },
        });

        let json = serde_json::to_value(&proof).expect("serialize");

        assert_eq!(json["type"], "chain");
        assert_eq!(json["coreChainLockedHeight"], 11);
        assert!(
            json.get("data").is_none(),
            "should not have a `data` wrapper, got: {}",
            json
        );

        // Round-trip
        let restored: AssetLockProof = serde_json::from_value(json).expect("deserialize");
        assert_eq!(proof, restored);
    }
}
