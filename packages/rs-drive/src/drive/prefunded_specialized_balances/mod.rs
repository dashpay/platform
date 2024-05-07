mod add_prefunded_specialized_balance;
mod deduct_from_prefunded_specialized_balance;
mod add_prefunded_specialized_balance_operations;
mod deduct_from_prefunded_specialized_balance_operations;

use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::drive::batch::GroveDbOpBatch;
use crate::drive::{Drive, RootTree};

pub const PREFUNDED_BALANCES_FOR_VOTING: [u8; 1] = [128];

impl Drive {
    /// Add operations for creating initial prefunded specialized balances state structure
    /// In v1 we will only have the prefunded balances for voting
    /// In the future, we could use this for allowing for "free" state transitions as long as the
    /// state transition matches specific criteria.
    /// For example let's say you make a food delivery app, and you want to pay for when your
    /// customers make an order, the restaurant or food delivery app might prepay for all documents
    /// that make an order
    pub fn add_initial_prefunded_specialized_balances_operations(batch: &mut GroveDbOpBatch) {
        batch.add_insert_empty_sum_tree(
            vec![vec![RootTree::PreFundedSpecializedBalances as u8]],
            PREFUNDED_BALANCES_FOR_VOTING.to_vec(),
        );
    }
}
