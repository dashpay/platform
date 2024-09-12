use crate::drive::balances::balance_path_vec;
use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::{identity_tree_path_vec, unique_key_hashes_tree_path_vec, Drive};
use std::ops::RangeFull;

use crate::error::Error;

use crate::drive::identity::contract_info::ContractInfoStructure;
use crate::drive::identity::contract_info::ContractInfoStructure::IdentityContractNonceKey;
use crate::drive::identity::IdentityRootStructure::{IdentityTreeNonce, IdentityTreeRevision};
use crate::drive::identity::{
    identity_contract_info_group_path_vec, identity_path_vec, IdentityRootStructure,
};
use crate::error::query::QuerySyntaxError;
use dpp::identity::Purpose;
use grovedb::query_result_type::Key;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery};
use grovedb_version::version::GroveVersion;

/// An enumeration representing the types of identity prove requests.
///
/// # Variants
///
/// * `FullIdentity`: Represents a request to prove the full identity (0).
/// * `Balance`: Represents a request to prove the account balance (1).
/// * `Keys`: Represents a request to prove the public keys (2).
#[repr(u8)]
pub enum IdentityProveRequestType {
    /// FullIdentity: A variant representing full identity access, assigned the value 0.
    FullIdentity = 0,
    /// Balance: A variant representing balance access only, assigned the value 1.
    Balance = 1,
    /// Keys: A variant representing keys access only, assigned the value 2.
    Keys = 2,
    /// Revision: A variant representing revision field
    Revision = 3,
}

impl TryFrom<u8> for IdentityProveRequestType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(IdentityProveRequestType::FullIdentity),
            1 => Ok(IdentityProveRequestType::Balance),
            2 => Ok(IdentityProveRequestType::Keys),
            3 => Ok(IdentityProveRequestType::Revision),
            _ => Err(Error::Query(QuerySyntaxError::InvalidIdentityProveRequest(
                "unknown prove request type",
            ))),
        }
    }
}

/// A struct used for querying identity drives.
///
/// # Fields
///
/// * `identity_id`: An array of 32 bytes representing the unique identity ID.
/// * `prove_request_type`: The type of identity proof requested, based on the `IdentityProveRequestType` enum.
pub struct IdentityDriveQuery {
    /// A 32-byte array representing the unique identifier for an identity.
    pub identity_id: [u8; 32],
    /// An instance of the `IdentityProveRequestType` enum that specifies
    /// the type of prove request being made for the identity.
    pub prove_request_type: IdentityProveRequestType,
}

impl Drive {
    /// The path query for the revision of an identity
    pub fn revision_for_identity_id_path_query(identity_id: [u8; 32]) -> PathQuery {
        let revision_path = identity_path_vec(&identity_id);
        PathQuery::new_single_key(revision_path, vec![IdentityTreeRevision as u8])
    }

