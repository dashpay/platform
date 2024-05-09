// Grouping std imports
use std::{
    collections::{BTreeMap, HashSet},
    ops::RangeFull,
};

// Conditional imports for the features "server" or "verify"
#[cfg(any(feature = "server", feature = "verify"))]
use {
    crate::{
        drive::identity::{
            identity_contract_info_group_path_key_purpose_vec, identity_key_tree_path_vec,
            identity_query_keys_tree_path_vec,
            key::fetch::KeyKindRequestType::{AllKeysOfKindRequest, CurrentKeyOfKindRequest},
            key::fetch::KeyRequestType::{
                AllKeys, ContractBoundKey, ContractDocumentTypeBoundKey, SearchKey, SpecificKeys,
            },
        },
        query::{Query, QueryItem},
    },
    dpp::{
        identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0,
        identity::{KeyID, Purpose},
    },
    grovedb::{PathQuery, SizedQuery},
    integer_encoding::VarInt,
};

// Conditional imports for the feature "server"
#[cfg(feature = "server")]
use {
    crate::error::{drive::DriveError, fee::FeeError, identity::IdentityError, Error},
    dpp::{
        block::epoch::Epoch,
        fee::{
            default_costs::{EpochCosts, KnownCostItem::FetchSingleIdentityKeyProcessingCost},
            Credits,
        },
        identity::{IdentityPublicKey, SecurityLevel},
        serialization::PlatformDeserializable,
        version::PlatformVersion,
    },
    grovedb::{
        query_result_type::{
            Key, Path, PathKeyOptionalElementTrio, QueryResultElement, QueryResultElements,
        },
        Element,
        Element::Item,
    },
};

// Modules conditionally compiled for the feature "server"
#[cfg(feature = "server")]
mod fetch_all_current_identity_keys;
#[cfg(feature = "server")]
mod fetch_all_identity_keys;
#[cfg(feature = "server")]
mod fetch_identities_all_keys;
#[cfg(feature = "server")]
mod fetch_identity_keys;

#[cfg(any(feature = "server", feature = "verify"))]
/// The kind of keys you are requesting
/// A kind is a purpose/security level pair
/// Do you want to get all keys in that pair
/// Or just the current one?
#[derive(Clone, Copy)]
pub enum KeyKindRequestType {
    /// Get only the last key of a certain kind
    CurrentKeyOfKindRequest,
    /// Get all keys of a certain kind
    AllKeysOfKindRequest,
}

#[cfg(any(feature = "server", feature = "verify"))]
/// The type of key request
#[derive(Clone)]
pub enum KeyRequestType {
    /// Get all keys of an identity
    AllKeys,
    /// Get specific keys for an identity
    SpecificKeys(Vec<KeyID>),
    /// Search for keys on an identity
    SearchKey(BTreeMap<PurposeU8, BTreeMap<SecurityLevelU8, KeyKindRequestType>>),
    /// Search for contract bound keys
    ContractBoundKey([u8; 32], Purpose, KeyKindRequestType),
    /// Search for contract bound keys
    ContractDocumentTypeBoundKey([u8; 32], String, Purpose, KeyKindRequestType),
}

#[cfg(any(feature = "server", feature = "verify"))]
/// The key purpose as u8.
pub type PurposeU8 = u8;
#[cfg(any(feature = "server", feature = "verify"))]
/// The key security level as u8.
pub type SecurityLevelU8 = u8;

#[cfg(feature = "server")]
/// Type alias for a hashset of IdentityPublicKey Ids as the outcome of the query.
pub type KeyIDHashSet = HashSet<KeyID>;

#[cfg(feature = "server")]
/// Type alias for a vec of IdentityPublicKey Ids as the outcome of the query.
pub type KeyIDVec = Vec<KeyID>;

#[cfg(feature = "server")]
/// Type alias for a vec of IdentityPublicKeys as the outcome of the query.
pub type KeyVec = Vec<IdentityPublicKey>;

#[cfg(feature = "server")]
/// Type alias for a vec of serialized IdentityPublicKeys as the outcome of the query.
pub type SerializedKeyVec = Vec<Vec<u8>>;

#[cfg(feature = "server")]
/// Type alias for a single IdentityPublicKey as the outcome of the query.
pub type SingleIdentityPublicKeyOutcome = IdentityPublicKey;

