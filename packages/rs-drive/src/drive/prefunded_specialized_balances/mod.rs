#[cfg(feature = "server")]
mod add_prefunded_specialized_balance;
#[cfg(feature = "server")]
mod add_prefunded_specialized_balance_operations;
#[cfg(feature = "server")]
mod deduct_from_prefunded_specialized_balance_operations;
#[cfg(feature = "server")]
mod empty_prefunded_specialized_balance;
#[cfg(feature = "server")]
mod empty_prefunded_specialized_balance_operations;
#[cfg(feature = "server")]
mod estimation_costs;
#[cfg(feature = "server")]
mod fetch;
#[cfg(feature = "server")]
mod prove;
#[cfg(all(feature = "server", any(test, feature = "structure")))]
pub(crate) mod structure;

#[cfg(any(feature = "server", feature = "verify"))]
use crate::drive::{Drive, RootTree};
#[cfg(feature = "server")]
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
#[cfg(feature = "server")]
use crate::util::batch::GroveDbOpBatch;

/// The key for prefunded balances for voting
pub const PREFUNDED_BALANCES_FOR_VOTING: u8 = 128;

/// The key for prefunded balances funding compilation readiness rounds. Provisional under the
/// allocation register's prefunded purpose row: one purpose key per fund kind, beside the
/// voting funds, each a sum tree so the root sum keeps covering every fund.
pub const PREFUNDED_BALANCES_FOR_READINESS: u8 = 129;

/// prefunded specialized balances for voting
pub fn prefunded_specialized_balances_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(
        RootTree::PreFundedSpecializedBalances,
    )]
}

/// prefunded specialized balances vector
pub fn prefunded_specialized_balances_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::PreFundedSpecializedBalances).to_vec()]
}

/// prefunded specialized balances for voting
pub fn prefunded_specialized_balances_for_voting_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::PreFundedSpecializedBalances),
        &[PREFUNDED_BALANCES_FOR_VOTING],
    ]
}

/// prefunded specialized balances for voting vector
pub fn prefunded_specialized_balances_for_voting_path_vec() -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::PreFundedSpecializedBalances).to_vec(),
        vec![PREFUNDED_BALANCES_FOR_VOTING],
    ]
}

/// prefunded specialized balances for compilation readiness
pub fn prefunded_specialized_balances_for_readiness_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::PreFundedSpecializedBalances),
        &[PREFUNDED_BALANCES_FOR_READINESS],
    ]
}

/// prefunded specialized balances for compilation readiness vector
pub fn prefunded_specialized_balances_for_readiness_path_vec() -> Vec<Vec<u8>> {
    vec![
        Into::<&[u8; 1]>::into(RootTree::PreFundedSpecializedBalances).to_vec(),
        vec![PREFUNDED_BALANCES_FOR_READINESS],
    ]
}

impl Drive {
    #[cfg(feature = "server")]
    /// Add operations for creating initial prefunded specialized balances state structure
    /// In v1 we will only have the prefunded balances for voting
    /// In the future, we could use this for allowing for "free" state transitions as long as the
    /// state transition matches specific criteria.
    /// For example let's say you make a food delivery app, and you want to pay for when your
    /// customers make an order, the restaurant or food delivery app might prepay for all documents
    /// that make an order
    ///
    /// The readiness fund tree (`129`) is not created here: this helper is unversioned and
    /// genesis builds the lower layers in one batch, so the tree is added by the versioned
    /// vote setup (`add_initial_vote_tree_main_structure_operations` generation 1) and, on
    /// upgrade, by the protocol change hook.
    pub fn add_initial_prefunded_specialized_balances_operations(batch: &mut GroveDbOpBatch) {
        batch.add_insert_empty_sum_tree(
            vec![vec![RootTree::PreFundedSpecializedBalances as u8]],
            vec![PREFUNDED_BALANCES_FOR_VOTING],
        );
    }
}
