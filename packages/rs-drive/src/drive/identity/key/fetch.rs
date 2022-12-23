





use crate::drive::identity::{
    identity_key_tree_path_vec, identity_query_keys_tree_path_vec,
};

use crate::drive::identity::key::fetch::KeyKindRequestType::{
    AllKeysOfKindRequest, CurrentKeyOfKindRequest,
};
use crate::drive::identity::key::fetch::KeyRequestType::{
    AllKeysRequest, SearchKeyRequest, SpecificKeysRequest,
};
use crate::drive::{Drive};
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::DriveOperation;

use crate::query::{Query, QueryItem};

use dpp::identity::{KeyID, Purpose, SecurityLevel};
use dpp::prelude::IdentityPublicKey;
use grovedb::query_result_type::QueryResultType::{
    QueryPathKeyElementTrioResultType,
};
use grovedb::query_result_type::{
    Key, Path, PathKeyOptionalElementTrio, QueryResultElement,
    QueryResultElements,
};
use grovedb::Element::{Item};
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
    SpecificKeysRequest(Vec<KeyID>),
    SearchKeyRequest(BTreeMap<PurposeU8, BTreeMap<SecurityLevelU8, KeyKindRequestType>>),
}

type PurposeU8 = u8;
type SecurityLevelU8 = u8;

/// Type alias for a Vector for key id to identity public key pair common pattern.
pub type KeyIDIdentityPublicKeyPairVec = Vec<(KeyID, IdentityPublicKey)>;

/// Type alias for a Vector for key id to optional identity public key pair common pattern.
pub type KeyIDOptionalIdentityPublicKeyPairVec = Vec<(KeyID, Option<IdentityPublicKey>)>;

/// Type alias for a Vector for query key path to optional identity public key pair common pattern.
pub type QueryKeyPathOptionalIdentityPublicKeyTrioVec = Vec<(Path, Key, Option<IdentityPublicKey>)>;

/// Type alias for a bTreemap for a key id to identity public key pair common pattern.
pub type KeyIDIdentityPublicKeyPairBTreeMap = BTreeMap<KeyID, IdentityPublicKey>;

/// Type alias for a bTreemap for a key id to optional identity public key pair common pattern.
pub type KeyIDOptionalIdentityPublicKeyPairBTreeMap = BTreeMap<KeyID, Option<IdentityPublicKey>>;

/// Type alias for a bTreemap for a query key path to optional identity public key pair common pattern.
pub type QueryKeyPathOptionalIdentityPublicKeyTrioBTreeMap =
    BTreeMap<(Path, Key), Option<IdentityPublicKey>>;

pub trait IdentityPublicKeyResult {
    fn try_from_path_key_optional(value: Vec<PathKeyOptionalElementTrio>) -> Result<Self, Error>
    where
        Self: Sized;
    fn try_from_query_results(value: QueryResultElements) -> Result<Self, Error>
    where
        Self: Sized;
}

fn element_to_identity_public_key(element: Element) -> Result<IdentityPublicKey, Error> {
    let Item(value, _) = element else {
        return Err(Error::Drive(DriveError::CorruptedElementType(
            "expected item for identity public key",
        )))
    };

    IdentityPublicKey::deserialize(value.as_slice()).map_err(Error::Protocol)
}

// fn element_to_identity_public_key_id_and_object_pair(element: Element) -> Result<IdentityPublicKey, Error> {
//     let public_key = element_to_identity_public_key(element)?;
//
//     (public_key.idю public_key)
// }
//
fn key_and_optional_element_to_identity_public_key_id_and_object_pair(
    key: Key,
    maybe_element: Option<Element>,
) -> Result<(KeyID, Option<IdentityPublicKey>), Error> {
    if let Some(element) = maybe_element {
        let public_key = element_to_identity_public_key(element)?;

        return Ok((public_key.id, Some(public_key)));
    }

    let (key_id, _) = KeyID::decode_var(key.as_slice())
        .ok_or_else(|| Error::Drive(DriveError::CorruptedSerialization("can't decode key id")))?;

    Ok((key_id, None))
}

impl IdentityPublicKeyResult for KeyIDIdentityPublicKeyPairVec {
    fn try_from_path_key_optional(value: Vec<PathKeyOptionalElementTrio>) -> Result<Self, Error> {
        // We do not care about non existence
        value
            .into_iter()
            .filter_map(|(_, _key, maybe_element)| maybe_element.map(|element| element))
            .map(|element| {
                let public_key = element_to_identity_public_key(element)?;

                Ok((public_key.id, public_key))
            })
            .collect()
    }

