use crate::drive::shielded::paths::{
    shielded_credit_pool_path, MAIN_SHIELDED_CREDIT_POOL_KEY_U8, SHIELDED_ANCHORS_BY_HEIGHT_KEY,
    SHIELDED_ANCHORS_IN_POOL_KEY, SHIELDED_NOTES_CHUNK_POWER, SHIELDED_NOTES_KEY,
    SHIELDED_NULLIFIERS_KEY, SHIELDED_TOTAL_BALANCE_KEY,
};
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;

impl Drive {
    /// Inserts the main shielded credit pool and its five child subtrees under
    /// an already-existing top-level `RootTree::ShieldedBalances` SumTree.
    ///
    /// CONSENSUS-CRITICAL: this is the single source of truth for the shielded
    /// pool's internal GroveDB shape. Both the fresh-genesis-v12 path
    /// (`Drive::create_initial_state_structure_v3`) and the in-place upgrade
    /// path (`Platform::transition_to_version_12`) call this helper so the two
    /// node populations build a byte-identical `[ShieldedBalances]` subtree. The
    /// previous implementation built the pool two different ways — genesis via a
    /// sorted `GroveDbOpBatch` (which roots the parent Merk at the batch's
    /// median key `[160]`) and the upgrade via the sequential breadth-first
    /// inserts below (which root it at `[128]`, the intended NOTES-at-root
    /// layout from `crate::drive::shielded::paths`). That divergence produced
    /// two different subtree root hashes for the same logical structure and
    /// would have forked a state-synced v12 node from an upgraded one.
    ///
    /// The construction is SEQUENTIAL (one `grove_insert_if_not_exists` per
    /// element) and the child order is breadth-first, because AVL rebalancing is
    /// order-sensitive: this exact ordering is what places `SHIELDED_NOTES_KEY`
    /// (`[128]`) at the root of the parent Merk and the spend-path keys at depth
    /// 1. Do not reorder these inserts and do not move them into a batch.
    ///
    /// The caller MUST have already created the top-level
    /// `RootTree::ShieldedBalances` SumTree; this helper only fills it in.
    ///
    /// # Parameters
    ///
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `platform_version`: The platform version used to select grove method
    ///   versions.
    pub fn insert_shielded_pool_structure(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Main shielded credit pool SumTree: [ShieldedBalances] / "M".
        // Must be inserted before any of its children so the subtree exists.
        self.grove_insert_if_not_exists(
            SubtreePath::from(&[&[RootTree::ShieldedBalances as u8] as &[u8]]),
            &[MAIN_SHIELDED_CREDIT_POOL_KEY_U8],
            Element::empty_sum_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;

        // The five child inserts below are ordered breadth-first to match the
        // intended balanced shape of the parent Merk tree (see the layout
        // diagram in `crate::drive::shielded::paths`). AVL rebalancing is
        // order-sensitive, so this ordering is what actually places
        // `SHIELDED_NOTES_KEY` at the root and the spend-path keys at depth 1.

        let shielded_pool_path = shielded_credit_pool_path();

        // Level 0 (root): notes tree (CommitmentTree = CountTree items + Sinsemilla Frontier)
        // [ShieldedBalances, "M"] / [128]
        self.grove_insert_if_not_exists(
            (&shielded_pool_path).into(),
            &[SHIELDED_NOTES_KEY],
            Element::empty_commitment_tree(SHIELDED_NOTES_CHUNK_POWER)
                .expect("SHIELDED_NOTES_CHUNK_POWER is valid"),
            transaction,
            None,
            &platform_version.drive,
        )?;

        // Level 1 (left): nullifiers tree (ProvableCountTree) — checked on every spend.
        // [ShieldedBalances, "M"] / [64]
        self.grove_insert_if_not_exists(
            (&shielded_pool_path).into(),
            &[SHIELDED_NULLIFIERS_KEY],
            Element::empty_provable_count_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;

        // Level 1 (right): anchors tree (NormalTree) — anchor_bytes → block_height_be.
        // [ShieldedBalances, "M"] / [192]
        self.grove_insert_if_not_exists(
            (&shielded_pool_path).into(),
            &[SHIELDED_ANCHORS_IN_POOL_KEY],
            Element::empty_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;

        // Level 2: total balance SumItem(0).
        // [ShieldedBalances, "M"] / [32]
        self.grove_insert_if_not_exists(
            (&shielded_pool_path).into(),
            &[SHIELDED_TOTAL_BALANCE_KEY],
            Element::new_sum_item(0),
            transaction,
            None,
            &platform_version.drive,
        )?;

        // Level 2: anchors-by-height tree (NormalTree) — block_height_be → anchor_bytes.
        // [ShieldedBalances, "M"] / [96]
        self.grove_insert_if_not_exists(
            (&shielded_pool_path).into(),
            &[SHIELDED_ANCHORS_BY_HEIGHT_KEY],
            Element::empty_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;

        Ok(())
    }
}
