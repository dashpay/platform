// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

use crate::abci::messages::BlockBeginRequest;
use crate::execution::fee_pools::epoch::EpochInfo;

/// Block info
pub struct BlockInfo {
    /// Block height
    pub block_height: u64,
    /// Block time in ms
    pub block_time_ms: u64,
    /// Previous block time in ms
    pub previous_block_time_ms: Option<u64>,
    /// Block proposer's proTxHash
    pub proposer_pro_tx_hash: [u8; 32],
}

impl BlockInfo {
    /// Given a `BlockBeginRequest` return `BlockInfo`
    pub fn from_block_begin_request(block_begin_request: &BlockBeginRequest) -> BlockInfo {
        BlockInfo {
            block_height: block_begin_request.block_height,
            block_time_ms: block_begin_request.block_time_ms,
            previous_block_time_ms: block_begin_request.previous_block_time_ms,
            proposer_pro_tx_hash: block_begin_request.proposer_pro_tx_hash,
        }
    }
}

/// Block execution context
pub struct BlockExecutionContext {
    /// Block info
    pub block_info: BlockInfo,
    /// Epoch info
    pub epoch_info: EpochInfo,
}
