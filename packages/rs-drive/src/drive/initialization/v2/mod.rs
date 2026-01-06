//! Drive Initialization

use crate::drive::address_funds::queries::CLEAR_ADDRESS_POOL;
use crate::drive::saved_block_transactions::{
    ADDRESS_BALANCES_KEY_U8, COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY_U8,
    COMPACTED_ADDRESS_BALANCES_KEY_U8,
};
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg, TreeType};
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
            TreeType::NormalTree,
            transaction,
            None,
            &mut vec![],
            drive_version,
        )?;

        //This is new in v2
        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::AddressBalances as u8],
            TreeType::SumTree,
            transaction,
            None,
            &mut vec![],
            drive_version,
        )?;

        // SavedBlockTransactions for address-based transaction sync
        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::SavedBlockTransactions as u8],
            TreeType::NormalTree,
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
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.initial_state_structure_lower_layers_add_operations_1(batch, platform_version)?;

        batch.add_insert(
            Self::addresses_path(),
            CLEAR_ADDRESS_POOL.to_vec(),
            Element::empty_provable_count_sum_tree(),
        );

        // Address balances subtree under SavedBlockTransactions for storing
        // address balance changes per block. Uses a count sum tree where:
        // - Each item is an ItemWithSumItem (serialized data + entry count as sum)
        // - The tree tracks total block count and total address balance entry count
        batch.add_insert(
            Self::saved_block_transactions_path(),
            vec![ADDRESS_BALANCES_KEY_U8],
            Element::empty_count_sum_tree(),
        );

        // Compacted address balances subtree under SavedBlockTransactions for storing
        // compacted/aggregated address balance changes spanning multiple blocks.
        // Each item is stored with a (start_block, end_block) tuple key.
        batch.add_insert(
            Self::saved_block_transactions_path(),
            vec![COMPACTED_ADDRESS_BALANCES_KEY_U8],
            Element::empty_tree(),
        );

        // Compacted addresses expiration time subtree under SavedBlockTransactions for storing
        // expiration timestamps (in milliseconds) for compacted address balance ranges.
        // Each item uses the same (start_block, end_block) key as the compacted balances,
        // with the value being the expiration time (block time + 1 day).
        batch.add_insert(
            Self::saved_block_transactions_path(),
            vec![COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY_U8],
            Element::empty_tree(),
        );

        Ok(())
    }
}
