use crate::drive::{non_unique_key_hashes_sub_tree_path, Drive};
use crate::util::grove_operations::DirectQueryType::StatefulDirectQuery;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Operations for if a key with that public key hash already exists in the non unique set?
    /// For a particular identity
    pub(super) fn has_non_unique_public_key_hash_already_for_identity_operations_v0(
        &self,
        public_key_hash: [u8; 20],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        let public_key_hash_sub_tree =
            non_unique_key_hashes_sub_tree_path(public_key_hash.as_slice());
        // this will actually get a tree
        self.grove_has_raw(
            (&public_key_hash_sub_tree).into(),
            identity_id.as_slice(),
            StatefulDirectQuery,
            transaction,
            drive_operations,
            drive_version,
        )
    }
}
