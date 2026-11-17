use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_path, PREFUNDED_BALANCES_FOR_TOKEN_ORDERS,
};
use crate::drive::tokens::paths::{tokens_root_path, TOKEN_ORDER_ESCROW_KEY};
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;

impl Drive {
    /// Inserts the two escrow trees of the token marketplace (protocol version 15) if they do not
    /// exist yet:
    ///
    /// - `[Tokens, TOKEN_ORDER_ESCROW_KEY]`, an empty BigSumTree that holds one SumTree per
    ///   token, keyed by order id, of the tokens its open listings hold in escrow. It is counted
    ///   on the balance side of the token conservation check
    ///   (`calculate_total_tokens_balance` 2).
    /// - `[PreFundedSpecializedBalances, PREFUNDED_BALANCES_FOR_TOKEN_ORDERS]`, an empty SumTree,
    ///   keyed by order id, of the credits the open offers hold in escrow. The prefunded
    ///   specialized balances root is a SumTree, so the credit conservation check counts it
    ///   without a change.
    ///
    /// CONSENSUS-CRITICAL: both the genesis path (`Drive::create_initial_state_structure_v5`)
    /// and the upgrade path (`Platform::transition_to_version_15`) call this one helper, in the
    /// same position, so a chain born at protocol version 15 and a chain upgraded to it build
    /// byte-identical `[Tokens]` and `[PreFundedSpecializedBalances]` subtrees.
    pub fn insert_token_order_escrow_trees(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let tokens_root = tokens_root_path();
        self.grove_insert_if_not_exists(
            SubtreePath::from(&tokens_root),
            &[TOKEN_ORDER_ESCROW_KEY],
            Element::empty_big_sum_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;
        let prefunded_root = prefunded_specialized_balances_path();
        self.grove_insert_if_not_exists(
            SubtreePath::from(&prefunded_root),
            &[PREFUNDED_BALANCES_FOR_TOKEN_ORDERS],
            Element::empty_sum_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;
        Ok(())
    }
}
