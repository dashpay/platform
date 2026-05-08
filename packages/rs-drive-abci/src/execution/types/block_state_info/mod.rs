/// Version 0
pub mod v0;

use crate::error::Error;
use crate::execution::types::block_state_info::v0::{
    BlockStateInfoV0Getters, BlockStateInfoV0Methods, BlockStateInfoV0Setters,
};

use derive_more::From;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;

/// The versioned block state info
#[derive(Debug, From, Clone, Eq, PartialEq)]
pub enum BlockStateInfo {
    /// Version 0
    V0(v0::BlockStateInfoV0),
}

impl BlockStateInfoV0Getters for BlockStateInfo {
    fn height(&self) -> u64 {
        match self {
            BlockStateInfo::V0(v0) => v0.height(),
        }
    }

    fn round(&self) -> u32 {
        match self {
            BlockStateInfo::V0(v0) => v0.round(),
        }
    }

    fn block_time_ms(&self) -> u64 {
        match self {
            BlockStateInfo::V0(v0) => v0.block_time_ms(),
        }
    }

    fn previous_block_time_ms(&self) -> Option<u64> {
        match self {
            BlockStateInfo::V0(v0) => v0.previous_block_time_ms(),
        }
    }

    fn proposer_pro_tx_hash(&self) -> [u8; 32] {
        match self {
            BlockStateInfo::V0(v0) => v0.proposer_pro_tx_hash(),
        }
    }

    fn core_chain_locked_height(&self) -> u32 {
        match self {
            BlockStateInfo::V0(v0) => v0.core_chain_locked_height(),
        }
    }

    fn block_hash(&self) -> Option<[u8; 32]> {
        match self {
            BlockStateInfo::V0(v0) => v0.block_hash(),
        }
    }

    fn app_hash(&self) -> Option<[u8; 32]> {
        match self {
            BlockStateInfo::V0(v0) => v0.app_hash(),
        }
    }
}

impl BlockStateInfoV0Setters for BlockStateInfo {
    fn set_height(&mut self, height: u64) {
        match self {
            BlockStateInfo::V0(v0) => {
                v0.set_height(height);
            }
        }
    }

    fn set_round(&mut self, round: u32) {
        match self {
            BlockStateInfo::V0(v0) => {
                v0.set_round(round);
            }
        }
    }

    fn set_block_time_ms(&mut self, block_time_ms: u64) {
        match self {
            BlockStateInfo::V0(v0) => {
                v0.set_block_time_ms(block_time_ms);
            }
        }
    }

    fn set_previous_block_time_ms(&mut self, previous_block_time_ms: Option<u64>) {
        match self {
            BlockStateInfo::V0(v0) => {
                v0.set_previous_block_time_ms(previous_block_time_ms);
            }
        }
    }

    fn set_proposer_pro_tx_hash(&mut self, proposer_pro_tx_hash: [u8; 32]) {
        match self {
            BlockStateInfo::V0(v0) => {
                v0.set_proposer_pro_tx_hash(proposer_pro_tx_hash);
            }
        }
    }

    fn set_core_chain_locked_height(&mut self, core_chain_locked_height: u32) {
        match self {
            BlockStateInfo::V0(v0) => {
                v0.set_core_chain_locked_height(core_chain_locked_height);
            }
        }
    }

    fn set_block_hash(&mut self, block_hash: Option<[u8; 32]>) {
        match self {
            BlockStateInfo::V0(v0) => {
                v0.set_block_hash(block_hash);
            }
        }
    }

    fn set_app_hash(&mut self, app_hash: Option<[u8; 32]>) {
        match self {
            BlockStateInfo::V0(v0) => {
                v0.set_app_hash(app_hash);
            }
        }
    }
}

impl BlockStateInfoV0Methods for BlockStateInfo {
    fn to_block_info(&self, epoch: Epoch) -> BlockInfo {
        match self {
            BlockStateInfo::V0(v0) => v0.to_block_info(epoch),
        }
    }

    fn next_block_to(
        &self,
        previous_height: u64,
        previous_core_block_height: u32,
    ) -> Result<bool, Error> {
        match self {
            BlockStateInfo::V0(v0) => v0.next_block_to(previous_height, previous_core_block_height),
        }
    }

    fn matches_current_block<I: TryInto<[u8; 32]>>(
        &self,
        height: u64,
        round: u32,
        block_hash: I,
    ) -> Result<bool, Error> {
        match self {
            BlockStateInfo::V0(v0) => v0.matches_current_block(height, round, block_hash),
        }
    }

