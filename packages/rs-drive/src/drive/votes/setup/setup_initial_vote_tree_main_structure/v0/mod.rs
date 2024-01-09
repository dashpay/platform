use crate::drive::votes::{
    vote_contested_resource_end_date_queries_tree_path,
    vote_contested_resource_identity_votes_tree_path, vote_root_path, CONTESTED_RESOURCE_TREE_KEY,
    VOTE_DECISIONS_TREE_KEY,
};
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use grovedb::operations::insert::InsertOptions;
use grovedb::TransactionArg;
use grovedb_path::SubtreePath;
use platform_version::version::PlatformVersion;

impl Drive {
    pub fn setup_initial_vote_tree_main_structure_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;

        let mut drive_operations = vec![];

        self.grove_insert_empty_tree(
            SubtreePath::from(vote_root_path()),
            &[VOTE_DECISIONS_TREE_KEY.into()],
            transaction,
            Some(InsertOptions {
                validate_insertion_does_not_override: true,
                validate_insertion_does_not_override_tree: true,
                base_root_storage_is_free: true,
            }),
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_tree(
            SubtreePath::from(vote_root_path()),
            &[CONTESTED_RESOURCE_TREE_KEY.into()],
            transaction,
            Some(InsertOptions {
                validate_insertion_does_not_override: true,
                validate_insertion_does_not_override_tree: true,
                base_root_storage_is_free: true,
            }),
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_tree(
            SubtreePath::from(vote_contested_resource_end_date_queries_tree_path()),
            &[CONTESTED_RESOURCE_TREE_KEY.into()],
            transaction,
            Some(InsertOptions {
                validate_insertion_does_not_override: true,
                validate_insertion_does_not_override_tree: true,
                base_root_storage_is_free: true,
            }),
            &mut drive_operations,
            drive_version,
        )?;

        self.grove_insert_empty_tree(
            SubtreePath::from(vote_contested_resource_identity_votes_tree_path()),
            &[CONTESTED_RESOURCE_TREE_KEY.into()],
            transaction,
            Some(InsertOptions {
                validate_insertion_does_not_override: true,
                validate_insertion_does_not_override_tree: true,
                base_root_storage_is_free: true,
            }),
            &mut drive_operations,
            drive_version,
        )
    }
}
