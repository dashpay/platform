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

use crate::execution::types::block_state_info;
use crate::platform::{epoch, state};
use dashcore_rpc::dashcore::Txid;
use std::collections::BTreeMap;
use tenderdash_abci::proto::abci::ResponsePrepareProposal;

/// V0 of the Block execution context
pub struct BlockExecutionContext {
    /// Block info
    pub block_state_info: block_state_info::v0::BlockStateInfo,
    /// Epoch info
    pub epoch_info: epoch::v0::EpochInfo,
    /// Total hpmn count
    pub hpmn_count: u32,
    /// Current withdrawal transactions hash -> Transaction
    pub withdrawal_transactions: BTreeMap<Txid, Vec<u8>>,
    /// Block state
    pub block_platform_state: state::v0::PlatformState,
    /// The response prepare proposal if proposed by us
    pub proposer_results: Option<ResponsePrepareProposal>,
}
