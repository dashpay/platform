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
use dpp::Convertible;
use grovedb::query_result_type::QueryResultType;

use dpp::version::drive_versions::DriveVersion;
use grovedb::Element::Item;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeMap;
use std::ops::RangeFull;

impl Drive {
    /// Fetches an identity with all its information from storage.
    pub(super) fn fetch_serialized_full_identity_by_unique_public_key_hash_v0(
        &self,
        public_key_hash: [u8; 20],
        encoding: QueryResultEncoding,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let identity = self.fetch_full_identity_by_unique_public_key_hash(
            public_key_hash,
            transaction,
            drive_version,
        )?;

        let identity_value = match identity {
            None => Value::Null,
            Some(identity) => identity.to_cleaned_object()?,
        };
        encoding.encode_value(&identity_value)
    }
}
