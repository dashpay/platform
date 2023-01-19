use crate::drive::grove_operations::DirectQueryType::StatefulDirectQuery;
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
    /// Fetches an identity with all its information from storage.
    pub fn prove_full_identity_by_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let identity_id = self.fetch_identity_id_by_unique_public_key_hash_operations(
            public_key_hash,
            transaction,
            &mut vec![],
        )?;
        if let Some(identity_id) = identity_id {
            let query =
                Self::full_identity_with_public_key_hash_query(public_key_hash, identity_id)?;
            self.grove_get_proved_path_query(&query, transaction, &mut vec![])
        } else {
            // We only prove the absence of the public key hash
            let query = Self::identity_id_by_unique_public_key_hash_query(public_key_hash);
            self.grove_get_proved_path_query(&query, transaction, &mut vec![])
        }
    }

    /// Given public key hashes, fetches full identities as proofs.
    pub fn prove_full_identities_by_unique_public_key_hashes(
        &self,
        public_key_hashes: Vec<[u8; 20]>,
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let identity_ids =
            self.fetch_identity_ids_by_unique_public_key_hashes(public_key_hashes, transaction)?;
        let path_queries = identity_ids
            .into_iter()
            .map(|(public_key_hash, maybe_identity_id)| {
                if let Some(identity_id) = maybe_identity_id {
                    Self::full_identity_with_public_key_hash_query(public_key_hash, identity_id)
                } else {
                    Ok(Self::identity_id_by_unique_public_key_hash_query(
                        public_key_hash,
                    ))
                }
            })
            .collect::<Result<Vec<PathQuery>, Error>>()?;

        let path_query = PathQuery::merge(path_queries.iter().collect()).map_err(Error::GroveDB)?;

        self.grove_get_proved_path_query(&path_query, transaction, &mut vec![])
    }
}
