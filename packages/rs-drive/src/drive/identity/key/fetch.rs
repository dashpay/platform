use crate::drive::block_info::BlockInfo;
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::DirectQueryType;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
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
use crate::drive::identity::key::fetch::KeyKindRequestType::CurrentKeyOfKindRequest;
use crate::drive::identity::key::fetch::KeyRequestType::{AllKeysRequest, SearchKeyRequest, SpecificKeysRequest};

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
    SpecificKeysRequest(Vec<KeyID>),
    SearchKeyRequest(BTreeMap<PurposeU8, BTreeMap<SecurityLevelU8, KeyKindRequestType>>),
}

type PurposeU8 = u8;
type SecurityLevelU8 = u8;

/// A request to get Keys from an Identity
pub struct IdentityKeysRequest {
    /// The request identity id
    pub identity_id: [u8; 32],
    /// The type of key request
    pub key_request: KeyRequestType,
    /// The limit of the amount of keys you wish to get back
    pub limit: Option<u16>,
    /// The offset of the start of the amount of keys you wish to get back
    pub offset: Option<u16>,
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
            SpecificKeysRequest(key_ids) => {
                let query_keys_path = identity_key_tree_path_vec(identity_id.as_slice());
                PathQuery {
                    path: query_keys_path,
                    query: SizedQuery {
                        query: Self::specific_keys_query(key_ids),
                        limit: None,
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
    fn specific_keys_query(key_ids: Vec<KeyID>) -> Query {
        let mut query = Query::new();
        for key_id in key_ids {
            query.insert_key(key_id.encode_var_vec());
        }
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
    /// Fetch all the current keys of every kind for a specific Identity
    pub fn fetch_all_current_identity_keys(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_all_current_identity_keys_operations(
            identity_id,
            transaction,
            &mut drive_operations,
        )
    }

    /// Operations for fetching all the current keys of every kind for a specific Identity
    pub(crate) fn fetch_all_current_identity_keys_operations(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let key_request = IdentityKeysRequest::new_all_current_keys_query(identity_id);
        self.fetch_identity_keys_operations(key_request, transaction, drive_operations)
    }

    /// Fetch all the keys of every kind for a specific Identity
    pub fn fetch_all_identity_keys(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_all_identity_keys_operations(
            identity_id,
            transaction,
            &mut drive_operations,
        )
    }

    /// Operations for fetching all the keys of every kind for a specific Identity
    pub(crate) fn fetch_all_identity_keys_operations(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let key_request = IdentityKeysRequest::new_all_keys_query(identity_id);
        self.fetch_identity_keys_operations(key_request, transaction, drive_operations)
    }

    /// Fetch keys matching the request for a specific Identity
    pub fn fetch_identity_keys(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_identity_keys_operations(
            key_request,
            transaction,
            &mut drive_operations,
        )
    }

    /// Operations for fetching keys matching the request for a specific Identity
    pub(crate) fn fetch_identity_keys_operations(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
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
}
