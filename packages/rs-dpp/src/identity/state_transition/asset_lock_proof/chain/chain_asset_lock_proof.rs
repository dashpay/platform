#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use ::serde::{Deserialize, Serialize};
use platform_value::Value;
use std::convert::TryFrom;

use crate::util::hash::hash_double;
use crate::{identifier::Identifier, ProtocolError};
use dashcore::OutPoint;

/// Instant Asset Lock Proof is a part of Identity Create and Identity Topup
/// transitions. It is a proof that specific output of dash is locked in credits
/// pull and the transitions can mint credits and populate identity's balance.
/// To prove that the output is locked, a height where transaction was chain locked is provided.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainAssetLockProof {
    /// Core height on which the asset lock transaction was chain locked or higher
    pub core_chain_locked_height: u32,
    /// A reference to Asset Lock Special Transaction ID and output index in the payload
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

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;
    use dashcore::{OutPoint, Txid};
    use std::str::FromStr;

    #[test]
    fn chain_asset_lock_proof_json_round_trip() {
        let txid_hex = "e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d";
        let txid = Txid::from_str(txid_hex).unwrap();
        let proof = ChainAssetLockProof {
            core_chain_locked_height: 11,
            out_point: OutPoint { txid, vout: 1 },
        };

        let json = proof.to_json().expect("to_json should succeed");

        // OutPoint should be "txid:vout" string (human-readable serde_json)
        assert!(
            json["outPoint"].is_string(),
            "outPoint should be a string, got: {:?}",
            json["outPoint"]
        );
        assert!(
            json["outPoint"].as_str().unwrap().contains(":"),
            "outPoint should contain ':'"
        );
        assert_eq!(json["coreChainLockedHeight"].as_u64().unwrap(), 11);

        let restored = ChainAssetLockProof::from_json(json).expect("from_json should succeed");
        assert_eq!(proof, restored);
    }

    #[test]
    fn chain_asset_lock_proof_value_round_trip() {
        let txid_hex = "e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d";
        let txid = Txid::from_str(txid_hex).unwrap();
        let proof = ChainAssetLockProof {
            core_chain_locked_height: 11,
            out_point: OutPoint { txid, vout: 1 },
        };

        let obj = proof.to_object().expect("to_object should succeed");
        let restored = ChainAssetLockProof::from_object(obj).expect("from_object should succeed");
        assert_eq!(proof, restored);
    }
}
