//! Drive Initialization

use crate::drive::{Drive, RootTree};
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{TransactionArg, TreeType};
use grovedb_path::SubtreePath;

impl Drive {
    /// Creates the initial state structure.
    ///
    /// v4 adds the `ContractCredits` root sum tree (introduced at protocol
    /// v17 / drive v10). Everything else is identical to v3.
    pub(super) fn create_initial_state_structure_v4(
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

        // ContractCredits top-level sum tree (introduced in v4). Contract credit
        // buckets live under it as `contract_id (SumTree) / bucket_key (SumItem)`,
        // so its aggregate is the total of live contract credits and is read by
        // credit conservation as its own term. CONSENSUS-CRITICAL: this is a
        // standalone non-batch root insert placed right after ShieldedBalances,
        // and the upgrade path (`Platform::transition_to_version_17`) creates
        // the same empty sum tree with an insert-if-not-exists, so a fresh
        // genesis-v17 node and an in-place-upgraded v17 node hold a
        // byte-identical `[ContractCredits]` element.
        self.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::ContractCredits as u8],
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

#[cfg(test)]
mod tests {
    use crate::drive::RootTree;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use grovedb::Element;
    use grovedb_path::SubtreePath;

    #[test]
    fn should_create_contract_credits_root_at_latest_genesis() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        let element = drive
            .grove
            .get(
                SubtreePath::empty(),
                &[RootTree::ContractCredits as u8],
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("the contract credits root should exist at the latest genesis");

        assert_eq!(
            element,
            Element::empty_sum_tree(),
            "the contract credits root must be an empty sum tree with no flags"
        );
    }

    #[test]
    fn should_not_create_contract_credits_root_at_protocol_14_genesis() {
        let platform_version = PlatformVersion::get(14).expect("protocol version 14 exists");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        let result = drive
            .grove
            .get(
                SubtreePath::empty(),
                &[RootTree::ContractCredits as u8],
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap();

        assert!(
            result.is_err(),
            "a protocol 14 genesis must not contain the contract credits root, got {result:?}"
        );
    }
}
