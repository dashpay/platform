//! Blocks of the Platform chain and the state transitions in them (`getBlocks`,
//! `getOwnerTransactionsInBlock`).

use dpp::state_transition::StateTransition;

/// A state transition a block holds, with the result of its execution.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockTransaction {
    /// The state transition.
    pub state_transition: StateTransition,
    /// The SHA-256 of the serialized state transition, the hash Tenderdash knows it by.
    pub hash: [u8; 32],
    /// 0 if it executed, otherwise the code of the error it paid for.
    pub code: u32,
    /// The data of its execution result.
    pub result_data: Vec<u8>,
    /// The gas of its execution result.
    pub gas_used: i64,
}

/// The state transitions one owner has in a block, in block order.
///
/// When proved, every transition is in the block at `height`, with the result it is shown
/// with, and reports the requested owner as its own. The list is not proved complete: a
/// node can leave some of the owner's transitions out. To be sure of having them all, read
/// the whole block ([`Blocks`]).
#[derive(Debug, Clone, PartialEq)]
pub struct OwnerTransactionsInBlock {
    /// The height of the block.
    pub height: u64,
    /// The owner's state transitions.
    pub transactions: Vec<BlockTransaction>,
}

/// A block of the Platform chain with every state transition it holds, in block order.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The block's height.
    pub height: u64,
    /// The block's hash.
    pub hash: [u8; 32],
    /// The block time, in milliseconds since the Unix epoch.
    pub time_ms: u64,
    /// The Core height the block is chain locked at.
    pub core_chain_locked_height: u32,
    /// The root of Platform's state after this block's transitions.
    pub app_hash: [u8; 32],
    /// The masternode that proposed the block.
    pub proposer_pro_tx_hash: [u8; 32],
    /// The block's state transitions, each with its result.
    pub transactions: Vec<BlockTransaction>,
}

/// Consecutive blocks from a start height.
///
/// When proved, each block is one a validator quorum committed at its height, with exactly
/// these state transitions and results. There can be fewer blocks than asked for, and
/// nothing proves that a block left off the end does not exist: the node's chain may end
/// there, or its answer was full. Ask from the height after the last block to continue.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Blocks(pub Vec<Block>);
