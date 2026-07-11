//! Drive Initialization

use crate::drive::{Drive, RootTree};
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{TransactionArg, TreeType};
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

        // ShieldedBalances top-level sum tree — separate from AddressBalances so
        // per-pool internal trees (notes, nullifiers, anchors, …) cannot
        // contaminate the address-credit aggregate via sum propagation.
        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::ShieldedBalances as u8],
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

        // Add the shielded pool structures AFTER the batch apply so the
        // top-level `[ShieldedBalances]` SumTree (inserted above) already
        // exists. CONSENSUS-CRITICAL: this MUST go through the shared
        // `insert_shielded_pool_structure` helper — the same sequential builder
        // the upgrade path (`Platform::transition_to_version_12`) uses — so a
        // fresh-genesis-v12 node and an in-place-upgraded v12 node build a
        // byte-identical `[ShieldedBalances]` subtree. Building the pool here in
        // the sorted `GroveDbOpBatch` instead would root the parent Merk at the
        // batch's median key (`[160]`) rather than the intended NOTES-at-root
        // (`[128]`) layout, diverging from the upgrade path and forking the
        // network at the v11→v12 boundary.
        self.insert_shielded_pool_structure(transaction, platform_version)?;

        Ok(())
    }
}