#[cfg(feature = "server")]
/// Type alias for an optional single IdentityPublicKey as the outcome of the query.
pub type OptionalSingleIdentityPublicKeyOutcome = Option<IdentityPublicKey>;

#[cfg(feature = "server")]
/// Type alias for a Vector for key id to identity public key pair common pattern.
pub type KeyIDIdentityPublicKeyPairVec = Vec<(KeyID, IdentityPublicKey)>;

#[cfg(feature = "server")]
/// Type alias for a Vector for key id to optional identity public key pair common pattern.
pub type KeyIDOptionalIdentityPublicKeyPairVec = Vec<(KeyID, Option<IdentityPublicKey>)>;

#[cfg(feature = "server")]
/// Type alias for a Vector for query key path to optional identity public key pair common pattern.
pub type QueryKeyPathOptionalIdentityPublicKeyTrioVec = Vec<(Path, Key, Option<IdentityPublicKey>)>;

#[cfg(feature = "server")]
/// Type alias for a bTreemap for a key id to identity public key pair common pattern.
pub type KeyIDIdentityPublicKeyPairBTreeMap = BTreeMap<KeyID, IdentityPublicKey>;

#[cfg(feature = "server")]
/// Type alias for a bTreemap for a key id to optional identity public key pair common pattern.
pub type KeyIDOptionalIdentityPublicKeyPairBTreeMap = BTreeMap<KeyID, Option<IdentityPublicKey>>;

#[cfg(feature = "server")]
/// Type alias for a bTreemap for a query key path to optional identity public key pair common pattern.
pub type QueryKeyPathOptionalIdentityPublicKeyTrioBTreeMap =
    BTreeMap<(Path, Key), Option<IdentityPublicKey>>;

#[cfg(feature = "server")]
/// A trait to get typed results from raw results from Drive
pub trait IdentityPublicKeyResult {
    /// Get a typed result from a trio of path key elements
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error>
    where
        Self: Sized;
    /// Get a typed result from query results
    fn try_from_query_results(
        value: QueryResultElements,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error>
    where
        Self: Sized;
}

#[cfg(feature = "server")]
fn element_to_serialized_identity_public_key(element: Element) -> Result<Vec<u8>, Error> {
    let Item(value, _) = element else {
        return Err(Error::Drive(DriveError::CorruptedElementType(
            "expected item for identity public key",
        )));
    };

    Ok(value)
}

#[cfg(feature = "server")]
fn element_to_identity_public_key(element: Element) -> Result<IdentityPublicKey, Error> {
    let Item(value, _) = element else {
        return Err(Error::Drive(DriveError::CorruptedElementType(
            "expected item for identity public key",
        )));
    };

    IdentityPublicKey::deserialize_from_bytes(value.as_slice()).map_err(Error::Protocol)
}

#[cfg(feature = "server")]
fn element_to_identity_public_key_id(element: Element) -> Result<KeyID, Error> {
    let public_key = element_to_identity_public_key(element)?;

    Ok(public_key.id())
}

#[cfg(feature = "server")]
fn element_to_identity_public_key_id_and_object_pair(
    element: Element,
) -> Result<(KeyID, IdentityPublicKey), Error> {
    let public_key = element_to_identity_public_key(element)?;

    Ok((public_key.id(), public_key))
}

#[cfg(feature = "server")]
fn key_and_optional_element_to_identity_public_key_id_and_object_pair(
    (_path, key, maybe_element): (Path, Key, Option<Element>),
) -> Result<(KeyID, Option<IdentityPublicKey>), Error> {
    if let Some(element) = maybe_element {
        let public_key = element_to_identity_public_key(element)?;

        return Ok((public_key.id(), Some(public_key)));
    }

    let (key_id, _) = KeyID::decode_var(key.as_slice()).ok_or_else(|| {
        Error::Drive(DriveError::CorruptedSerialization(String::from(
            "can't decode key id",
        )))
    })?;

    Ok((key_id, None))
}

#[cfg(feature = "server")]
fn supported_query_result_element_to_identity_public_key(
    query_result_element: QueryResultElement,
) -> Result<IdentityPublicKey, Error> {
    match query_result_element {
        QueryResultElement::ElementResultItem(element)
        | QueryResultElement::KeyElementPairResultItem((_, element))
        | QueryResultElement::PathKeyElementTrioResultItem((_, _, element)) => {
            element_to_identity_public_key(element)
        }
    }
}

#[cfg(feature = "server")]
fn supported_query_result_element_to_serialized_identity_public_key(
    query_result_element: QueryResultElement,
) -> Result<Vec<u8>, Error> {
    match query_result_element {
        QueryResultElement::ElementResultItem(element)
        | QueryResultElement::KeyElementPairResultItem((_, element))
        | QueryResultElement::PathKeyElementTrioResultItem((_, _, element)) => {
            element_to_serialized_identity_public_key(element)
        }
    }
}

#[cfg(feature = "server")]
fn supported_query_result_element_to_identity_public_key_id(
    query_result_element: QueryResultElement,
) -> Result<KeyID, Error> {
    match query_result_element {
        QueryResultElement::ElementResultItem(element)
        | QueryResultElement::KeyElementPairResultItem((_, element))
        | QueryResultElement::PathKeyElementTrioResultItem((_, _, element)) => {
            element_to_identity_public_key_id(element)
        }
    }
}

#[cfg(feature = "server")]
fn supported_query_result_element_to_identity_public_key_id_and_object_pair(
    query_result_element: QueryResultElement,
) -> Result<(KeyID, IdentityPublicKey), Error> {
    match query_result_element {
        QueryResultElement::ElementResultItem(element)
        | QueryResultElement::KeyElementPairResultItem((_, element))
        | QueryResultElement::PathKeyElementTrioResultItem((_, _, element)) => {
            element_to_identity_public_key_id_and_object_pair(element)
        }
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for SingleIdentityPublicKeyOutcome {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        // We do not care about non existence
        let mut keys = value
            .into_iter()
            .filter_map(|(_, _, maybe_element)| maybe_element)
            .map(element_to_identity_public_key)
            .collect::<Result<Vec<_>, Error>>()?;

        if keys.is_empty() {
            return Err(Error::Identity(IdentityError::IdentityPublicKeyNotFound(
                "no result found".to_string(),
            )));
        }

        if keys.len() > 1 {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "more than one key was returned when expecting only one result",
            )));
        }

