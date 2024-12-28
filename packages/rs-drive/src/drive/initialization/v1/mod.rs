//! Drive Initialization

use crate::drive::balances::TOTAL_TOKEN_SUPPLIES_STORAGE_KEY;
use crate::util::batch::GroveDbOpBatch;

use crate::drive::system::misc_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;

use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path_vec, WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY,
    WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
};
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Creates the initial state structure.
    pub(super) fn create_initial_state_structure_1(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        self.create_initial_state_structure_top_level_0(transaction, platform_version)?;

        // On lower layers we can use batching

        let mut batch =
            self.create_initial_state_structure_lower_layers_operations_0(platform_version)?;

        self.initial_state_structure_lower_layers_add_operations_1(&mut batch, platform_version)?;

        self.grove_apply_batch(batch, false, transaction, drive_version)?;

        Ok(())
    }

    /// Creates the initial state structure.
    pub(in crate::drive::initialization) fn initial_state_structure_lower_layers_add_operations_1(
        &self,
        batch: &mut GroveDbOpBatch,
        _platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // In Misc
        batch.add_insert(
            misc_path_vec(),
            TOTAL_TOKEN_SUPPLIES_STORAGE_KEY.to_vec(),
            Element::empty_tree(),
        );

        // We are adding the withdrawal transactions sum amount tree
        let path = get_withdrawal_root_path_vec();

        batch.add_insert(
            path.clone(),
            WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY.to_vec(),
            Element::empty_sum_tree(),
        );

        batch.add_insert(
            path,
            WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY.to_vec(),
            Element::empty_tree(),
        );

        Ok(())
    }
}
