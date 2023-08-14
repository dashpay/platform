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
use grovedb::Element::Item;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeMap;
use std::ops::RangeFull;

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
