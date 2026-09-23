use crate::drive::votes::paths::{
    vote_decisions_tree_path, ACTIVE_POLLS_TREE_KEY, IDENTITY_VOTES_TREE_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Creates the active polls and identity votes trees of the decisions branch when they are
    /// missing. A chain upgrading to protocol version 14 calls this on its first block and a
    /// chain born at 14 calls it from `create_initial_state_structure` v4, so both populations
    /// build the same votes Merk.
    pub fn insert_vote_decisions_trees(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for subtree_key in [ACTIVE_POLLS_TREE_KEY, IDENTITY_VOTES_TREE_KEY] {
            self.grove_insert_if_not_exists(
                vote_decisions_tree_path().as_slice().into(),
                &[subtree_key as u8],
                Element::empty_tree(),
                transaction,
                None,
                &platform_version.drive,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::votes::paths::{
        vote_decisions_active_polls_tree_path, vote_decisions_identity_votes_tree_path,
    };
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_create_the_decisions_trees_once_and_keep_them() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        // Genesis at the latest version already made them; a second run changes nothing.
        let root_hash_before = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("root hash");
        drive
            .insert_vote_decisions_trees(None, platform_version)
            .expect("expected to insert the decisions trees");
        let root_hash_after = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("root hash");
        assert_eq!(root_hash_before, root_hash_after);
        for path in [
            vote_decisions_active_polls_tree_path(),
            vote_decisions_identity_votes_tree_path(),
        ] {
            let (parent, key) = path.split_at(3);
            let _ = key;
            let element = drive
                .grove
                .get(
                    &parent[..2],
                    path[2],
                    None,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .expect("expected the tree");
            assert!(element.is_any_tree());
        }
    }
}
