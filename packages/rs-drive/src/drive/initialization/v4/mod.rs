//! Drive Initialization

use crate::drive::{Drive, RootTree};
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{TransactionArg, TreeType};
use grovedb_path::SubtreePath;

impl Drive {
    /// Creates the initial state structure.
    ///
    /// v4 adds the token contract lifecycle ledger under `[Tokens]` (introduced at protocol
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

        // The token contract lifecycle ledger goes through the same sequential helper the
        // upgrade path (`Platform::transition_to_version_17`) calls, for the reason given
        // above: the ledger has two one-byte keys whose Merk placement depends on the
        // insertion order, and a fresh genesis-v17 node and an in-place-upgraded v17 node
        // must hold a byte-identical `[Tokens] / 224` subtree.
        self.insert_token_contract_lifecycles_structure(transaction, platform_version)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::tokens::lifecycle::encode_destroyed_supply;
    use crate::drive::tokens::paths::{
        token_contract_lifecycles_root_path, tokens_root_path, TOKEN_CONTRACT_LIFECYCLES_KEY,
        TOKEN_DESTROYED_SUPPLY_KEY, TOKEN_LIFECYCLE_CLEANUP_QUEUE_KEY,
    };
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    #[test]
    fn should_create_the_token_contract_lifecycle_ledger_at_latest_genesis() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let grove_version = &platform_version.drive.grove_version;

        let ledger = drive
            .grove
            .get(
                &tokens_root_path(),
                &[TOKEN_CONTRACT_LIFECYCLES_KEY],
                None,
                grove_version,
            )
            .unwrap()
            .expect("the lifecycle ledger should exist at the latest genesis");
        assert_eq!(ledger, Element::empty_tree());

        let destroyed_supply = drive
            .grove
            .get(
                &token_contract_lifecycles_root_path(),
                &TOKEN_DESTROYED_SUPPLY_KEY,
                None,
                grove_version,
            )
            .unwrap()
            .expect("the destroyed supply scalar should exist");
        assert_eq!(
            destroyed_supply,
            Element::new_item(encode_destroyed_supply(0))
        );

        let queue = drive
            .grove
            .get(
                &token_contract_lifecycles_root_path(),
                &TOKEN_LIFECYCLE_CLEANUP_QUEUE_KEY,
                None,
                grove_version,
            )
            .unwrap()
            .expect("the cleanup queue should exist");
        assert_eq!(queue, Element::empty_tree());
    }

    #[test]
    fn should_not_create_the_ledger_at_protocol_14_genesis() {
        let platform_version = PlatformVersion::get(14).expect("protocol version 14 exists");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        let result = drive
            .grove
            .get(
                &tokens_root_path(),
                &[TOKEN_CONTRACT_LIFECYCLES_KEY],
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap();

        assert!(
            result.is_err(),
            "a protocol 14 genesis must not contain the lifecycle ledger, got {result:?}"
        );
    }

    #[test]
    fn should_build_the_same_ledger_when_the_structure_is_inserted_twice() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let grove_version = &platform_version.drive.grove_version;
        let root_hash_before = drive
            .grove
            .root_hash(None, grove_version)
            .unwrap()
            .expect("expected a root hash");

        drive
            .insert_token_contract_lifecycles_structure(None, platform_version)
            .expect("expected the second insert to be a no-op");

        let root_hash_after = drive
            .grove
            .root_hash(None, grove_version)
            .unwrap()
            .expect("expected a root hash");
        assert_eq!(root_hash_before, root_hash_after);
    }
}
