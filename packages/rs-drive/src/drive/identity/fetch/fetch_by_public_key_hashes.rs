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

use grovedb::Element::Item;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeMap;
use std::ops::RangeFull;

impl Drive {
    /// Fetches an identity id with all its information from storage.
    pub fn fetch_identity_id_by_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
    ) -> Result<Option<[u8; 32]>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_id_by_unique_public_key_hash_operations(
            public_key_hash,
            transaction,
            &mut drive_operations,
        )
    }

    /// Fetches identity ids from storage.
    pub fn fetch_identity_ids_by_non_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
    ) -> Result<Vec<[u8; 32]>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_ids_by_non_unique_public_key_hash_operations(
            public_key_hash,
            transaction,
            &mut drive_operations,
        )
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(crate) fn fetch_identity_id_by_unique_public_key_hash_operations(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Option<[u8; 32]>, Error> {
        let unique_key_hashes = unique_key_hashes_tree_path();
        match self.grove_get_raw(
            (&unique_key_hashes).into(),
            public_key_hash.as_slice(),
            StatefulDirectQuery,
            transaction,
            drive_operations,
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

    /// Gets identity ids from non unique public key hashes.
    pub(crate) fn fetch_identity_ids_by_non_unique_public_key_hash_operations(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
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

    /// Fetches identity ids with all its information from storage.
    pub fn fetch_identity_ids_by_unique_public_key_hashes(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
    ) -> Result<BTreeMap<[u8; 20], Option<[u8; 32]>>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_ids_by_unique_public_key_hashes_operations(
            public_key_hashes,
            transaction,
            &mut drive_operations,
        )
    }

    /// Given public key hashes, fetches identity ids from storage.
    pub(crate) fn fetch_identity_ids_by_unique_public_key_hashes_operations(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<BTreeMap<[u8; 20], Option<[u8; 32]>>, Error> {
        let unique_key_hashes = unique_key_hashes_tree_path_vec();
        let mut query = Query::new();
        query.insert_keys(
            public_key_hashes
                .iter()
                .map(|key_hash| key_hash.to_vec())
                .collect(),
        );
        let sized_query = SizedQuery {
            query,
            limit: Some(public_key_hashes.len() as u16),
            offset: None,
        };
        let path_query = PathQuery::new(unique_key_hashes, sized_query);
        self.grove_get_raw_path_query_with_optional(&path_query, transaction, drive_operations)?
            .into_iter()
            .map(|(_, key, element)| {
                let identity_key_hash: [u8; 20] = key.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution("key hash not 20 bytes"))
                })?;
                match element {
                    Some(Item(identity_id_vec, ..)) => {
                        let identity_id: [u8; 32] = identity_id_vec.try_into().map_err(|_| {
                            Error::Drive(DriveError::CorruptedCodeExecution(
                                "key hash not 20 bytes",
                            ))
                        })?;
                        Ok((identity_key_hash, Some(identity_id)))
                    }
                    None => Ok((identity_key_hash, None)),
                    _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                        "unique public key hashes containing non identity ids".to_string(),
                    ))),
                }
            })
            .collect()
    }

    /// Does a key with that public key hash already exist in the unique tree?
    pub fn has_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
    ) -> Result<bool, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.has_unique_public_key_hash_operations(
            public_key_hash,
            transaction,
            &mut drive_operations,
        )
    }

    /// Operations for if a key with that public key hash already exists in the unique set?
    pub(crate) fn has_unique_public_key_hash_operations(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<bool, Error> {
        let unique_key_hashes = unique_key_hashes_tree_path();
        self.grove_has_raw(
            (&unique_key_hashes).into(),
            public_key_hash.as_slice(),
            StatefulDirectQuery,
            transaction,
            drive_operations,
        )
    }

    /// Do any keys with given public key hashes already exist in the unique tree?
    /// Will return public key hashes that already exist
    pub fn has_any_of_unique_public_key_hashes(
        &self,
        public_key_hashes: Vec<[u8; 20]>,
        transaction: TransactionArg,
    ) -> Result<Vec<[u8; 20]>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.has_any_of_unique_public_key_hashes_operations(
            public_key_hashes,
            transaction,
            &mut drive_operations,
        )
    }

    /// Operations for if any keys with given public key hashes already exist in the unique tree.
    /// Will return public key hashes that already exist
    pub(crate) fn has_any_of_unique_public_key_hashes_operations(
        &self,
        public_key_hashes: Vec<[u8; 20]>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
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

    /// Does a key with that public key hash already exist in the non unique set?
    pub fn has_non_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
    ) -> Result<bool, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.has_unique_public_key_hash_operations(
            public_key_hash,
            transaction,
            &mut drive_operations,
        )
    }

    /// Operations for if a key with that public key hash already exists in the non unique set?
    pub(crate) fn has_non_unique_public_key_hash_operations(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<bool, Error> {
        let non_unique_key_hashes = non_unique_key_hashes_tree_path();
        // this will actually get a tree
        self.grove_has_raw(
            (&non_unique_key_hashes).into(),
            public_key_hash.as_slice(),
            StatefulDirectQuery,
            transaction,
            drive_operations,
        )
    }

    /// Operations for if a key with that public key hash already exists in the non unique set?
    /// For a particular identity
    pub(crate) fn has_non_unique_public_key_hash_already_for_identity_operations(
        &self,
        public_key_hash: [u8; 20],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<bool, Error> {
        let public_key_hash_sub_tree =
            non_unique_key_hashes_sub_tree_path(public_key_hash.as_slice());
        // this will actually get a tree
        self.grove_has_raw(
            (&public_key_hash_sub_tree).into(),
            identity_id.as_slice(),
            StatefulDirectQuery,
            transaction,
            drive_operations,
        )
    }

    /// Fetches an identity with all its information from storage.
    pub fn fetch_serialized_full_identity_by_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        encoding: QueryResultEncoding,
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let identity =
            self.fetch_full_identity_by_unique_public_key_hash(public_key_hash, transaction)?;

        let identity_value = match identity {
            None => Value::Null,
            Some(identity) => identity.to_cleaned_object()?,
        };
        encoding.encode_value(&identity_value)
    }

    /// Fetches an identity with all its information from storage.
    pub fn fetch_full_identity_by_unique_public_key_hash(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
    ) -> Result<Option<Identity>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_full_identity_by_unique_public_key_hash_operations(
            public_key_hash,
            transaction,
            &mut drive_operations,
        )
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(crate) fn fetch_full_identity_by_unique_public_key_hash_operations(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Option<Identity>, Error> {
        let identity_id = self.fetch_identity_id_by_unique_public_key_hash_operations(
            public_key_hash,
            transaction,
            drive_operations,
        )?;
        if let Some(identity_id) = identity_id {
            self.fetch_full_identity(identity_id, transaction)
        } else {
            Ok(None)
        }
    }

    /// Fetches identities with all its information from storage.
    pub fn fetch_full_identities_by_unique_public_key_hashes(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
    ) -> Result<BTreeMap<[u8; 20], Option<Identity>>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_full_identities_by_unique_public_key_hashes_operations(
            public_key_hashes,
            transaction,
            &mut drive_operations,
        )
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(crate) fn fetch_full_identities_by_unique_public_key_hashes_operations(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<BTreeMap<[u8; 20], Option<Identity>>, Error> {
        let identity_ids = self.fetch_identity_ids_by_unique_public_key_hashes_operations(
            public_key_hashes,
            transaction,
            drive_operations,
        )?;
        identity_ids
            .into_iter()
            .map(|(public_key_hash, maybe_identity_id)| {
                let identity = maybe_identity_id
                    .map(|identity_id| self.fetch_full_identity(identity_id, transaction))
                    .transpose()?
                    .flatten();
                Ok((public_key_hash, identity))
            })
            .collect::<Result<BTreeMap<[u8; 20], Option<Identity>>, Error>>()
    }
}

#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use crate::tests::helpers::setup::setup_drive;
    use dpp::block::block_info::BlockInfo;

    use super::*;

    #[test]
    fn test_fetch_all_keys_on_identity() {
        let drive = setup_drive(None);

        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction))
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345));

        drive
            .add_new_identity(
                identity.clone(),
                &BlockInfo::default(),
                true,
                Some(&transaction),
            )
            .expect("expected to insert identity");

        let public_keys = drive
            .fetch_all_identity_keys(identity.id.to_buffer(), Some(&transaction))
            .expect("expected to fetch keys");

        assert_eq!(public_keys.len(), 5);

        for (_, key) in public_keys {
            let hash = key
                .hash()
                .expect("expected to get hash")
                .try_into()
                .expect("expected 20 bytes");
            if key.key_type.is_unique_key_type() {
                let identity_id = drive
                    .fetch_identity_id_by_unique_public_key_hash(hash, Some(&transaction))
                    .expect("expected to fetch identity_id")
                    .expect("expected to get an identity id");
                assert_eq!(identity_id, identity.id.to_buffer());
            } else {
                let identity_ids = drive
                    .fetch_identity_ids_by_non_unique_public_key_hash(hash, Some(&transaction))
                    .expect("expected to get identity ids");
                assert!(identity_ids.contains(&identity.id.to_buffer()));
            }
        }
    }
}
