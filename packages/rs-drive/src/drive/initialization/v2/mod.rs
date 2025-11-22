//! Drive Initialization

use crate::drive::{Drive, RootTree};
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
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
            &[RootTree::AddressBalances as u8],
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
