use crate::abci::messages::BlockBeginRequest;
use crate::execution::fee_pools::epoch::EpochInfo;

pub struct BlockInfo {
    pub block_height: u64,
    pub block_time_ms: u64,
    pub previous_block_time_ms: Option<u64>,
    pub proposer_pro_tx_hash: [u8; 32],
}

impl BlockInfo {
    pub fn from_block_begin_request(block_begin_request: &BlockBeginRequest) -> BlockInfo {
        BlockInfo {
            block_height: block_begin_request.block_height,
            block_time_ms: block_begin_request.block_time_ms,
            previous_block_time_ms: block_begin_request.previous_block_time_ms,
            proposer_pro_tx_hash: block_begin_request.proposer_pro_tx_hash,
        }
    }
}

pub struct BlockExecutionContext {
    pub block_info: BlockInfo,
    pub epoch_info: EpochInfo,
}