    fn matches_expected_block_info<I: TryInto<[u8; 32]>>(
        &self,
        height: u64,
        round: u32,
        core_block_height: u32,
        proposer_pro_tx_hash: [u8; 32],
        commit_hash: I,
        app_hash: I,
    ) -> Result<bool, Error> {
        match self {
            BlockStateInfo::V0(v0) => v0.matches_expected_block_info(
                height,
                round,
                core_block_height,
                proposer_pro_tx_hash,
                commit_hash,
                app_hash,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
    use dpp::block::epoch::Epoch;

    fn make_wrapper() -> BlockStateInfo {
        BlockStateInfo::V0(BlockStateInfoV0 {
            height: 10,
            round: 2,
            block_time_ms: 1_700_000_000_000,
            previous_block_time_ms: Some(1_699_999_990_000),
            proposer_pro_tx_hash: [1u8; 32],
            core_chain_locked_height: 500,
            block_hash: Some([0xAA; 32]),
            app_hash: Some([0xBB; 32]),
        })
    }

    #[test]
    fn wrapper_getters_delegate_correctly() {
        let info = make_wrapper();
        assert_eq!(info.height(), 10);
        assert_eq!(info.round(), 2);
        assert_eq!(info.block_time_ms(), 1_700_000_000_000);
        assert_eq!(info.previous_block_time_ms(), Some(1_699_999_990_000));
        assert_eq!(info.proposer_pro_tx_hash(), [1u8; 32]);
        assert_eq!(info.core_chain_locked_height(), 500);
        assert_eq!(info.block_hash(), Some([0xAA; 32]));
        assert_eq!(info.app_hash(), Some([0xBB; 32]));
    }

    #[test]
    fn wrapper_setters_delegate_correctly() {
        let mut info = make_wrapper();
        info.set_height(20);
        info.set_round(3);
        info.set_block_time_ms(2_000_000_000_000);
        info.set_previous_block_time_ms(None);
        info.set_proposer_pro_tx_hash([2u8; 32]);
        info.set_core_chain_locked_height(600);
        info.set_block_hash(None);
        info.set_app_hash(None);

        assert_eq!(info.height(), 20);
        assert_eq!(info.round(), 3);
        assert_eq!(info.block_time_ms(), 2_000_000_000_000);
        assert_eq!(info.previous_block_time_ms(), None);
        assert_eq!(info.proposer_pro_tx_hash(), [2u8; 32]);
        assert_eq!(info.core_chain_locked_height(), 600);
        assert_eq!(info.block_hash(), None);
        assert_eq!(info.app_hash(), None);
    }

    #[test]
    fn wrapper_to_block_info() {
        let info = make_wrapper();
        let epoch = Epoch::new(1).unwrap();
        let block_info = info.to_block_info(epoch);
        assert_eq!(block_info.height, 10);
        assert_eq!(block_info.core_height, 500);
    }

    #[test]
    fn wrapper_next_block_to() {
        let info = make_wrapper();
        assert!(info.next_block_to(9, 500).unwrap());
        assert!(!info.next_block_to(10, 500).unwrap());
    }

    #[test]
    fn wrapper_matches_current_block() {
        let info = make_wrapper();
        assert!(info.matches_current_block(10, 2, [0xAA; 32]).unwrap());
        assert!(!info.matches_current_block(11, 2, [0xAA; 32]).unwrap());
    }

    #[test]
    fn wrapper_matches_expected_block_info() {
        let info = make_wrapper();
        assert!(info
            .matches_expected_block_info(10, 2, 500, [1u8; 32], [0xAA; 32], [0xBB; 32])
            .unwrap());
        assert!(!info
            .matches_expected_block_info(10, 2, 501, [1u8; 32], [0xAA; 32], [0xBB; 32])
            .unwrap());
    }

    #[test]
    fn from_v0_conversion() {
        let v0 = BlockStateInfoV0 {
            height: 1,
            round: 0,
            block_time_ms: 0,
            previous_block_time_ms: None,
            proposer_pro_tx_hash: [0u8; 32],
            core_chain_locked_height: 0,
            block_hash: None,
            app_hash: None,
        };
        let info: BlockStateInfo = v0.into();
        assert_eq!(info.height(), 1);
    }

    #[test]
    fn equality_works() {
        let a = make_wrapper();
        let b = make_wrapper();
        assert_eq!(a, b);
    }
}
