use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::identifier::Identifier;
use crate::util::hash::hash;
use crate::util::vec::vec_to_array;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ChainAssetLockProof {
    #[serde(rename = "type")]
    asset_lock_type: u8,
    core_chain_locked_height: u32,
    #[serde(with = "BigArray")]
    out_point: [u8; 36],
}

impl ChainAssetLockProof {
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

    /// Get Asset Lock proof core height
    pub fn core_chain_locked_height(&self) -> u32 {
        self.core_chain_locked_height
    }

    /// Get out_point
    pub fn out_point(&self) -> &[u8; 36] {
        &self.out_point
    }

    /// Create identifier
    pub fn create_identifier(&self) -> Identifier {
        return Identifier::new(
            vec_to_array(hash(self.out_point()).as_ref())
                .expect("Expected hash function to give a 32 byte output"),
        );
    }
}
