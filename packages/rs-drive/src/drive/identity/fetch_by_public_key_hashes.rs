use grovedb::Element::Item;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use integer_encoding::VarInt;
use dpp::identifier::Identifier;
use dpp::identity::Identity;
use crate::drive::{Drive, unique_key_hashes_tree_path, unique_key_hashes_tree_path_vec};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::grove_operations::DirectQueryType;
use crate::drive::grove_operations::DirectQueryType::StatefulDirectQuery;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::identity::identity_path;
use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;

impl Drive {
    /// Fetches an identity id with all its information from storage.
    pub fn fetch_identity_id_by_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
    ) -> Result<Option<[u8; 32]>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_identity_id_by_unique_public_key_hash_operations(public_key_hash, transaction, &mut drive_operations)
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(crate) fn fetch_identity_id_by_unique_public_key_hash_operations(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<[u8; 32]>, Error> {
        let unique_key_hashes = unique_key_hashes_tree_path();
        match self.grove_get_raw(
            unique_key_hashes,
            public_key_hash.as_slice(),
            StatefulDirectQuery,
            transaction,
            drive_operations,
        ) {
            Ok(Some(Item(identity_id, _))) => {
                if identity_id.len() != 32 {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                        "identity id should be 32 bytes".to_string(),
                    )));
                }
                Ok(Some(owner_id.as_slice().try_into()))
            }

            Ok(None) => Ok(None),

            Ok(Some(..)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity public key hash was present but was not identified as an item",
            ))),

            Err(e) => Err(e),
        }
    }

    /// Fetches an identity with all its information from storage.
    pub fn fetch_full_identity_by_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
    ) -> Result<Option<Identity>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_full_identity_by_unique_public_key_hash_operations(public_key_hash, transaction, &mut drive_operations)
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(crate) fn fetch_full_identity_by_unique_public_key_hash_operations(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<Identity>, Error> {
        let identity_id = self.fetch_identity_id_by_unique_public_key_hash_operations(public_key_hash, transaction, drive_operations)?;
        if let Some(identity_id) = identity_id {
            self.fetch_full_identity(identity_id, transaction)
        } else {
            Ok(None)
        }
    }

    /// The query for the identity revision
    pub fn identity_id_by_public_key_hash_query(identity_id: [u8; 32]) -> PathQuery {
        let unique_key_hashes = unique_key_hashes_tree_path_vec();
        let mut query = Query::new();
        query.insert_key(identity_id.to_vec());
        PathQuery {
            path: unique_key_hashes,
            query: SizedQuery {
                query,
                limit: Some(1),
                offset: None,
            },
        }
    }

    /// This query gets the full identity and the public key hash
    pub fn full_identity_with_public_key_hash_query(public_key_hash: [u8; 20], identity_id: [u8; 32]) -> Result<PathQuery, Error> {
        let full_identity_query = Self::full_identity_query(identity_id)?;
        let identity_id_by_public_key_hash_query = Self::identity_id_by_public_key_hash_query(public_key_hash)?;
        // let key_request = IdentityKeysRequest::new_all_keys_query(identity_id);
        // let all_keys_query = key_request.into_path_query();
        PathQuery::merge(vec![&full_identity_query, &identity_id_by_public_key_hash_query]).map_err(Error::GroveDB)
    }

    /// Fetches an identity with all its information from storage.
    pub fn fetch_proved_full_identity_by_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_proved_full_identity_by_unique_public_key_hash_operations(public_key_hash, transaction, &mut drive_operations)
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(crate) fn fetch_proved_full_identity_by_unique_public_key_hash_operations(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Vec<u8>, Error> {
        let identity_id = self.fetch_identity_id_by_unique_public_key_hash_operations(public_key_hash, transaction, drive_operations)?;
        if let Some(identity_id) = identity_id {
            let query = Self::full_identity_with_public_key_hash_query(public_key_hash, identity_id)?;
            self.grove_get_proved_path_query(&query, transaction, drive_operations)
        } else {
            // We only prove the absence of the public key hash
            let query = Self::identity_id_by_public_key_hash_query(public_key_hash);
            self.grove_get_proved_path_query(&query, transaction, drive_operations)
        }
    }
}