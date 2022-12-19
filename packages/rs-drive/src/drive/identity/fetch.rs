use crate::common::encode::{encode_u16, encode_u32};
use crate::drive::block_info::BlockInfo;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::DirectQueryType;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::identity::{
    balance_path, balance_path_vec, identity_path, identity_query_keys_tree_path_vec, IDENTITY_KEY,
};
use crate::drive::object_size_info::KeyValueInfo::KeyRefRequest;
use crate::drive::{identity_tree_path, Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::fee::{calculate_fee, FeeResult};
use crate::query::{Query, QueryItem};
use dpp::identifier::Identifier;
use dpp::identity::{Identity, KeyType, Purpose, SecurityLevel};
use grovedb::query_result_type::QueryResultType::QueryElementResultType;
use grovedb::Element::{Item, SumItem};
use grovedb::{Element, PathQuery, SizedQuery, TransactionArg};
use integer_encoding::{VarInt, VarIntReader};
use std::collections::{BTreeMap, BTreeSet};

/// The type of key request
/// You can either get the current keys
/// or you can get all keys
pub enum KeyRequestType {
    CurrentKeyRequest,
    AllKeysRequest,
}

/// A request to get Keys from an Identity
pub struct IdentityKeysRequest {
    identity_id: [u8; 32],
    key_requests: BTreeMap<KeyType, BTreeMap<Purpose, BTreeMap<SecurityLevel, KeyRequestType>>>,
}

impl IdentityKeysRequest {
    fn to_path_query(self) -> PathQuery {
        let IdentityKeysRequest {
            identity_id,
            key_requests,
        } = self;
        let query_keys_path = identity_query_keys_tree_path_vec(identity_id);
        PathQuery {
            path: query_keys_path,
            query: SizedQuery {
                query: Self::construct_query(key_requests),
                limit: None,
                offset: None,
            },
        }
    }

    fn construct_query(
        key_requests: BTreeMap<KeyType, BTreeMap<Purpose, BTreeMap<SecurityLevel, KeyRequestType>>>,
    ) -> Query {
        let mut query = Query::new();

        for (key_type, leftover_query) in key_requests {
            let key = vec![key_type as u8];
            if !leftover_query.is_empty() {
                query.add_conditional_subquery(
                    QueryItem::Key(key),
                    None,
                    Some(Self::construct_purpose_query(leftover_query)),
                );
            }
        }
        query
    }

    fn construct_purpose_query(
        key_requests: BTreeMap<Purpose, BTreeMap<SecurityLevel, KeyRequestType>>,
    ) -> Query {
        let mut query = Query::new();

        for (purpose, leftover_query) in key_requests {
            let key = vec![purpose as u8];
            if !leftover_query.is_empty() {
                query.add_conditional_subquery(
                    QueryItem::Key(key),
                    None,
                    Some(Self::construct_security_level_query(leftover_query)),
                );
            }
        }
        query
    }

    fn construct_security_level_query(
        key_requests: BTreeMap<SecurityLevel, KeyRequestType>,
    ) -> Query {
        let mut query = Query::new();

        for (security_level, key_request_type) in key_requests {
            let key = vec![security_level as u8];
            let subquery = match key_request_type {
                KeyRequestType::CurrentKeyRequest => {
                    let mut subquery = Query::new();
                    subquery.insert_key(vec![]);
                    subquery
                }
                KeyRequestType::AllKeysRequest => {
                    let mut subquery = Query::new();
                    subquery.insert_range_after(vec![]..);
                    subquery
                }
            };
            query.add_conditional_subquery(QueryItem::Key(key), None, Some(subquery));
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
                if let SumItem(identity_balance_element, element_flags) = identity_balance_element {
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
        let identity_path = identity_tree_path();
        let identity_revision_element = self.grove_get_direct(
            identity_path,
            identity_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
        )?;
        if apply {
            if let Some(identity_revision_element) = identity_revision_element {
                if let Item(identity_revision_element, _) = identity_revision_element {
                    let (revision, _) = u64::decode_var(identity_revision_element.as_slice())
                        .ok_or(Err(Error::Drive(DriveError::CorruptedElementType(
                            "identity revision could not be decoded",
                        ))))?;
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

    /// Given an identity, fetches the identity with its flags from storage.
    pub fn fetch_full_identity(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<(Option<Identity>), Error> {
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

        let loaded_public_keys = self
            .fetch_all_identity_keys(identity_id, true, transaction)?
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "revision not found on identity".to_string(),
            )))?;
        Ok(Some(Identity {
            protocol_version: 0,
            id: Identifier::new(identity_id),
            loaded_public_keys,
            balance,
            revision,
            asset_lock_proof: None,
            metadata: None,
        }))
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
