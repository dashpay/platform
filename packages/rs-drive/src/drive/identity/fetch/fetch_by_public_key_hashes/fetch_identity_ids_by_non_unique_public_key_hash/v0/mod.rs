use crate::drive::{non_unique_key_hashes_sub_tree_path_vec, Drive};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::QueryItem;

use grovedb::query_result_type::QueryResultType;

use dpp::version::drive_versions::DriveVersion;

use grovedb::{PathQuery, TransactionArg};

use std::ops::RangeFull;

impl Drive {
    /// Fetches identity ids from storage.
    pub(super) fn fetch_identity_ids_by_non_unique_public_key_hash_v0(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<[u8; 32]>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_ids_by_non_unique_public_key_hash_operations_v0(
            public_key_hash,
            transaction,
            &mut drive_operations,
            drive_version,
        )
    }

    /// Gets identity ids from non unique public key hashes.
    pub(super) fn fetch_identity_ids_by_non_unique_public_key_hash_operations_v0(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<[u8; 32]>, Error> {
        let non_unique_key_hashes = non_unique_key_hashes_sub_tree_path_vec(public_key_hash);
        let path_query = PathQuery::new_single_query_item(
            non_unique_key_hashes,
            QueryItem::RangeFull(RangeFull),
        );
        let (results, _) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            drive_version,
        )?;
        results
            .to_keys()
            .into_iter()
            .map(|key| {
                key.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "non unique public key hashes should point to identity ids of 32 bytes"
                            .to_string(),
                    ))
                })
            })
            .collect()
    }
}
