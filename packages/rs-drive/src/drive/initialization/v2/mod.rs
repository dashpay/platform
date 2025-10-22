//! Drive Initialization

use crate::drive::balances::TOTAL_TOKEN_SUPPLIES_STORAGE_KEY;
use crate::util::batch::GroveDbOpBatch;

use crate::drive::system::misc_path_vec;
use crate::drive::tokens::paths::{
    token_distributions_root_path_vec, token_timed_distributions_path_vec, tokens_root_path_vec,
    TOKEN_BALANCES_KEY, TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY, TOKEN_CONTRACT_INFO_KEY,
    TOKEN_DIRECT_SELL_PRICE_KEY, TOKEN_DISTRIBUTIONS_KEY, TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY,
    TOKEN_IDENTITY_INFO_KEY, TOKEN_MS_TIMED_DISTRIBUTIONS_KEY, TOKEN_PERPETUAL_DISTRIBUTIONS_KEY,
    TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY, TOKEN_STATUS_INFO_KEY, TOKEN_TIMED_DISTRIBUTIONS_KEY,
};
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;

impl Drive {
    /// Creates the initial state structure.
    pub(super) fn create_initial_state_structure_v2(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        self.create_initial_state_structure_top_level_0(transaction, platform_version)?;

        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::GroupActions as u8],
            transaction,
            None,
            &mut vec![],
            drive_version,
        )?;

        //This is new in v2
        self.grove_insert_empty_sum_tree(
            SubtreePath::empty(),
            &[RootTree::SingleUseKeyBalances as u8],
            transaction,
            None,
            &mut vec![],
            drive_version,
        )?;

        // On lower layers we can use batching

        let mut batch =
            self.create_initial_state_structure_lower_layers_operations_0(platform_version)?;

        self.initial_state_structure_lower_layers_add_operations_1(&mut batch, platform_version)?;

        self.grove_apply_batch(batch, false, transaction, drive_version)?;

        Ok(())
    }
}