    fn try_from_query_results(value: QueryResultElements) -> Result<Self, Error> {
        value
            .elements
            .into_iter()
            .map(|query_result_element| match query_result_element {
                QueryResultElement::ElementResultItem(_element) => Err(Error::Identity(
                    IdentityError::IdentityKeyIncorrectQueryMissingInformation(
                        "no key present in return information",
                    ),
                )),
                QueryResultElement::KeyElementPairResultItem((_key, element))
                | QueryResultElement::PathKeyElementTrioResultItem((_, _key, element)) => {
                    let public_key = element_to_identity_public_key(element)?;

                    Ok((public_key.id, public_key))
                }
            })
            .collect()
    }
}

impl IdentityPublicKeyResult for KeyIDOptionalIdentityPublicKeyPairVec {
    fn try_from_path_key_optional(value: Vec<PathKeyOptionalElementTrio>) -> Result<Self, Error> {
        value
            .into_iter()
            .map(|(_, key, maybe_element)| {
                key_and_optional_element_to_identity_public_key_id_and_object_pair(
                    key,
                    maybe_element,
                )
            })
            .collect()
    }

    fn try_from_query_results(_value: QueryResultElements) -> Result<Self, Error> {
        Err(Error::Drive(DriveError::NotSupported(
            "KeyIDOptionalIdentityPublicKeyPairVec try from QueryResultElements",
        )))
    }
}

impl IdentityPublicKeyResult for QueryKeyPathOptionalIdentityPublicKeyTrioVec {
    fn try_from_path_key_optional(value: Vec<PathKeyOptionalElementTrio>) -> Result<Self, Error> {
        value
            .into_iter()
            .map(|(path, key, maybe_element)| {
                let maybe_public_key = if let Some(element) = maybe_element {
                    Some(element_to_identity_public_key(element)?)
                } else {
                    None
                };

                Ok((path, key, maybe_public_key))
            })
            .collect()
    }

    fn try_from_query_results(_value: QueryResultElements) -> Result<Self, Error> {
        Err(Error::Drive(DriveError::NotSupported(
            "QueryKeyPathOptionalIdentityPublicKeyTrioVec try from QueryResultElements",
        )))
    }
}

impl IdentityPublicKeyResult for KeyIDIdentityPublicKeyPairBTreeMap {
    fn try_from_path_key_optional(value: Vec<PathKeyOptionalElementTrio>) -> Result<Self, Error> {
        // We do not care about non existence
        value
            .into_iter()
            .filter_map(|(_, key, maybe_element)| maybe_element.map(|element| (key, element)))
            .map(|(_key, element)| {
                let public_key = element_to_identity_public_key(element)?;

                Ok((public_key.id, public_key))
            })
            .collect()
    }

    fn try_from_query_results(value: QueryResultElements) -> Result<Self, Error> {
        value
            .elements
            .into_iter()
            .map(|query_result_element| match query_result_element {
                QueryResultElement::ElementResultItem(_element) => Err(Error::Identity(
                    IdentityError::IdentityKeyIncorrectQueryMissingInformation(
                        "no key present in return information",
                    ),
                )),
                QueryResultElement::KeyElementPairResultItem((_key, element))
                | QueryResultElement::PathKeyElementTrioResultItem((_, _key, element)) => {
                    let public_key = element_to_identity_public_key(element)?;

                    Ok((public_key.id, public_key))
                }
            })
            .collect()
    }
}

impl IdentityPublicKeyResult for KeyIDOptionalIdentityPublicKeyPairBTreeMap {
    fn try_from_path_key_optional(value: Vec<PathKeyOptionalElementTrio>) -> Result<Self, Error> {
        value
            .into_iter()
            .map(|(_, key, maybe_element)| {
                key_and_optional_element_to_identity_public_key_id_and_object_pair(
                    key,
                    maybe_element,
                )
            })
            .collect()
    }

    fn try_from_query_results(_value: QueryResultElements) -> Result<Self, Error> {
        Err(Error::Drive(DriveError::NotSupported(
            "KeyIDOptionalIdentityPublicKeyPairVec try from QueryResultElements",
        )))
    }
}