    /// The path query for the revision and the balance of an identity
    pub fn revision_and_balance_path_query(
        identity_id: [u8; 32],
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let revision_query = Self::revision_for_identity_id_path_query(identity_id);
        let balance_query = Self::balance_for_identity_id_query(identity_id);
        PathQuery::merge(vec![&revision_query, &balance_query], grove_version)
            .map_err(Error::GroveDB)
    }

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
                .iter()
                .map(|key_hash| key_hash.to_vec())
                .collect(),
        );
        PathQuery::new_unsized(unique_key_hashes, query)
    }

    /// The query getting all keys and balance and revision
    pub fn full_identity_query(
        identity_id: &[u8; 32],
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let balance_query = Self::identity_balance_query(identity_id);
        let revision_query = Self::identity_revision_query(identity_id);
        let key_request = IdentityKeysRequest::new_all_keys_query(identity_id, None);
        let all_keys_query = key_request.into_path_query();
        PathQuery::merge(
            vec![&balance_query, &revision_query, &all_keys_query],
            grove_version,
        )
        .map_err(Error::GroveDB)
    }

    /// The query getting all keys and revision
    pub fn identity_all_keys_query(
        identity_id: &[u8; 32],
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let revision_query = Self::identity_revision_query(identity_id);
        let key_request = IdentityKeysRequest::new_all_keys_query(identity_id, None);
        let all_keys_query = key_request.into_path_query();
        PathQuery::merge(vec![&revision_query, &all_keys_query], grove_version)
            .map_err(Error::GroveDB)
    }

    /// The query getting all balances and revision
    pub fn balances_for_identity_ids_query(identity_ids: &[[u8; 32]]) -> PathQuery {
        let balance_path = balance_path_vec();
        let mut query = Query::new();
        query.insert_keys(identity_ids.iter().map(|key| key.to_vec()).collect());
        PathQuery {
            path: balance_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// The query getting all balances and revision
    pub fn balances_for_range_query(
        start_at: Option<([u8; 32], bool)>,
        ascending: bool,
        limit: u16,
    ) -> PathQuery {
        let balance_path = balance_path_vec();
        let mut query = Query::new_with_direction(ascending);
        if ascending {
            if let Some((start_at, start_at_included)) = start_at {
                if start_at_included {
                    query.insert_item(QueryItem::RangeFrom(start_at.to_vec()..))
                } else {
                    query.insert_item(QueryItem::RangeAfter(start_at.to_vec()..))
                }
            } else {
                query.insert_item(QueryItem::RangeFull(RangeFull))
            }
        } else if let Some((start_at, start_at_included)) = start_at {
            if start_at_included {
                query.insert_item(QueryItem::RangeToInclusive(..=start_at.to_vec()))
            } else {
                query.insert_item(QueryItem::RangeTo(..start_at.to_vec()))
            }
        } else {
            query.insert_item(QueryItem::RangeFull(RangeFull))
        }
        PathQuery {
            path: balance_path,
            query: SizedQuery {
                query,
                limit: Some(limit),
                offset: None,
            },
        }
    }

    /// The query getting all keys and balance and revision
    pub fn full_identities_query(
        identity_ids: &[[u8; 32]],
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let path_queries: Vec<PathQuery> = identity_ids
            .iter()
            .map(|identity_id| Self::full_identity_query(identity_id, grove_version))
            .collect::<Result<Vec<PathQuery>, Error>>()?;
        PathQuery::merge(path_queries.iter().collect(), grove_version).map_err(Error::GroveDB)
    }

    /// This query gets the full identity and the public key hash
    pub fn full_identity_with_public_key_hash_query(
        public_key_hash: [u8; 20],
        identity_id: [u8; 32],
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let full_identity_query = Self::full_identity_query(&identity_id, grove_version)?;
        let identity_id_by_public_key_hash_query =
            Self::identity_id_by_unique_public_key_hash_query(public_key_hash);
        PathQuery::merge(
            vec![&full_identity_query, &identity_id_by_public_key_hash_query],
            grove_version,
        )
        .map_err(Error::GroveDB)
    }

    /// The query full identities with key hashes too
    pub fn full_identities_with_keys_hashes_query(
        identity_ids: &[[u8; 32]],
        key_hashes: &[[u8; 20]],
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let identities_path_query = Self::full_identities_query(identity_ids, grove_version)?;
        let key_hashes_to_identity_ids_query =
            Self::identity_ids_by_unique_public_key_hash_query(key_hashes);

        PathQuery::merge(
            vec![&identities_path_query, &key_hashes_to_identity_ids_query],
            grove_version,
        )
        .map_err(Error::GroveDB)
    }

    /// The query for the identity balance
    pub fn identity_balance_query(identity_id: &[u8; 32]) -> PathQuery {
        let balance_path = balance_path_vec();
        let mut query = Query::new();
        query.insert_key(identity_id.to_vec());
        PathQuery {
            path: balance_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// The query for the identity contract bounded keys for multiple identities
    pub fn identities_contract_keys_query(
        identity_ids: &[[u8; 32]],
        contract_id: &[u8; 32],
        document_type_name: &Option<String>,
        purposes: &[Purpose],
        limit: Option<u16>,
    ) -> PathQuery {
        let identities_path = identity_tree_path_vec();
        let mut query = Query::new();
        query.insert_keys(
            identity_ids
                .iter()
                .map(|identity_id| identity_id.to_vec())
                .collect(),
        );

        let mut group_id = contract_id.to_vec();
        if let Some(document_type_name) = document_type_name {
            group_id.extend(document_type_name.as_bytes());
        }

        query.default_subquery_branch.subquery_path = Some(vec![
            vec![IdentityRootStructure::IdentityContractInfo as u8],
            group_id,
            vec![ContractInfoStructure::ContractInfoKeysKey as u8],
        ]);

        let mut sub_query = Query::new();

        sub_query.insert_keys(
            purposes
                .iter()
                .map(|purpose| vec![*purpose as u8])
                .collect(),
        );

        sub_query.set_subquery_key(Key::new());

        query.default_subquery_branch.subquery = Some(sub_query.into());
        PathQuery {
            path: identities_path,
            query: SizedQuery {
                query,
                limit,
                offset: None,
            },
        }
    }

    /// The query for the identity contract document type bounded keys for multiple identities
    pub fn identities_contract_document_type_keys_query(
        identity_ids: &[[u8; 32]],
        contract_id: [u8; 32],
        document_type_name: &str,
        purposes: Vec<Purpose>,
    ) -> PathQuery {
        let identities_path = identity_tree_path_vec();
        let mut query = Query::new();
        query.insert_keys(
            identity_ids
                .iter()
                .map(|identity_id| identity_id.to_vec())
                .collect(),
        );
        let mut group_id = contract_id.to_vec();
        group_id.extend(document_type_name.as_bytes());
        query.default_subquery_branch.subquery_path = Some(vec![
            vec![IdentityRootStructure::IdentityContractInfo as u8],
            contract_id.to_vec(),
            vec![ContractInfoStructure::ContractInfoKeysKey as u8],
        ]);

        let mut sub_query = Query::new();

        sub_query.insert_keys(
            purposes
                .into_iter()
                .map(|purpose| vec![purpose as u8])
                .collect(),
        );

        query.default_subquery_branch.subquery = Some(sub_query.into());
        PathQuery {
            path: identities_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// The query for proving the identities balance from an identity id.
    pub fn balance_for_identity_id_query(identity_id: [u8; 32]) -> PathQuery {
        let balance_path = balance_path_vec();
        PathQuery::new_single_key(balance_path, identity_id.to_vec())
    }

    /// The query for proving an identity's nonce.
    pub fn identity_nonce_query(identity_id: [u8; 32]) -> PathQuery {
        let identity_path = identity_path_vec(identity_id.as_slice());
        PathQuery::new_single_key(identity_path, vec![IdentityTreeNonce as u8])
    }

    /// The query for proving the identities nonce for a specific contract.
    pub fn identity_contract_nonce_query(
        identity_id: [u8; 32],
        contract_id: [u8; 32],
    ) -> PathQuery {
        let identity_contract_path =
            identity_contract_info_group_path_vec(&identity_id, contract_id.as_slice());
        PathQuery::new_single_key(identity_contract_path, vec![IdentityContractNonceKey as u8])
    }

    /// The query for proving the identities balance and revision from an identity id.
    pub fn balance_and_revision_for_identity_id_query(
        identity_id: [u8; 32],
        grove_version: &GroveVersion,
    ) -> PathQuery {
        let balance_path_query = Self::balance_for_identity_id_query(identity_id);
        let revision_path_query = Self::identity_revision_query(&identity_id);
        //todo: lazy static this
        PathQuery::merge(
            vec![&balance_path_query, &revision_path_query],
            grove_version,
        )
        .unwrap()
    }
}
