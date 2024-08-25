use crate::drive::{unique_key_hashes_tree_path_vec, Drive};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use grovedb::query_result_type::QueryResultType;

use dpp::version::PlatformVersion;

use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    /// Do any keys with given public key hashes already exist in the unique tree?
    /// Will return public key hashes that already exist
    pub(super) fn has_any_of_unique_public_key_hashes_v0(
        &self,
        public_key_hashes: Vec<[u8; 20]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<[u8; 20]>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.has_any_of_unique_public_key_hashes_operations_v0(
            public_key_hashes,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Operations for if any keys with given public key hashes already exist in the unique tree.
    /// Will return public key hashes that already exist
    pub(super) fn has_any_of_unique_public_key_hashes_operations_v0(
        &self,
        public_key_hashes: Vec<[u8; 20]>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<[u8; 20]>, Error> {
        let unique_key_hashes = unique_key_hashes_tree_path_vec();
        let mut query = Query::new();
        query.insert_keys(
            public_key_hashes
                .into_iter()
                .map(|key_hash| key_hash.to_vec())
                .collect(),
        );
        let path_query = PathQuery::new(unique_key_hashes, SizedQuery::new(query, None, None));
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            &platform_version.drive,
        )?;
        results
            .to_keys()
            .into_iter()
            .map(|key| {
                key.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedElementType(
                        "as we pass 20 byte values we should get back 20 byte values",
                    ))
                })
            })
            .collect()
    }
}
