use crate::drive::grove_operations::DirectQueryType::StatefulDirectQuery;
use crate::drive::{
    non_unique_key_hashes_sub_tree_path, non_unique_key_hashes_sub_tree_path_vec,
    non_unique_key_hashes_tree_path, unique_key_hashes_tree_path, unique_key_hashes_tree_path_vec,
    Drive,
};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::query::{QueryItem, QueryResultEncoding};
use dpp::identity::Identity;
use dpp::platform_value::Value;
use grovedb::query_result_type::QueryResultType;

use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeMap;
use std::ops::RangeFull;

impl Drive {
    /// Fetches an identity id with all its information from storage.
    pub(super) fn fetch_identity_id_by_unique_public_key_hash_v0(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<[u8; 32]>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_id_by_unique_public_key_hash_operations_v0(
            public_key_hash,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(super) fn fetch_identity_id_by_unique_public_key_hash_operations_v0(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<[u8; 32]>, Error> {
        let unique_key_hashes = unique_key_hashes_tree_path();
        match self.grove_get_raw(
            (&unique_key_hashes).into(),
            public_key_hash.as_slice(),
            StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(identity_id, _))) => identity_id
                .as_slice()
                .try_into()
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "identity id should be 32 bytes".to_string(),
                    ))
                })
                .map(Some),

            Ok(None) => Ok(None),

            Ok(Some(..)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity public key hash was present but was not identified as an item",
            ))),

            Err(e) => Err(e),
        }
    }
}