impl IdentityPublicKeyResult for QueryKeyPathOptionalIdentityPublicKeyTrioBTreeMap {
    fn try_from_path_key_optional(value: Vec<PathKeyOptionalElementTrio>) -> Result<Self, Error> {
        value
            .into_iter()
            .map(|(path, key, maybe_element)| {
                let maybe_public_key = if let Some(element) = maybe_element {
                    Some(element_to_identity_public_key(element)?)
                } else {
                    None
                };

                Ok(((path, key), maybe_public_key))
            })
            .collect()
    }

    fn try_from_query_results(_value: QueryResultElements) -> Result<Self, Error> {
        Err(Error::Drive(DriveError::NotSupported(
            "QueryKeyPathOptionalIdentityPublicKeyTrioVec try from QueryResultElements",
        )))
    }
}

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
        self.fetch_identity_keys_operations::<KeyIDIdentityPublicKeyPairBTreeMap>(
            key_request,
            transaction,
            drive_operations,
        )
    }

    /// Fetch all the keys of every kind for a specific Identity
    pub fn fetch_all_identity_keys(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<BTreeMap<KeyID, IdentityPublicKey>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_all_identity_keys_operations(identity_id, transaction, &mut drive_operations)
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
    pub fn fetch_identity_keys<T: IdentityPublicKeyResult>(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
    ) -> Result<T, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_identity_keys_operations(key_request, transaction, &mut drive_operations)
    }

    /// Operations for fetching keys matching the request for a specific Identity
    pub(crate) fn fetch_identity_keys_operations<T: IdentityPublicKeyResult>(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<T, Error> {
        match &key_request.key_request {
            AllKeysRequest => {
                let path_query = key_request.to_path_query();

                let (result, _) = self.grove_get_raw_path_query(
                    &path_query,
                    transaction,
                    QueryPathKeyElementTrioResultType,
                    drive_operations,
                )?;

                T::try_from_query_results(result)
            }
            SpecificKeysRequest(_) => {
                let path_query = key_request.to_path_query();

                let result = self.grove_get_raw_path_query_with_optional(
                    &path_query,
                    transaction,
                    drive_operations,
                )?;

                T::try_from_path_key_optional(result)
            }
            SearchKeyRequest(_) => {
                let path_query = key_request.to_path_query();

                let result = self.grove_get_path_query_with_optional(
                    &path_query,
                    transaction,
                    drive_operations,
                )?;

                T::try_from_path_key_optional(result)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive;
    use crate::drive::block_info::BlockInfo;
    use dpp::identity::Identity;

    

    use crate::drive::identity::key::fetch::KeyRequestType::SpecificKeysRequest;
    use crate::drive::identity::key::fetch::{
        IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap,
    };
    

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
    }

    #[test]
    fn test_fetch_single_identity_key() {
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

        let key_request = IdentityKeysRequest {
            identity_id: identity.id.to_buffer(),
            key_request: SpecificKeysRequest(vec![0]),
            limit: None,
            offset: None,
        };

        let public_keys: KeyIDIdentityPublicKeyPairBTreeMap = drive
            .fetch_identity_keys(key_request, Some(&transaction))
            .expect("expected to fetch keys");

        assert_eq!(public_keys.len(), 1);
    }

    #[test]
    fn test_fetch_multiple_identity_key() {
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

        let key_request = IdentityKeysRequest {
            identity_id: identity.id.to_buffer(),
            key_request: SpecificKeysRequest(vec![0, 4]),
            limit: None,
            offset: None,
        };

        let public_keys: KeyIDIdentityPublicKeyPairBTreeMap = drive
            .fetch_identity_keys(key_request, Some(&transaction))
            .expect("expected to fetch keys");

        assert_eq!(public_keys.len(), 2);
    }

    #[test]
    fn test_fetch_unknown_identity_key_returns_not_found() {
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

        let key_request = IdentityKeysRequest {
            identity_id: identity.id.to_buffer(),
            key_request: SpecificKeysRequest(vec![0, 6]),
            limit: None,
            offset: None,
        };

        let public_keys: KeyIDIdentityPublicKeyPairBTreeMap = drive
            .fetch_identity_keys(key_request, Some(&transaction))
            .expect("expected to fetch keys");

        assert_eq!(public_keys.len(), 2);
    }
}
