use ::serde::{Deserialize, Serialize};
use platform_value::Value;
use std::convert::TryFrom;

use crate::util::hash::hash_double;
use crate::{identifier::Identifier, ProtocolError};
pub use bincode::{Decode, Encode};
use dashcore::OutPoint;

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
