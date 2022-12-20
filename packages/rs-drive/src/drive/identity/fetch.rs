use crate::drive::block_info::BlockInfo;
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::DirectQueryType;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::identity::fetch::KeyKindRequestType::{
    AllKeysOfKindRequest, CurrentKeyOfKindRequest,
};
use crate::drive::identity::fetch::KeyRequestType::{
    AllKeysRequest, SearchKeyRequest, SpecificKeyRequest,
};
use crate::drive::identity::IdentityRootStructure::IdentityTreeRevision;
use crate::drive::identity::{
    balance_path, identity_key_tree_path_vec, identity_path, identity_query_keys_tree_path_vec,
    IDENTITY_KEY,
};

use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::fee::{calculate_fee, FeeResult};
use crate::query::{Query, QueryItem};
use dpp::identifier::Identifier;
use dpp::identity::{Identity, KeyID, Purpose, SecurityLevel};
use dpp::prelude::IdentityPublicKey;
use grovedb::query_result_type::QueryResultType::{
    QueryElementResultType, QueryKeyElementPairResultType,
};
use grovedb::Element::{Item, SumItem};
use grovedb::{Element, PathQuery, SizedQuery, TransactionArg};
use integer_encoding::VarInt;
use std::collections::BTreeMap;

/// The kind of keys you are requesting
/// A kind is a purpose/security level pair
/// Do you want to get all keys in that pair
/// Or just the current one?
#[derive(Clone, Copy)]
pub enum KeyKindRequestType {
    CurrentKeyOfKindRequest,
    AllKeysOfKindRequest,
}

/// The type of key request
#[derive(Clone)]
pub enum KeyRequestType {
    AllKeysRequest,
    SpecificKeyRequest(KeyID),
    SearchKeyRequest(BTreeMap<PurposeU8, BTreeMap<SecurityLevelU8, KeyKindRequestType>>),
}

type PurposeU8 = u8;
type SecurityLevelU8 = u8;

/// A request to get Keys from an Identity
pub struct IdentityKeysRequest {
    identity_id: [u8; 32],
    key_request: KeyRequestType,
    limit: Option<u16>,
    offset: Option<u16>,
}

impl IdentityKeysRequest {
    /// Make a request for all current keys for the identity
    pub fn new_all_current_keys_query(identity_id: [u8; 32]) -> Self {
        let mut sec_btree_map = BTreeMap::new();
        for security_level in 0..=SecurityLevel::last() as u8 {
            sec_btree_map.insert(security_level, CurrentKeyOfKindRequest);
        }
        let mut purpose_btree_map = BTreeMap::new();
        for purpose in 0..=Purpose::last() as u8 {
            purpose_btree_map.insert(purpose, sec_btree_map.clone());
        }
        IdentityKeysRequest {
            identity_id,
            key_request: SearchKeyRequest(purpose_btree_map),
            limit: None,
            offset: None,
        }
    }

    /// Make a request for all current keys for the identity
    pub fn new_all_keys_query(identity_id: [u8; 32]) -> Self {
        IdentityKeysRequest {
            identity_id,
            key_request: AllKeysRequest,
            limit: None,
            offset: None,
        }
    }

    /// Create the path query for the request
    pub fn to_path_query(self) -> PathQuery {
        let IdentityKeysRequest {
            identity_id,
            key_request,
            limit,
            offset,
        } = self;
        match key_request {
            AllKeysRequest => {
                let query_keys_path = identity_key_tree_path_vec(identity_id.as_slice());
                PathQuery {
                    path: query_keys_path,
                    query: SizedQuery {
                        query: Self::all_keys_query(),
                        limit,
                        offset,
                    },
                }
            }
            SpecificKeyRequest(key_id) => {
                let query_keys_path = identity_key_tree_path_vec(identity_id.as_slice());
                PathQuery {
                    path: query_keys_path,
                    query: SizedQuery {
                        query: Self::specific_key_query(key_id),
                        limit: Some(1),
                        offset: None,
                    },
                }
            }
            SearchKeyRequest(map) => {
                let query_keys_path = identity_query_keys_tree_path_vec(identity_id);
                PathQuery {
                    path: query_keys_path,
                    query: SizedQuery {
                        query: Self::construct_search_query(map),
                        limit,
                        offset,
                    },
                }
            }
        }
    }

    /// All keys
    fn all_keys_query() -> Query {
        let mut query = Query::new();
        query.insert_all();
        query
    }

    /// Fetch a specific key knowing the id
    fn specific_key_query(key_id: KeyID) -> Query {
        let mut query = Query::new();
        query.insert_key(key_id.encode_var_vec());
        query
    }

