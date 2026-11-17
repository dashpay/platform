//! Drive Initialization
//!
//! Version 5: version 4 plus the token marketplace escrow trees (protocol version 15).

use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Creates the initial state structure.
    pub(super) fn create_initial_state_structure_v5(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.create_initial_state_structure_v4(transaction, platform_version)?;

        // Token marketplace escrow trees (protocol version 15): the BigSumTree under the Tokens
        // tree of the tokens open listings hold, and the SumTree under the prefunded specialized
        // balances of the credits open offers hold. Last, after everything version 4 builds, and
        // through the same helper as the upgrade path (`Platform::transition_to_version_15`), so
        // a chain born at version 15 and one upgraded to it build byte-identical `[Tokens]` and
        // `[PreFundedSpecializedBalances]` subtrees.
        self.insert_token_order_escrow_trees(transaction, platform_version)?;

        Ok(())
    }
}