        Ok(keys.remove(0))
    }

    fn try_from_query_results(
        value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let mut keys = value
            .elements
            .into_iter()
            .map(supported_query_result_element_to_identity_public_key)
            .collect::<Result<Vec<_>, Error>>()?;

        if keys.is_empty() {
            return Err(Error::Identity(IdentityError::IdentityPublicKeyNotFound(
                "no result found".to_string(),
            )));
        }

        if keys.len() > 1 {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "more than one key was returned when expecting only one result",
            )));
        }

        Ok(keys.remove(0))
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for OptionalSingleIdentityPublicKeyOutcome {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        // We do not care about non existence
        let mut keys = value
            .into_iter()
            .filter_map(|(_, _, maybe_element)| maybe_element)
            .map(element_to_identity_public_key)
            .collect::<Result<Vec<_>, Error>>()?;

        if keys.is_empty() {
            return Ok(None);
        }

        if keys.len() > 1 {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "more than one key was returned when expecting only one result",
            )));
        }

        Ok(Some(keys.remove(0)))
    }

    fn try_from_query_results(
        value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let mut keys = value
            .elements
            .into_iter()
            .map(supported_query_result_element_to_identity_public_key)
            .collect::<Result<Vec<_>, Error>>()?;

        if keys.is_empty() {
            return Ok(None);
        }

        if keys.len() > 1 {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "more than one key was returned when expecting only one result",
            )));
        }

        Ok(Some(keys.remove(0)))
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for KeyIDHashSet {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        // We do not care about non existence
        value
            .into_iter()
            .filter_map(|(_, _, maybe_element)| maybe_element)
            .map(element_to_identity_public_key_id)
            .collect()
    }

    fn try_from_query_results(
        value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        value
            .elements
            .into_iter()
            .map(supported_query_result_element_to_identity_public_key_id)
            .collect()
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for KeyIDVec {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        // We do not care about non existence
        value
            .into_iter()
            .filter_map(|(_, _, maybe_element)| maybe_element)
            .map(element_to_identity_public_key_id)
            .collect()
    }

    fn try_from_query_results(
        value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        value
            .elements
            .into_iter()
            .map(supported_query_result_element_to_identity_public_key_id)
            .collect()
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for KeyVec {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        // We do not care about non existence
        value
            .into_iter()
            .filter_map(|(_, _, maybe_element)| maybe_element)
            .map(element_to_identity_public_key)
            .collect()
    }

    fn try_from_query_results(
        value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        value
            .elements
            .into_iter()
            .map(supported_query_result_element_to_identity_public_key)
            .collect()
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for SerializedKeyVec {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        // We do not care about non existence
        value
            .into_iter()
            .filter_map(|(_, _, maybe_element)| maybe_element)
            .map(element_to_serialized_identity_public_key)
            .collect()
    }

    fn try_from_query_results(
        value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        value
            .elements
            .into_iter()
            .map(supported_query_result_element_to_serialized_identity_public_key)
            .collect()
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for KeyIDIdentityPublicKeyPairVec {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        // We do not care about non existence
        value
            .into_iter()
            .filter_map(|(_, _, maybe_element)| maybe_element)
            .map(element_to_identity_public_key_id_and_object_pair)
            .collect()
    }

    fn try_from_query_results(
        value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        value
            .elements
            .into_iter()
            .map(supported_query_result_element_to_identity_public_key_id_and_object_pair)
            .collect()
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for KeyIDOptionalIdentityPublicKeyPairVec {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        value
            .into_iter()
            .map(key_and_optional_element_to_identity_public_key_id_and_object_pair)
            .collect()
    }

    fn try_from_query_results(
        _value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        Err(Error::Drive(DriveError::NotSupported(
            "KeyIDOptionalIdentityPublicKeyPairVec try from QueryResultElements",
        )))
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for QueryKeyPathOptionalIdentityPublicKeyTrioVec {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
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

    fn try_from_query_results(
        _value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        Err(Error::Drive(DriveError::NotSupported(
            "QueryKeyPathOptionalIdentityPublicKeyTrioVec try from QueryResultElements",
        )))
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for KeyIDIdentityPublicKeyPairBTreeMap {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        // We do not care about non existence
        value
            .into_iter()
            .filter_map(|(_, _, maybe_element)| maybe_element)
            .map(element_to_identity_public_key_id_and_object_pair)
            .collect()
    }

    fn try_from_query_results(
        value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        value
            .elements
            .into_iter()
            .map(supported_query_result_element_to_identity_public_key_id_and_object_pair)
            .collect()
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for KeyIDOptionalIdentityPublicKeyPairBTreeMap {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        value
            .into_iter()
            .map(key_and_optional_element_to_identity_public_key_id_and_object_pair)
            .collect()
    }

    fn try_from_query_results(
        _value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        Err(Error::Drive(DriveError::NotSupported(
            "KeyIDOptionalIdentityPublicKeyPairVec try from QueryResultElements",
        )))
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for QueryKeyPathOptionalIdentityPublicKeyTrioBTreeMap {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
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

    fn try_from_query_results(
        _value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        Err(Error::Drive(DriveError::NotSupported(
            "QueryKeyPathOptionalIdentityPublicKeyTrioVec try from QueryResultElements",
        )))
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
/// A request to get Keys from an Identity
#[derive(Clone)]
pub struct IdentityKeysRequest {
    /// The request identity id
    pub identity_id: [u8; 32],
    /// The type of key request
    pub request_type: KeyRequestType,
    /// The limit of the amount of keys you wish to get back
    pub limit: Option<u16>,
    /// The offset of the start of the amount of keys you wish to get back
    pub offset: Option<u16>,
}

impl IdentityKeysRequest {
    #[cfg(feature = "server")]
    /// Gets the processing cost of an identity keys request
    pub fn processing_cost(&self, epoch: &Epoch) -> Result<Credits, Error> {
        match &self.request_type {
            AllKeys => Err(Error::Fee(FeeError::OperationNotAllowed(
                "You can not get costs for requesting all keys",
            ))),
            SpecificKeys(keys) => Ok(keys.len() as u64
                * epoch.cost_for_known_cost_item(FetchSingleIdentityKeyProcessingCost)),
            SearchKey(_search) => todo!(),
            ContractBoundKey(_, _, key_kind) | ContractDocumentTypeBoundKey(_, _, _, key_kind) => {
                match key_kind {
                    CurrentKeyOfKindRequest => {
                        Ok(epoch.cost_for_known_cost_item(FetchSingleIdentityKeyProcessingCost))
                    }
                    AllKeysOfKindRequest => Err(Error::Fee(FeeError::OperationNotAllowed(
                        "You can not get costs for an all keys of kind request",
                    ))),
                }
            }
        }
    }

    #[cfg(feature = "server")]
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
            request_type: SearchKey(purpose_btree_map),
            limit: None,
            offset: None,
        }
    }

    #[cfg(feature = "server")]
    /// Make a request for an encryption key for a specific contract
    pub fn new_contract_encryption_keys_query(
        identity_id: [u8; 32],
        contract_id: [u8; 32],
    ) -> Self {
        IdentityKeysRequest {
            identity_id,
            request_type: ContractBoundKey(
                contract_id,
                Purpose::ENCRYPTION,
                CurrentKeyOfKindRequest,
            ),
            limit: None,
            offset: None,
        }
    }

    #[cfg(feature = "server")]
    /// Make a request for an decryption key for a specific contract
    pub fn new_contract_decryption_keys_query(
        identity_id: [u8; 32],
        contract_id: [u8; 32],
    ) -> Self {
        IdentityKeysRequest {
            identity_id,
            request_type: ContractBoundKey(
                contract_id,
                Purpose::DECRYPTION,
                CurrentKeyOfKindRequest,
            ),
            limit: None,
            offset: None,
        }
    }

    #[cfg(feature = "server")]
    /// Make a request for an encryption key for a specific contract document type
    pub fn new_document_type_encryption_keys_query(
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        document_type_name: String,
    ) -> Self {
        IdentityKeysRequest {
            identity_id,
            request_type: ContractDocumentTypeBoundKey(
                contract_id,
                document_type_name,
                Purpose::ENCRYPTION,
                CurrentKeyOfKindRequest,
            ),
            limit: None,
            offset: None,
        }
    }

    #[cfg(feature = "server")]
    /// Make a request for an decryption key for a specific contract document type
    pub fn new_document_type_decryption_keys_query(
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        document_type_name: String,
    ) -> Self {
        IdentityKeysRequest {
            identity_id,
            request_type: ContractDocumentTypeBoundKey(
                contract_id,
                document_type_name,
                Purpose::DECRYPTION,
                CurrentKeyOfKindRequest,
            ),
            limit: None,
            offset: None,
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Make a request for all current keys for the identity
    pub fn new_all_keys_query(identity_id: &[u8; 32], limit: Option<u16>) -> Self {
        IdentityKeysRequest {
            identity_id: *identity_id,
            request_type: AllKeys,
            limit,
            offset: None,
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Make a request for specific keys for the identity
    pub fn new_specific_keys_query(identity_id: &[u8; 32], key_ids: Vec<KeyID>) -> Self {
        let limit = key_ids.len() as u16;
        IdentityKeysRequest {
            identity_id: *identity_id,
            request_type: SpecificKeys(key_ids),
            limit: Some(limit),
            offset: None,
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Make a request for specific keys for the identity
    pub fn new_specific_keys_query_without_limit(
        identity_id: &[u8; 32],
        key_ids: Vec<KeyID>,
    ) -> Self {
        IdentityKeysRequest {
            identity_id: *identity_id,
            request_type: SpecificKeys(key_ids),
            limit: None,
            offset: None,
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Make a request for a specific key for the identity without a limit
    /// Not have a limit is needed if you want to merge path queries
    pub fn new_specific_key_query_without_limit(identity_id: &[u8; 32], key_id: KeyID) -> Self {
        IdentityKeysRequest {
            identity_id: *identity_id,
            request_type: SpecificKeys(vec![key_id]),
            limit: None,
            offset: None,
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Make a request for a specific key for the identity
    pub fn new_specific_key_query(identity_id: &[u8; 32], key_id: KeyID) -> Self {
        IdentityKeysRequest {
            identity_id: *identity_id,
            request_type: SpecificKeys(vec![key_id]),
            limit: Some(1),
            offset: None,
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Create the path query for the request
    pub fn into_path_query(self) -> PathQuery {
        let IdentityKeysRequest {
            identity_id,
            request_type: key_request,
            mut limit,
            offset,
        } = self;

        match key_request {
            AllKeys => {
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
            SpecificKeys(key_ids) => {
                let query_keys_path = identity_key_tree_path_vec(identity_id.as_slice());
                PathQuery {
                    path: query_keys_path,
                    query: SizedQuery {
                        query: Self::specific_keys_query(key_ids),
                        limit,
                        offset: None,
                    },
                }
            }
            SearchKey(map) => {
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
            ContractBoundKey(contract_id, purpose, key_request_type) => {
                let query_keys_path = identity_contract_info_group_path_key_purpose_vec(
                    &identity_id,
                    &contract_id,
                    purpose,
                );
                let query = match key_request_type {
                    CurrentKeyOfKindRequest => {
                        limit = Some(1);
                        Query::new_single_key(vec![])
                    }
                    AllKeysOfKindRequest => {
                        Query::new_single_query_item(QueryItem::RangeFull(RangeFull))
                    }
                };
                PathQuery {
                    path: query_keys_path,
                    query: SizedQuery {
                        query,
                        limit,
                        offset,
                    },
                }
            }
            ContractDocumentTypeBoundKey(
                contract_id,
                document_type_name,
                purpose,
                key_request_type,
            ) => {
                let mut group_id = contract_id.to_vec();
                group_id.extend(document_type_name.as_bytes());
                let query_keys_path = identity_contract_info_group_path_key_purpose_vec(
                    &identity_id,
                    &group_id,
                    purpose,
                );
                let query = match key_request_type {
                    CurrentKeyOfKindRequest => {
                        limit = Some(1);
                        Query::new_single_key(vec![])
                    }
                    AllKeysOfKindRequest => {
                        Query::new_single_query_item(QueryItem::RangeFull(RangeFull))
                    }
                };
                PathQuery {
                    path: query_keys_path,
                    query: SizedQuery {
                        query,
                        limit,
                        offset,
                    },
                }
            }
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// All keys
    fn all_keys_query() -> Query {
        let mut query = Query::new();
        query.insert_all();
        query
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Fetch a specific key knowing the id
    fn specific_keys_query(key_ids: Vec<KeyID>) -> Query {
        let mut query = Query::new();
        for key_id in key_ids {
            query.insert_key(key_id.encode_var_vec());
        }
        query
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Construct the query for the request
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

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::tests::helpers::setup::setup_drive;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::drive_versions::DriveVersion;

    use super::*;

    #[test]
    fn test_fetch_all_keys_on_identity() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();

        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to insert identity");

        let public_keys = drive
            .fetch_all_identity_keys(
                identity.id().to_buffer(),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to fetch keys");

        assert_eq!(public_keys.len(), 5);
    }

    #[test]
    fn test_fetch_single_identity_key() {
        let drive = setup_drive(None);
        let _drive_version = DriveVersion::latest();

        let transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::first();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to insert identity");

        let key_request = IdentityKeysRequest {
            identity_id: identity.id().to_buffer(),
            request_type: SpecificKeys(vec![0]),
            limit: Some(1),
            offset: None,
        };

        let public_keys: KeyIDIdentityPublicKeyPairBTreeMap = drive
            .fetch_identity_keys(key_request, Some(&transaction), platform_version)
            .expect("expected to fetch keys");

        assert_eq!(public_keys.len(), 1);
    }

    #[test]
    fn test_fetch_multiple_identity_key() {
        let drive = setup_drive(None);
        let _drive_version = DriveVersion::latest();

        let transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::first();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to insert identity");

        let key_request = IdentityKeysRequest {
            identity_id: identity.id().to_buffer(),
            request_type: SpecificKeys(vec![0, 4]),
            limit: Some(2),
            offset: None,
        };

        let public_keys: KeyIDIdentityPublicKeyPairBTreeMap = drive
            .fetch_identity_keys(key_request, Some(&transaction), platform_version)
            .expect("expected to fetch keys");

        assert_eq!(public_keys.len(), 2);
    }

    #[test]
    fn test_fetch_unknown_identity_key_returns_not_found() {
        let drive = setup_drive(None);
        let _drive_version = DriveVersion::latest();

        let transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::first();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to insert identity");

        let key_request = IdentityKeysRequest {
            identity_id: identity.id().to_buffer(),
            request_type: SpecificKeys(vec![0, 6]),
            limit: Some(2),
            offset: None,
        };

        let public_keys: KeyIDIdentityPublicKeyPairBTreeMap = drive
            .fetch_identity_keys(key_request.clone(), Some(&transaction), platform_version)
            .expect("expected to fetch keys");

        assert_eq!(public_keys.len(), 1); //because we are not requesting with options

        let public_keys: KeyIDOptionalIdentityPublicKeyPairBTreeMap = drive
            .fetch_identity_keys(key_request, Some(&transaction), platform_version)
            .expect("expected to fetch keys");

        assert_eq!(public_keys.len(), 2);
    }
}
