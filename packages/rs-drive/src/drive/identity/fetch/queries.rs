use crate::drive::grove_operations::DirectQueryType::StatefulDirectQuery;
use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::{
    non_unique_key_hashes_sub_tree_path, non_unique_key_hashes_tree_path,
    unique_key_hashes_tree_path, unique_key_hashes_tree_path_vec, Drive,
};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use dpp::identity::Identity;
use grovedb::Element::Item;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeMap;

impl Drive {
    /// The query for proving an identity id from a public key hash.
    pub fn identity_id_by_unique_public_key_hash_query(public_key_hash: [u8; 20]) -> PathQuery {
        let unique_key_hashes = unique_key_hashes_tree_path_vec();
        PathQuery::new_single_key(unique_key_hashes, public_key_hash.to_vec())
    }

    /// The query for proving identity ids from a vector of public key hashes.
    pub fn identity_ids_by_unique_public_key_hash_query(
        public_key_hashes: &[[u8; 20]],
    ) -> PathQuery {
        let unique_key_hashes = unique_key_hashes_tree_path_vec();
        let mut query = Query::new();
        query.insert_keys(
            public_key_hashes
                .into_iter()
                .map(|key_hash| key_hash.to_vec())
                .collect(),
        );
        PathQuery::new_unsized(unique_key_hashes, query)
    }

    /// The query getting all keys and balance and revision
    pub fn full_identity_query(identity_id: &[u8; 32]) -> Result<PathQuery, Error> {
        let balance_query = Self::identity_balance_query(identity_id);
        let revision_query = Self::identity_revision_query(identity_id);
        let key_request = IdentityKeysRequest::new_all_keys_query(identity_id);
        let all_keys_query = key_request.into_path_query();
        PathQuery::merge(vec![&balance_query, &revision_query, &all_keys_query])
            .map_err(Error::GroveDB)
    }

    /// The query getting all keys and balance and revision
    pub fn full_identities_query(identity_ids: &[[u8; 32]]) -> Result<PathQuery, Error> {
        let path_queries: Vec<PathQuery> = identity_ids
            .into_iter()
            .map(|identity_id| Self::full_identity_query(identity_id))
            .collect::<Result<Vec<PathQuery>, Error>>()?;
        PathQuery::merge(path_queries.iter().map(|query| query).collect()).map_err(Error::GroveDB)
    }

    /// This query gets the full identity and the public key hash
    pub fn full_identity_with_public_key_hash_query(
        public_key_hash: [u8; 20],
        identity_id: [u8; 32],
    ) -> Result<PathQuery, Error> {
        let full_identity_query = Self::full_identity_query(&identity_id)?;
        let identity_id_by_public_key_hash_query =
            Self::identity_id_by_unique_public_key_hash_query(public_key_hash);
        PathQuery::merge(vec![
            &full_identity_query,
            &identity_id_by_public_key_hash_query,
        ])
        .map_err(Error::GroveDB)
    }

    /// The query full identities with key hashes too
    pub fn full_identities_with_keys_hashes_query(
        identity_ids: &[[u8; 32]],
        key_hashes: &[[u8; 20]],
    ) -> Result<PathQuery, Error> {
        let identities_path_query = Self::full_identities_query(identity_ids)?;
        let key_hashes_to_identity_ids_query =
            Self::identity_ids_by_unique_public_key_hash_query(key_hashes);

        PathQuery::merge(vec![
            &identities_path_query,
            &key_hashes_to_identity_ids_query,
        ])
        .map_err(Error::GroveDB)
    }
}