    /// Contruct the query for the request
    fn construct_search_query(
        key_requests: BTreeMap<PurposeU8, BTreeMap<SecurityLevelU8, KeyKindRequestType>>,
    ) -> Query {
        fn construct_security_level_query(
            key_requests: BTreeMap<SecurityLevelU8, KeyKindRequestType>,
        ) -> Query {
            let mut query = Query::new();

            for (security_level, key_request_type) in key_requests {
                let key = vec![security_level];
                let subquery = match key_request_type {
                    CurrentKeyOfKindRequest => {
                        let mut subquery = Query::new();
                        subquery.insert_key(vec![]);
                        subquery
                    }
                    AllKeysOfKindRequest => {
                        let mut subquery = Query::new();
                        subquery.insert_range_after(vec![]..);
                        subquery
                    }
                };
                query.add_conditional_subquery(QueryItem::Key(key), None, Some(subquery));
            }
            query
        }
        let mut query = Query::new();

        for (purpose, leftover_query) in key_requests {
            let key = vec![purpose];
            if !leftover_query.is_empty() {
                query.add_conditional_subquery(
                    QueryItem::Key(key),
                    None,
                    Some(construct_security_level_query(leftover_query)),
                );
            }
        }
        query
    }
}

impl Drive {
    /// Fetches the Identity's balance from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_balance(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<Option<u64>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_identity_balance_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )
    }

    /// Fetches the Identity's balance from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_balance_with_fees(
        &self,
        identity_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<(Option<u64>, FeeResult), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let value = self.fetch_identity_balance_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok((value, fees))
    }

    /// Creates the operations to get Identity's balance from the backing store
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn fetch_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<u64>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            // 8 is the size of a i64 used in sum trees
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: true,
                query_target: QueryTargetValue(8),
            }
        };
        let balance_path = balance_path();
        let identity_balance_element = self.grove_get_direct(
            balance_path,
            identity_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
        )?;
        if apply {
            if let Some(identity_balance_element) = identity_balance_element {
                if let SumItem(identity_balance_element, _element_flags) = identity_balance_element
                {
                    if identity_balance_element < 0 {
                        Err(Error::Drive(DriveError::CorruptedElementType(
                            "identity balance was present but was negative",
                        )))
                    } else {
                        Ok(Some(identity_balance_element as u64))
                    }
                } else {
                    Err(Error::Drive(DriveError::CorruptedElementType(
                        "identity balance was present but was not identified as a sum item",
                    )))
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Fetches the Identity's revision from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_revision(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<Option<u64>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_identity_revision_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )
    }

    /// Fetches the Identity's revision from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_revision_with_fees(
        &self,
        identity_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<(Option<u64>, FeeResult), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let value = self.fetch_identity_revision_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok((value, fees))
    }

    /// Creates the operations to get Identity's revision from the backing store
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn fetch_identity_revision_operations(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<u64>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: false,
                query_target: QueryTargetValue(1),
            }
        };
        let identity_path = identity_path(identity_id.as_slice());
        let identity_revision_element = self.grove_get_direct(
            identity_path,
            &[IdentityTreeRevision as u8],
            direct_query_type,
            transaction,
            drive_operations,
        )?;
        if apply {
            if let Some(identity_revision_element) = identity_revision_element {
                if let Item(identity_revision_element, _) = identity_revision_element {
                    let (revision, _) = u64::decode_var(identity_revision_element.as_slice())
                        .ok_or(Error::Drive(DriveError::CorruptedElementType(
                            "identity revision could not be decoded",
                        )))?;
                    Ok(Some(revision))
                } else {
                    Err(Error::Drive(DriveError::CorruptedElementType(
                        "identity revision was present but was not identified as an item",
                    )))
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Fetch all the current keys of every kind for a specific Identity
    pub fn fetch_all_current_identity_keys(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_all_current_identity_keys_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )
    }

    /// Operations for fetching all the current keys of every kind for a specific Identity
    pub(crate) fn fetch_all_current_identity_keys_operations(
        &self,
        identity_id: [u8; 32],
        _apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let key_request = IdentityKeysRequest::new_all_current_keys_query(identity_id);
        let path_query = key_request.to_path_query();

        let (serialized_keys, _) =
            self.grove_get_path_query(&path_query, transaction, drive_operations)?;
        serialized_keys
            .into_iter()
            .map(|serialized_key| {
                let key = IdentityPublicKey::deserialize(serialized_key.as_slice())?;
                Ok((key.id, key))
            })
            .collect()
    }

    /// Fetch all the keys of every kind for a specific Identity
    pub fn fetch_all_identity_keys(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_all_identity_keys_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )
    }

    /// Operations for fetching all the keys of every kind for a specific Identity
    pub(crate) fn fetch_all_identity_keys_operations(
        &self,
        identity_id: [u8; 32],
        _apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let key_request = IdentityKeysRequest::new_all_keys_query(identity_id);
        let path_query = key_request.to_path_query();

        let (serialized_keys, _) =
            self.grove_get_path_query(&path_query, transaction, drive_operations)?;
        serialized_keys
            .into_iter()
            .map(|serialized_key| {
                let key = IdentityPublicKey::deserialize(serialized_key.as_slice())?;
                Ok((key.id, key))
            })
            .collect()
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub fn fetch_full_identity(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Option<Identity>, Error> {
        // let's start by getting the balance
        let balance = self.fetch_identity_balance(identity_id, true, transaction)?;
        if balance.is_none() {
            return Ok(None);
        }
        let balance = balance.unwrap();
        let revision = self
            .fetch_identity_revision(identity_id, true, transaction)?
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "revision not found on identity".to_string(),
            )))?;

        let public_keys = self.fetch_all_identity_keys(identity_id, true, transaction)?;
        Ok(Some(Identity {
            protocol_version: PROTOCOL_VERSION,
            id: Identifier::new(identity_id),
            public_keys,
            balance,
            revision,
            asset_lock_proof: None,
            metadata: None,
        }))
    }

    /// Given a vector of identities, fetches the identities from storage.
    pub fn verify_all_identities_exist(
        &self,
        ids: &Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<bool, Error> {
        let mut query = Query::new();
        for id in ids {
            query.insert_item(QueryItem::Key(id.to_vec()));
        }
        let path_query = PathQuery {
            path: vec![vec![RootTree::Identities as u8]],
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        };
        let (result_items, _) = self
            .grove
            .query_raw(&path_query, QueryElementResultType, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

        Ok(result_items.len() == ids.len())
    }

    /// Given a vector of identities, fetches the identities from storage.
    pub fn fetch_identities_balances(
        &self,
        ids: &Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<BTreeMap<[u8; 32], u64>, Error> {
        let mut query = Query::new();
        for id in ids {
            query.insert_item(QueryItem::Key(id.to_vec()));
        }
        let path_query = PathQuery {
            path: vec![vec![RootTree::Balances as u8]],
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        };
        let (result_items, _) = self
            .grove
            .query_raw(&path_query, QueryKeyElementPairResultType, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

        result_items
            .to_key_elements()
            .into_iter()
            .map(|key_element| {
                if let SumItem(balance, _) = &key_element.1 {
                    let identifier: [u8; 32] = key_element.0.try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization("expected 32 bytes"))
                    })?;
                    Ok((identifier, *balance as u64))
                } else {
                    Err(Error::Drive(DriveError::CorruptedIdentityNotItem(
                        "identity balance must be a sum item",
                    )))
                }
            })
            .collect()
    }

    /// Given a vector of identities, fetches the identities from storage.
    pub fn fetch_identities(
        &self,
        ids: &Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<Vec<Identity>, Error> {
        Ok(self
            .fetch_identities_with_flags(ids, transaction)?
            .into_iter()
            .map(|(identity, _)| identity)
            .collect())
    }

    /// Given a vector of identities, fetches the identities with their flags from storage.
    pub fn fetch_identities_with_flags(
        &self,
        ids: &Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<Vec<(Identity, Option<StorageFlags>)>, Error> {
        let mut query = Query::new();
        query.set_subquery_key(IDENTITY_KEY.to_vec());
        for id in ids {
            query.insert_item(QueryItem::Key(id.to_vec()));
        }
        let path_query = PathQuery {
            path: vec![vec![RootTree::Identities as u8]],
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        };
        let (result_items, _) = self
            .grove
            .query_raw(&path_query, QueryElementResultType, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

        result_items
            .to_elements()
            .into_iter()
            .map(|element| {
                if let Element::Item(identity_cbor, element_flags) = &element {
                    let identity =
                        Identity::from_buffer(identity_cbor.as_slice()).map_err(|_| {
                            Error::Identity(IdentityError::IdentitySerialization(
                                "failed to deserialize an identity",
                            ))
                        })?;

                    Ok((
                        identity,
                        StorageFlags::from_some_element_flags_ref(element_flags)?,
                    ))
                } else {
                    Err(Error::Drive(DriveError::CorruptedIdentityNotItem(
                        "identity must be an item",
                    )))
                }
            })
            .collect()
    }
}
