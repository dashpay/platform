use crate::identifier::Identifier;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Information about the status of an Evonode
#[derive(Clone, Debug, PartialEq, Encode, Decode, Serialize, Deserialize)]
pub struct EvonodeStatusV0 {
    /// The Identifier of the Evonode
    pub pro_tx_hash: String,
    /// The latest block height stored on the Evonode
    pub latest_block_height: u64,
}

/// Trait defining getters for `EvonodeStatusV0`.
pub trait EvonodeStatusV0Getters {
    /// Returns the Evonode proTxHash
    fn pro_tx_hash(&self) -> String;

    /// Returns the Evonode's latest stored block height
    fn latest_block_height(&self) -> u64;
}
