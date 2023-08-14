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
    /// Fetches an identity with all its information from storage.
    pub(super) fn fetch_full_identity_by_unique_public_key_hash_v0(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_full_identity_by_unique_public_key_hash_operations_v0(
            public_key_hash,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(super) fn fetch_full_identity_by_unique_public_key_hash_operations_v0(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        let identity_id = self.fetch_identity_id_by_unique_public_key_hash_operations(
            public_key_hash,
            transaction,
            drive_operations,
            platform_version,
        )?;
        if let Some(identity_id) = identity_id {
            self.fetch_full_identity(identity_id, transaction, platform_version)
        } else {
            Ok(None)
        }
    }
}
