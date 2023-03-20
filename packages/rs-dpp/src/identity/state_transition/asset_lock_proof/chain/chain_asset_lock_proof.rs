use platform_value::Value;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::convert::TryFrom;

use crate::{
    errors::NonConsensusError, identifier::Identifier, util::hash::hash, util::vec::vec_to_array,
    ProtocolError,
};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainAssetLockProof {
    #[serde(rename = "type")]
    asset_lock_type: u8,
    pub core_chain_locked_height: u32,
    #[serde(with = "BigArray")]
    pub out_point: [u8; 36],
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
            out_point,
        }
    }

    /// Get proof type
    pub fn asset_lock_type() -> u8 {
        1
    }

    /// Create identifier
    pub fn create_identifier(&self) -> Result<Identifier, NonConsensusError> {
        let array = vec_to_array(hash(self.out_point).as_ref())?;
        Ok(Identifier::new(array))
    }
}
