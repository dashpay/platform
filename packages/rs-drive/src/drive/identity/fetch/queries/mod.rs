use crate::drive::balances::balance_path_vec;
use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::non_unique_key_hashes_tree_path_vec;
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
        PathQuery::merge(vec![&revision_query, &balance_query], grove_version).map_err(Error::from)
    }

    /// The query for proving an identity id from a public key hash.
    pub fn identity_id_by_unique_public_key_hash_query(public_key_hash: [u8; 20]) -> PathQuery {
        let unique_key_hashes = unique_key_hashes_tree_path_vec();
        PathQuery::new_single_key(unique_key_hashes, public_key_hash.to_vec())
    }

    /// The query for proving an identity id from a non-unique public key hash.
    /// This should be used for absence proofs
    pub fn identity_id_by_non_unique_public_key_hash_query(
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
    ) -> PathQuery {
        let non_unique_key_hashes = non_unique_key_hashes_tree_path_vec();
        let mut query = Query::new_single_key(public_key_hash.to_vec());
        let sub_query = if let Some(after) = after {
            Query::new_single_query_item(QueryItem::RangeAfter(after.to_vec()..))
        } else {
            // We do range full because this sub query can get multiple identities
            // as they are non unique.
            Query::new_range_full()
        };
        query.set_subquery(sub_query);
        PathQuery::new(non_unique_key_hashes, SizedQuery::new(query, None, None))
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
        .map_err(Error::from)
    }

    /// The query getting all keys and revision
    pub fn identity_all_keys_query(
        identity_id: &[u8; 32],
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let revision_query = Self::identity_revision_query(identity_id);
        let key_request = IdentityKeysRequest::new_all_keys_query(identity_id, None);
        let all_keys_query = key_request.into_path_query();
        PathQuery::merge(vec![&revision_query, &all_keys_query], grove_version).map_err(Error::from)
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
        PathQuery::merge(path_queries.iter().collect(), grove_version).map_err(Error::from)
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
        .map_err(Error::from)
    }

    /// This query gets the full identity and the public key hash
    pub fn full_identity_with_non_unique_public_key_hash_query(
        public_key_hash: [u8; 20],
        identity_id: [u8; 32],
        after: Option<[u8; 32]>,
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let full_identity_query = Self::full_identity_query(&identity_id, grove_version)?;
        let identity_id_by_public_key_hash_query =
            Self::identity_id_by_non_unique_public_key_hash_query(public_key_hash, after);
        PathQuery::merge(
            vec![&full_identity_query, &identity_id_by_public_key_hash_query],
            grove_version,
        )
        .map_err(Error::from)
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
        .map_err(Error::from)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::Drive;
    use dpp::identity::Purpose;
    use grovedb_version::version::GroveVersion;

    mod identity_prove_request_type {
        use super::*;

        #[test]
        fn should_convert_valid_values() {
            assert!(matches!(
                IdentityProveRequestType::try_from(0),
                Ok(IdentityProveRequestType::FullIdentity)
            ));
            assert!(matches!(
                IdentityProveRequestType::try_from(1),
                Ok(IdentityProveRequestType::Balance)
            ));
            assert!(matches!(
                IdentityProveRequestType::try_from(2),
                Ok(IdentityProveRequestType::Keys)
            ));
            assert!(matches!(
                IdentityProveRequestType::try_from(3),
                Ok(IdentityProveRequestType::Revision)
            ));
        }

        #[test]
        fn should_error_on_invalid_value() {
            let result = IdentityProveRequestType::try_from(4);
            assert!(result.is_err());

            let result = IdentityProveRequestType::try_from(255);
            assert!(result.is_err());
        }
    }

    mod query_construction {
        use super::*;

        #[test]
        fn should_build_revision_for_identity_id_path_query() {
            let identity_id = [1u8; 32];
            let pq = Drive::revision_for_identity_id_path_query(identity_id);

            assert!(!pq.path.is_empty());
            assert!(pq.query.limit.is_none());
            assert!(pq.query.offset.is_none());
        }

        #[test]
        fn should_build_revision_and_balance_path_query() {
            let identity_id = [2u8; 32];
            let grove_version = GroveVersion::latest();
            let pq = Drive::revision_and_balance_path_query(identity_id, grove_version)
                .expect("should build merged query");

            assert!(pq.query.limit.is_none());
            assert!(pq.query.offset.is_none());
        }

        #[test]
        fn should_build_identity_id_by_unique_public_key_hash_query() {
            let public_key_hash = [3u8; 20];
            let pq = Drive::identity_id_by_unique_public_key_hash_query(public_key_hash);

            assert!(!pq.path.is_empty());
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_identity_id_by_non_unique_public_key_hash_query_without_after() {
            let public_key_hash = [4u8; 20];
            let pq = Drive::identity_id_by_non_unique_public_key_hash_query(public_key_hash, None);

            assert!(!pq.path.is_empty());
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_identity_id_by_non_unique_public_key_hash_query_with_after() {
            let public_key_hash = [4u8; 20];
            let after_id = [5u8; 32];
            let pq = Drive::identity_id_by_non_unique_public_key_hash_query(
                public_key_hash,
                Some(after_id),
            );

            assert!(!pq.path.is_empty());
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_identity_ids_by_unique_public_key_hash_query() {
            let hashes = [[6u8; 20], [7u8; 20], [8u8; 20]];
            let pq = Drive::identity_ids_by_unique_public_key_hash_query(&hashes);

            assert!(!pq.path.is_empty());
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_identity_ids_by_unique_public_key_hash_query_empty() {
            let hashes: [[u8; 20]; 0] = [];
            let pq = Drive::identity_ids_by_unique_public_key_hash_query(&hashes);
            assert!(!pq.path.is_empty());
        }

        #[test]
        fn should_build_full_identity_query() {
            let identity_id = [9u8; 32];
            let grove_version = GroveVersion::latest();
            let pq = Drive::full_identity_query(&identity_id, grove_version)
                .expect("should build full identity query");

            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_identity_all_keys_query() {
            let identity_id = [10u8; 32];
            let grove_version = GroveVersion::latest();
            let pq = Drive::identity_all_keys_query(&identity_id, grove_version)
                .expect("should build all keys query");

            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_balances_for_identity_ids_query() {
            let ids = [[11u8; 32], [12u8; 32]];
            let pq = Drive::balances_for_identity_ids_query(&ids);

            assert!(!pq.path.is_empty());
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_balances_for_range_query_ascending_no_start() {
            let pq = Drive::balances_for_range_query(None, true, 10);
            assert_eq!(pq.query.limit, Some(10));
        }

        #[test]
        fn should_build_balances_for_range_query_ascending_with_start_included() {
            let start = [13u8; 32];
            let pq = Drive::balances_for_range_query(Some((start, true)), true, 5);
            assert_eq!(pq.query.limit, Some(5));
        }

        #[test]
        fn should_build_balances_for_range_query_ascending_with_start_excluded() {
            let start = [14u8; 32];
            let pq = Drive::balances_for_range_query(Some((start, false)), true, 5);
            assert_eq!(pq.query.limit, Some(5));
        }

        #[test]
        fn should_build_balances_for_range_query_descending_no_start() {
            let pq = Drive::balances_for_range_query(None, false, 10);
            assert_eq!(pq.query.limit, Some(10));
        }

        #[test]
        fn should_build_balances_for_range_query_descending_with_start_included() {
            let start = [15u8; 32];
            let pq = Drive::balances_for_range_query(Some((start, true)), false, 5);
            assert_eq!(pq.query.limit, Some(5));
        }

        #[test]
        fn should_build_balances_for_range_query_descending_with_start_excluded() {
            let start = [16u8; 32];
            let pq = Drive::balances_for_range_query(Some((start, false)), false, 5);
            assert_eq!(pq.query.limit, Some(5));
        }

        #[test]
        fn should_build_full_identities_query() {
            let ids = [[17u8; 32], [18u8; 32]];
            let grove_version = GroveVersion::latest();
            let pq = Drive::full_identities_query(&ids, grove_version)
                .expect("should build full identities query");
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_full_identity_with_public_key_hash_query() {
            let public_key_hash = [19u8; 20];
            let identity_id = [20u8; 32];
            let grove_version = GroveVersion::latest();
            let pq = Drive::full_identity_with_public_key_hash_query(
                public_key_hash,
                identity_id,
                grove_version,
            )
            .expect("should build query");
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_full_identity_with_non_unique_public_key_hash_query_no_after() {
            let public_key_hash = [21u8; 20];
            let identity_id = [22u8; 32];
            let grove_version = GroveVersion::latest();
            let pq = Drive::full_identity_with_non_unique_public_key_hash_query(
                public_key_hash,
                identity_id,
                None,
                grove_version,
            )
            .expect("should build query");
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_full_identity_with_non_unique_public_key_hash_query_with_after() {
            let public_key_hash = [23u8; 20];
            let identity_id = [24u8; 32];
            let after = [25u8; 32];
            let grove_version = GroveVersion::latest();
            let pq = Drive::full_identity_with_non_unique_public_key_hash_query(
                public_key_hash,
                identity_id,
                Some(after),
                grove_version,
            )
            .expect("should build query");
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_full_identities_with_keys_hashes_query() {
            let ids = [[26u8; 32], [27u8; 32]];
            let hashes = [[28u8; 20], [29u8; 20]];
            let grove_version = GroveVersion::latest();
            let pq = Drive::full_identities_with_keys_hashes_query(&ids, &hashes, grove_version)
                .expect("should build query");
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_identity_balance_query() {
            let identity_id = [30u8; 32];
            let pq = Drive::identity_balance_query(&identity_id);

            assert!(!pq.path.is_empty());
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_identities_contract_keys_query() {
            let ids = [[31u8; 32], [32u8; 32]];
            let contract_id = [33u8; 32];
            let purposes = vec![Purpose::ENCRYPTION];
            let pq = Drive::identities_contract_keys_query(
                &ids,
                &contract_id,
                &None,
                &purposes,
                Some(10),
            );

            assert!(!pq.path.is_empty());
            assert_eq!(pq.query.limit, Some(10));
        }

        #[test]
        fn should_build_identities_contract_keys_query_with_document_type() {
            let ids = [[34u8; 32]];
            let contract_id = [35u8; 32];
            let doc_type_name = Some("profile".to_string());
            let purposes = vec![Purpose::ENCRYPTION, Purpose::DECRYPTION];
            let pq = Drive::identities_contract_keys_query(
                &ids,
                &contract_id,
                &doc_type_name,
                &purposes,
                None,
            );

            assert!(!pq.path.is_empty());
            assert!(pq.query.limit.is_none());
        }

        #[test]
        fn should_build_identities_contract_document_type_keys_query() {
            let ids = [[36u8; 32], [37u8; 32]];
            let contract_id = [38u8; 32];
            let purposes = vec![Purpose::ENCRYPTION];
            let pq = Drive::identities_contract_document_type_keys_query(
                &ids,
                contract_id,
                "profile",
                purposes,
            );

            assert!(!pq.path.is_empty());
            assert!(pq.query.limit.is_none());
            // Note: currently the document type parameter does not affect the
            // query path structure. This may be a bug or an intentional
            // simplification in the current implementation.
        }

        #[test]
        fn should_build_balance_for_identity_id_query() {
            let identity_id = [39u8; 32];
            let pq = Drive::balance_for_identity_id_query(identity_id);
            assert!(!pq.path.is_empty());
        }

        #[test]
        fn should_build_identity_nonce_query() {
            let identity_id = [40u8; 32];
            let pq = Drive::identity_nonce_query(identity_id);
            assert!(!pq.path.is_empty());
        }

        #[test]
        fn should_build_identity_contract_nonce_query() {
            let identity_id = [41u8; 32];
            let contract_id = [42u8; 32];
            let pq = Drive::identity_contract_nonce_query(identity_id, contract_id);
            assert!(!pq.path.is_empty());
        }

        #[test]
        fn should_build_balance_and_revision_for_identity_id_query() {
            let identity_id = [43u8; 32];
            let grove_version = GroveVersion::latest();
            let pq = Drive::balance_and_revision_for_identity_id_query(identity_id, grove_version);
            assert!(pq.query.limit.is_none());
        }
    }
}
