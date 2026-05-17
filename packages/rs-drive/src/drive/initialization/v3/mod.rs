//! Drive Initialization

use crate::drive::shielded::nullifiers::queries::*;
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

        // Add shielded pool structures
        self.initial_state_structure_shielded_pool_operations(&mut batch)?;

        self.grove_apply_batch(batch, false, transaction, drive_version)?;

        Ok(())
    }

    /// Adds shielded pool batch operations for initialization.
    ///
    /// The main shielded credit pool lives under `RootTree::ShieldedBalances`
    /// at key `MAIN_SHIELDED_CREDIT_POOL_KEY` (`b"M"`).
    ///
    /// The eight subtree inserts inside the pool are ordered breadth-first to
    /// match the intended balanced shape of the parent Merk tree (see the
    /// layout diagram in `crate::drive::shielded::paths`): root first, then
    /// both depth-1 children, then the four depth-2 children, then the
    /// depth-3 leaf. AVL rebalancing is order-sensitive, so this ordering is
    /// what actually places `SHIELDED_NOTES_KEY` at the root and the
    /// spend-path keys at depth 1.
    pub(in crate::drive::initialization) fn initial_state_structure_shielded_pool_operations(
        &self,
        batch: &mut GroveDbOpBatch,
    ) -> Result<(), Error> {
        // Parent: main shielded credit pool SumTree under ShieldedBalances.
        // Must be inserted before any of its children so the subtree exists.
        batch.add_insert(
            vec![vec![RootTree::ShieldedBalances as u8]],
            vec![MAIN_SHIELDED_CREDIT_POOL_KEY_U8],
            Element::empty_sum_tree(),
        );

        // Level 0 (root): notes tree (CommitmentTree = CountTree items + Sinsemilla Frontier)
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_NOTES_KEY],
            Element::empty_commitment_tree(SHIELDED_NOTES_CHUNK_POWER)?,
        );

        // Level 1 (left): nullifiers tree (ProvableCountTree) — checked on every spend.
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_NULLIFIERS_KEY],
            Element::empty_provable_count_tree(),
        );

        // Level 1 (right): anchors tree (NormalTree) — checked on every spend.
        // Stores anchor_bytes → block_height_be.
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_ANCHORS_IN_POOL_KEY],
            Element::empty_tree(),
        );

        // Level 2: total balance SumItem(0).
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_TOTAL_BALANCE_KEY],
            Element::new_sum_item(0),
        );

        // Level 2: anchors-by-height tree (NormalTree) — block_height_be → anchor_bytes.
        // Reverse index for pruning old anchors by height range and the
        // canonical source of the most-recent anchor (read via `limit 1`
        // reverse query) — there is no separate "most recent" slot; key 7
        // was retired because the duplicate state could desync from the
        // anchors tree under prune.
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_ANCHORS_BY_HEIGHT_KEY],
            Element::empty_tree(),
        );

        // Level 2: per-block recent-nullifiers CountSumTree, wrapped in
        // NotSummed so its sum side (the per-block nullifier count, stored as
        // the sum half of each ItemWithSumItem) does NOT propagate into the
        // enclosing shielded pool SumTree — and therefore not into
        // ShieldedBalances either. Without the wrapper, every spent nullifier
        // would inflate the "credits in pool" aggregate by 1.
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_RECENT_NULLIFIERS_KEY_U8],
            Element::new_not_summed(Element::empty_count_sum_tree())?,
        );

        // Level 2: compacted nullifiers NormalTree.
        // Key: (start_block, end_block) as 16 bytes, Value: serialized Vec<[u8;32]>.
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_COMPACTED_NULLIFIERS_KEY_U8],
            Element::empty_tree(),
        );

        // Level 3: nullifiers-expiration-time NormalTree (deepest leaf).
        batch.add_insert(
            shielded_credit_pool_path_vec(),
            vec![SHIELDED_NULLIFIERS_EXPIRATION_TIME_KEY_U8],
            Element::empty_tree(),
        );

        Ok(())
    }
}
