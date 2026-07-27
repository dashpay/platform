use crate::block::block_info::BlockInfo;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Extended Block information
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedBlockInfoV0 {
    /// Basic block info
    pub basic_info: BlockInfo,
    /// App hash
    pub app_hash: [u8; 32],
    /// Quorum Hash
    pub quorum_hash: [u8; 32],
    /// The block id hash
    pub block_id_hash: [u8; 32],
    /// The proposer pro_tx_hash
    pub proposer_pro_tx_hash: [u8; 32],
    /// Signature
    // serde has no built-in `Serialize`/`Deserialize` for `[u8; N]` when N > 32,
    // so this 96-byte field needs an explicit byte serializer. `json_safe_fields`
    // injects one for byte fields, but only under `json-conversion`; this struct
    // derives serde unconditionally (no feature gate), so the annotation is written
    // out here to keep it compiling whenever `json-conversion` is off.
    #[serde(with = "crate::serialization::serde_bytes")]
    pub signature: [u8; 96],
    /// Round
    pub round: u32,
}

/// Trait for getting values from `ExtendedBlockInfoV0`
pub trait ExtendedBlockInfoV0Getters {
    /// Returns a reference to the basic block info.
    fn basic_info(&self) -> &BlockInfo;

    /// Returns a mutable reference to the basic block info.
    fn basic_info_mut(&mut self) -> &mut BlockInfo;

    /// Returns an owned copy of the basic block info.
    fn basic_info_owned(self) -> BlockInfo;

    /// Returns the app hash.
    fn app_hash(&self) -> &[u8; 32];

    /// Returns the quorum hash.
    fn quorum_hash(&self) -> &[u8; 32];
    /// Proposer pro tx hash.
    fn proposer_pro_tx_hash(&self) -> &[u8; 32];
    /// The block id hash
    fn block_id_hash(&self) -> &[u8; 32];

    /// Returns the signature.
    fn signature(&self) -> &[u8; 96];

    /// Returns the round.
    fn round(&self) -> u32;
}

/// Trait for setting values in `ExtendedBlockInfoV0`
pub trait ExtendedBlockInfoV0Setters {
    /// Sets the basic block info.
    fn set_basic_info(&mut self, info: BlockInfo);

    /// Sets the app hash.
    fn set_app_hash(&mut self, hash: [u8; 32]);

    /// Sets the quorum hash.
    fn set_quorum_hash(&mut self, hash: [u8; 32]);

    /// Sets the signature.
    fn set_signature(&mut self, signature: [u8; 96]);

    /// Sets the round.
    fn set_round(&mut self, round: u32);
}

impl ExtendedBlockInfoV0Getters for ExtendedBlockInfoV0 {
    fn basic_info(&self) -> &BlockInfo {
        &self.basic_info
    }

    fn basic_info_mut(&mut self) -> &mut BlockInfo {
        &mut self.basic_info
    }

    fn basic_info_owned(self) -> BlockInfo {
        self.basic_info
    }

    fn app_hash(&self) -> &[u8; 32] {
        &self.app_hash
    }

    fn quorum_hash(&self) -> &[u8; 32] {
        &self.quorum_hash
    }

    fn proposer_pro_tx_hash(&self) -> &[u8; 32] {
        &self.proposer_pro_tx_hash
    }

    fn block_id_hash(&self) -> &[u8; 32] {
        &self.block_id_hash
    }

    fn signature(&self) -> &[u8; 96] {
        &self.signature
    }

    fn round(&self) -> u32 {
        self.round
    }
}

impl ExtendedBlockInfoV0Setters for ExtendedBlockInfoV0 {
    fn set_basic_info(&mut self, info: BlockInfo) {
        self.basic_info = info;
    }

    fn set_app_hash(&mut self, hash: [u8; 32]) {
        self.app_hash = hash;
    }

    fn set_quorum_hash(&mut self, hash: [u8; 32]) {
        self.quorum_hash = hash;
    }

    fn set_signature(&mut self, signature: [u8; 96]) {
        self.signature = signature;
    }

    fn set_round(&mut self, round: u32) {
        self.round = round;
    }
}
