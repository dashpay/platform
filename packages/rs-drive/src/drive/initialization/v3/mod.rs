//! Drive Initialization

use crate::drive::shielded::paths::*;
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg, TreeType};
use grovedb_path::SubtreePath;

impl Drive {
    /// Creates the initial state structure.
    pub(super) fn create_initial_state_structure_v3(
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

        // AddressBalances top-level sum tree (introduced in v2)
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

        // Add shielded pool structures
        self.initial_state_structure_shielded_pool_operations(&mut batch)?;

        self.grove_apply_batch(batch, false, transaction, drive_version)?;

        Ok(())
    }

    /// Adds shielded pool batch operations for initialization.
    pub(in crate::drive::initialization) fn initial_state_structure_shielded_pool_operations(
        &self,
        batch: &mut GroveDbOpBatch,
    ) -> Result<(), Error> {
        // 1. Shielded credit pool SumTree under AddressBalances
        batch.add_insert(
            vec![vec![RootTree::AddressBalances as u8]],
            vec![SHIELDED_CREDIT_POOL_KEY_U8],
            Element::empty_sum_tree(),
        );

        // 2. Notes tree (CommitmentTree = CountTree items + Sinsemilla Frontier)
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_NOTES_KEY],
            Element::empty_commitment_tree(2048),
        );

        // 3. Nullifiers tree (NormalTree)
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_NULLIFIERS_KEY],
            Element::empty_tree(),
        );

        // 4. Total balance SumItem(0)
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_TOTAL_BALANCE_KEY],
            Element::new_sum_item(0),
        );

        // 5. Anchors tree (NormalTree) inside pool: block_height_be → anchor_bytes
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_ANCHORS_IN_POOL_KEY],
            Element::empty_tree(),
        );

        Ok(())
    }
}
