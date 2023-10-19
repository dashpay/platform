use ::serde::{Deserialize, Serialize};
use platform_value::Value;
use std::convert::TryFrom;

use crate::{
    identifier::Identifier, util::hash::hash_to_vec, util::vec::vec_to_array, ProtocolError,
};
pub use bincode::{Decode, Encode};
use dashcore::consensus::Encodable;
use dashcore::OutPoint;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainAssetLockProof {
    // TODO: Remove type
    #[serde(rename = "type")]
    asset_lock_type: u8,
    pub core_chain_locked_height: u32,
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
            // TODO: change to const
            asset_lock_type: 1,
            core_chain_locked_height,
            out_point: OutPoint::from(out_point),
        }
    }

    /// Get proof type
    pub fn asset_lock_type() -> u8 {
        1
    }

    /// Create identifier
    pub fn create_identifier(&self) -> Result<Identifier, ProtocolError> {
        let mut outpoint_bytes = Vec::new();

        self.out_point
            .consensus_encode(&mut outpoint_bytes)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        let hash = hash_to_vec(&outpoint_bytes);

        let hash_array = vec_to_array(&hash)?;

        Ok(Identifier::new(hash_array))
    }
}
