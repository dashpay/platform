//! Drive Initialization

use crate::drive::{Drive, RootTree};
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;
use crate::drive::address_funds::queries::CLEAR_ADDRESS_POOL;
use crate::drive::system::misc_path_vec;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;

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

        self.initial_state_structure_lower_layers_add_operations_2(&mut batch, platform_version)?;

        self.grove_apply_batch(batch, false, transaction, drive_version)?;

        Ok(())
    }


    /// Creates the initial state structure.
    pub(in crate::drive::initialization) fn initial_state_structure_lower_layers_add_operations_2(
        &self,
        batch: &mut GroveDbOpBatch,
        _platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.initial_state_structure_lower_layers_add_operations_1(batch, _platform_version)?;

        batch.add_insert(
            misc_path_vec(),
            CLEAR_ADDRESS_POOL.to_vec(),
            Element::empty_sum_tree(),
        );

        Ok(())
    }
}
