use crate::drive::{unique_key_hashes_tree_path, Drive};
use crate::util::grove_operations::DirectQueryType::StatefulDirectQuery;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Does a key with that public key hash already exist in the unique tree?
    pub(super) fn has_unique_public_key_hash_v0(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.has_unique_public_key_hash_operations_v0(
            public_key_hash,
            transaction,
            &mut drive_operations,
            drive_version,
        )
    }

    /// Operations for if a key with that public key hash already exists in the unique set?
    pub(super) fn has_unique_public_key_hash_operations_v0(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        let unique_key_hashes = unique_key_hashes_tree_path();
        self.grove_has_raw(
            (&unique_key_hashes).into(),
            public_key_hash.as_slice(),
            StatefulDirectQuery,
            transaction,
            drive_operations,
            drive_version,
        )
    }
}
