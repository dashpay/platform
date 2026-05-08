// Conditional imports for the features "server" or "verify"
#[cfg(any(feature = "server", feature = "verify"))]
use {
    crate::{
        drive::identity::{
            identity_contract_info_group_path_key_purpose_vec, identity_key_tree_path_vec,
            identity_query_keys_security_level_tree_path_vec, identity_query_keys_tree_path_vec,
            identity_transfer_keys_path_vec,
            key::fetch::KeyKindRequestType::{AllKeysOfKindRequest, CurrentKeyOfKindRequest},
            key::fetch::KeyRequestType::{
                AllKeys, ContractBoundKey, ContractDocumentTypeBoundKey, SearchKey, SpecificKeys,
            },
        },
        query::{Query, QueryItem},
    },
    dpp::identity::{KeyID, Purpose, SecurityLevel},
    grovedb::{PathQuery, SizedQuery},
    integer_encoding::VarInt,
    std::{collections::BTreeMap, ops::RangeFull},
};

#[cfg(feature = "server")]
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;

#[cfg(feature = "server")]
use {
    crate::error::{drive::DriveError, fee::FeeError, identity::IdentityError, Error},
    dpp::{
        fee::Credits, identity::IdentityPublicKey, serialization::PlatformDeserializable,
        version::PlatformVersion,
    },
    grovedb::{
        query_result_type::{
            Key, Path, PathKeyOptionalElementTrio, QueryResultElement, QueryResultElements,
        },
        Element,
        Element::Item,
    },
    std::collections::HashSet,
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
    /// Recent withdrawal keys
    RecentWithdrawalKeys,
    /// Search for contract bound keys
    ContractBoundKey([u8; 32], Purpose, KeyKindRequestType),
    /// Search for contract bound keys
    ContractDocumentTypeBoundKey([u8; 32], String, Purpose, KeyKindRequestType),
    /// Get Current Authentication Master Key
    LatestAuthenticationMasterKey,
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

    IdentityPublicKey::deserialize_from_bytes(value.as_slice()).map_err(Error::from)
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
fn element_to_identity_public_key_id_and_some_object_pair(
    element: Element,
) -> Result<(KeyID, Option<IdentityPublicKey>), Error> {
    let public_key = element_to_identity_public_key(element)?;

    Ok((public_key.id(), Some(public_key)))
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
fn supported_query_result_element_to_identity_public_key_id_and_some_object_pair(
    query_result_element: QueryResultElement,
) -> Result<(KeyID, Option<IdentityPublicKey>), Error> {
    match query_result_element {
        QueryResultElement::ElementResultItem(element)
        | QueryResultElement::KeyElementPairResultItem((_, element))
        | QueryResultElement::PathKeyElementTrioResultItem((_, _, element)) => {
            element_to_identity_public_key_id_and_some_object_pair(element)
        }
    }
}

#[cfg(feature = "server")]
impl IdentityPublicKeyResult for SingleIdentityPublicKeyOutcome {
    fn try_from_path_key_optional(
        value: Vec<PathKeyOptionalElementTrio>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        // We do not care about non-existence
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
        // We do not care about non-existence
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
        // We do not care about non-existence
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
        // We do not care about non-existence
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
        // We do not care about non-existence
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
        // We do not care about non-existence
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
        // We do not care about non-existence
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
            "KeyIDOptionalIdentityPublicKeyPairVec try from QueryResultElements in IdentityPublicKeyResult",
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
        // We do not care about non-existence
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
        value: QueryResultElements,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        value
            .elements
            .into_iter()
            .map(supported_query_result_element_to_identity_public_key_id_and_some_object_pair)
            .collect()
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
    pub fn processing_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error> {
        match &self.request_type {
            AllKeys => Err(Error::Fee(FeeError::OperationNotAllowed(
                "You can not get costs for requesting all keys",
            ))),
            SpecificKeys(keys) => Ok(keys.len() as u64
                * platform_version
                    .fee_version
                    .processing
                    .fetch_single_identity_key_processing_cost),
            SearchKey(_) => Err(Error::Fee(FeeError::OperationNotAllowed(
                "You can not get costs for requesting search key",
            ))),
            ContractBoundKey(_, _, key_kind) | ContractDocumentTypeBoundKey(_, _, _, key_kind) => {
                match key_kind {
                    CurrentKeyOfKindRequest => {
                        // not accessible
                        Ok(platform_version
                            .fee_version
                            .processing
                            .fetch_single_identity_key_processing_cost)
                    }
                    AllKeysOfKindRequest => Err(Error::Fee(FeeError::OperationNotAllowed(
                        "You can not get costs for an all keys of kind request",
                    ))),
                }
            }
            KeyRequestType::RecentWithdrawalKeys => Ok(self.limit.unwrap_or(10) as Credits
                * platform_version
                    .fee_version
                    .processing
                    .fetch_single_identity_key_processing_cost),
            KeyRequestType::LatestAuthenticationMasterKey => Ok(platform_version
                .fee_version
                .processing
                .fetch_single_identity_key_processing_cost),
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
        for purpose in Purpose::searchable_purposes() {
            purpose_btree_map.insert(purpose as u8, sec_btree_map.clone());
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
    /// Make a request for a decryption key for a specific contract
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
                        query: Self::specific_keys_query(key_ids.as_slice()),
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
                        query: Self::construct_search_query(&map),
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
            KeyRequestType::RecentWithdrawalKeys => {
                let query_keys_path = identity_transfer_keys_path_vec(&identity_id);
                let mut query = Query::new_with_direction(false);
                query.insert_all();
                PathQuery {
                    path: query_keys_path,
                    query: SizedQuery {
                        query,
                        limit,
                        offset,
                    },
                }
            }
            KeyRequestType::LatestAuthenticationMasterKey => {
                let query_keys_path = identity_query_keys_security_level_tree_path_vec(
                    &identity_id,
                    SecurityLevel::MASTER,
                );
                let mut query = Query::new_with_direction(false);
                query.insert_all();
                PathQuery {
                    path: query_keys_path,
                    query: SizedQuery {
                        query,
                        limit: Some(1),
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
    fn specific_keys_query(key_ids: &[KeyID]) -> Query {
        let mut query = Query::new();
        for key_id in key_ids {
            query.insert_key(key_id.encode_var_vec());
        }
        query
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Construct the query for the request
    fn construct_search_query(
        key_requests: &BTreeMap<PurposeU8, BTreeMap<SecurityLevelU8, KeyKindRequestType>>,
    ) -> Query {
        fn construct_security_level_query(
            key_requests: &BTreeMap<SecurityLevelU8, KeyKindRequestType>,
        ) -> Query {
            let mut query = Query::new();

            for (security_level, key_request_type) in key_requests {
                let key = vec![*security_level];
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
            let key = vec![*purpose];
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
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;

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

    // --- IdentityKeysRequest constructor and path query tests ---

    #[test]
    fn test_new_all_keys_query_structure() {
        let identity_id: [u8; 32] = [1u8; 32];
        let request = IdentityKeysRequest::new_all_keys_query(&identity_id, None);

        assert_eq!(request.identity_id, identity_id);
        assert!(matches!(request.request_type, AllKeys));
        assert!(request.limit.is_none());
        assert!(request.offset.is_none());

        let path_query = request.into_path_query();
        assert_eq!(path_query.path.len(), 3);
        assert_eq!(path_query.path[1], identity_id.to_vec());
        assert!(path_query.query.limit.is_none());
    }

    #[test]
    fn test_new_all_keys_query_with_limit() {
        let identity_id: [u8; 32] = [2u8; 32];
        let request = IdentityKeysRequest::new_all_keys_query(&identity_id, Some(10));

        assert_eq!(request.limit, Some(10));

        let path_query = request.into_path_query();
        assert_eq!(path_query.query.limit, Some(10));
    }

    #[test]
    fn test_new_specific_keys_query_structure() {
        let identity_id: [u8; 32] = [3u8; 32];
        let key_ids: Vec<u32> = vec![0, 1, 2];
        let request = IdentityKeysRequest::new_specific_keys_query(&identity_id, key_ids.clone());

        assert_eq!(request.identity_id, identity_id);
        assert!(matches!(request.request_type, SpecificKeys(_)));
        assert_eq!(request.limit, Some(3));

        let path_query = request.into_path_query();
        assert_eq!(path_query.path.len(), 3);
        assert_eq!(path_query.query.limit, Some(3));
    }

    #[test]
    fn test_new_specific_keys_query_single_key() {
        let identity_id: [u8; 32] = [4u8; 32];
        let request = IdentityKeysRequest::new_specific_key_query(&identity_id, 42);

        assert_eq!(request.limit, Some(1));

        if let SpecificKeys(ref ids) = request.request_type {
            assert_eq!(ids.len(), 1);
            assert_eq!(ids[0], 42);
        } else {
            panic!("expected SpecificKeys request type");
        }

        let path_query = request.into_path_query();
        assert_eq!(path_query.query.limit, Some(1));
    }

    #[test]
    fn test_new_specific_keys_query_without_limit() {
        let identity_id: [u8; 32] = [5u8; 32];
        let request =
            IdentityKeysRequest::new_specific_keys_query_without_limit(&identity_id, vec![0, 1]);

        assert!(request.limit.is_none());

        let path_query = request.into_path_query();
        assert!(path_query.query.limit.is_none());
    }

    #[test]
    fn test_new_specific_key_query_without_limit() {
        let identity_id: [u8; 32] = [6u8; 32];
        let request = IdentityKeysRequest::new_specific_key_query_without_limit(&identity_id, 99);

        assert!(request.limit.is_none());

        if let SpecificKeys(ref ids) = request.request_type {
            assert_eq!(ids, &[99]);
        } else {
            panic!("expected SpecificKeys request type");
        }
    }

    #[test]
    fn test_new_all_current_keys_query_structure() {
        let identity_id: [u8; 32] = [7u8; 32];
        let request = IdentityKeysRequest::new_all_current_keys_query(identity_id);

        assert_eq!(request.identity_id, identity_id);
        assert!(matches!(request.request_type, SearchKey(_)));
        assert!(request.limit.is_none());

        let path_query = request.into_path_query();
        assert_eq!(path_query.path.len(), 3);
        assert_eq!(path_query.path[1], identity_id.to_vec());
    }

    #[test]
    fn test_new_contract_encryption_keys_query_structure() {
        let identity_id: [u8; 32] = [8u8; 32];
        let contract_id: [u8; 32] = [9u8; 32];
        let request =
            IdentityKeysRequest::new_contract_encryption_keys_query(identity_id, contract_id);

        assert_eq!(request.identity_id, identity_id);
        assert!(request.limit.is_none());

        if let ContractBoundKey(ref cid, ref purpose, _) = request.request_type {
            assert_eq!(cid, &contract_id);
            assert_eq!(*purpose, Purpose::ENCRYPTION);
        } else {
            panic!("expected ContractBoundKey request type");
        }

        let path_query = request.into_path_query();
        assert_eq!(path_query.query.limit, Some(1));
    }

    #[test]
    fn test_new_contract_decryption_keys_query_structure() {
        let identity_id: [u8; 32] = [10u8; 32];
        let contract_id: [u8; 32] = [11u8; 32];
        let request =
            IdentityKeysRequest::new_contract_decryption_keys_query(identity_id, contract_id);

        if let ContractBoundKey(ref cid, ref purpose, _) = request.request_type {
            assert_eq!(cid, &contract_id);
            assert_eq!(*purpose, Purpose::DECRYPTION);
        } else {
            panic!("expected ContractBoundKey request type");
        }

        let path_query = request.into_path_query();
        assert_eq!(path_query.query.limit, Some(1));
    }

    #[test]
    fn test_new_document_type_encryption_keys_query_structure() {
        let identity_id: [u8; 32] = [12u8; 32];
        let contract_id: [u8; 32] = [13u8; 32];
        let doc_type = "note".to_string();

        let request = IdentityKeysRequest::new_document_type_encryption_keys_query(
            identity_id,
            contract_id,
            doc_type.clone(),
        );

        if let ContractDocumentTypeBoundKey(ref cid, ref dt, ref purpose, _) = request.request_type
        {
            assert_eq!(cid, &contract_id);
            assert_eq!(dt, &doc_type);
            assert_eq!(*purpose, Purpose::ENCRYPTION);
        } else {
            panic!("expected ContractDocumentTypeBoundKey request type");
        }

        let path_query = request.into_path_query();
        assert_eq!(path_query.query.limit, Some(1));
    }

    #[test]
    fn test_new_document_type_decryption_keys_query_structure() {
        let identity_id: [u8; 32] = [14u8; 32];
        let contract_id: [u8; 32] = [15u8; 32];
        let doc_type = "message".to_string();

        let request = IdentityKeysRequest::new_document_type_decryption_keys_query(
            identity_id,
            contract_id,
            doc_type.clone(),
        );

        if let ContractDocumentTypeBoundKey(ref cid, ref dt, ref purpose, _) = request.request_type
        {
            assert_eq!(cid, &contract_id);
            assert_eq!(dt, &doc_type);
            assert_eq!(*purpose, Purpose::DECRYPTION);
        } else {
            panic!("expected ContractDocumentTypeBoundKey request type");
        }

        let path_query = request.into_path_query();
        assert_eq!(path_query.query.limit, Some(1));
    }

    #[test]
    fn test_into_path_query_recent_withdrawal_keys() {
        let identity_id: [u8; 32] = [16u8; 32];
        let request = IdentityKeysRequest {
            identity_id,
            request_type: KeyRequestType::RecentWithdrawalKeys,
            limit: Some(5),
            offset: None,
        };

        let path_query = request.into_path_query();
        assert_eq!(path_query.path.len(), 4);
        assert_eq!(path_query.query.limit, Some(5));
    }

    #[test]
    fn test_into_path_query_latest_authentication_master_key() {
        let identity_id: [u8; 32] = [17u8; 32];
        let request = IdentityKeysRequest {
            identity_id,
            request_type: KeyRequestType::LatestAuthenticationMasterKey,
            limit: None,
            offset: None,
        };

        let path_query = request.into_path_query();
        assert_eq!(path_query.path.len(), 5);
        assert_eq!(path_query.query.limit, Some(1));
    }

    #[test]
    fn test_into_path_query_contract_bound_key_all_keys_of_kind() {
        let identity_id: [u8; 32] = [18u8; 32];
        let contract_id: [u8; 32] = [19u8; 32];

        let request = IdentityKeysRequest {
            identity_id,
            request_type: ContractBoundKey(contract_id, Purpose::ENCRYPTION, AllKeysOfKindRequest),
            limit: None,
            offset: None,
        };

        let path_query = request.into_path_query();
        assert!(path_query.query.limit.is_none());
    }

    #[test]
    fn test_into_path_query_contract_document_type_bound_key_all_keys() {
        let identity_id: [u8; 32] = [20u8; 32];
        let contract_id: [u8; 32] = [21u8; 32];
        let doc_type = "profile".to_string();

        let request = IdentityKeysRequest {
            identity_id,
            request_type: ContractDocumentTypeBoundKey(
                contract_id,
                doc_type,
                Purpose::DECRYPTION,
                AllKeysOfKindRequest,
            ),
            limit: Some(50),
            offset: None,
        };

        let path_query = request.into_path_query();
        assert_eq!(path_query.query.limit, Some(50));
    }

    #[test]
    fn test_processing_cost_specific_keys() {
        let identity_id: [u8; 32] = [30u8; 32];
        let platform_version = PlatformVersion::latest();

        let request = IdentityKeysRequest::new_specific_keys_query(&identity_id, vec![0, 1, 2]);
        let cost = request
            .processing_cost(platform_version)
            .expect("expected cost for specific keys");

        let expected = 3u64
            * platform_version
                .fee_version
                .processing
                .fetch_single_identity_key_processing_cost;
        assert_eq!(cost, expected);
    }

    #[test]
    fn test_processing_cost_all_keys_not_allowed() {
        let identity_id: [u8; 32] = [31u8; 32];
        let platform_version = PlatformVersion::latest();

        let request = IdentityKeysRequest::new_all_keys_query(&identity_id, None);
        let result = request.processing_cost(platform_version);
        assert!(result.is_err(), "AllKeys should not allow cost calculation");
    }

    #[test]
    fn test_processing_cost_search_key_not_allowed() {
        let identity_id: [u8; 32] = [32u8; 32];
        let platform_version = PlatformVersion::latest();

        let request = IdentityKeysRequest::new_all_current_keys_query(identity_id);
        let result = request.processing_cost(platform_version);
        assert!(
            result.is_err(),
            "SearchKey should not allow cost calculation"
        );
    }

    #[test]
    fn test_processing_cost_contract_bound_current_key() {
        let identity_id: [u8; 32] = [33u8; 32];
        let contract_id: [u8; 32] = [34u8; 32];
        let platform_version = PlatformVersion::latest();

        let request =
            IdentityKeysRequest::new_contract_encryption_keys_query(identity_id, contract_id);
        let cost = request
            .processing_cost(platform_version)
            .expect("expected cost for contract bound current key");

        assert_eq!(
            cost,
            platform_version
                .fee_version
                .processing
                .fetch_single_identity_key_processing_cost
        );
    }

    #[test]
    fn test_processing_cost_contract_bound_all_keys_not_allowed() {
        let identity_id: [u8; 32] = [35u8; 32];
        let contract_id: [u8; 32] = [36u8; 32];
        let platform_version = PlatformVersion::latest();

        let request = IdentityKeysRequest {
            identity_id,
            request_type: ContractBoundKey(contract_id, Purpose::ENCRYPTION, AllKeysOfKindRequest),
            limit: None,
            offset: None,
        };
        let result = request.processing_cost(platform_version);
        assert!(
            result.is_err(),
            "AllKeysOfKindRequest should not allow cost calculation"
        );
    }

    #[test]
    fn test_processing_cost_contract_doc_type_bound_current_key() {
        let identity_id: [u8; 32] = [37u8; 32];
        let contract_id: [u8; 32] = [38u8; 32];
        let platform_version = PlatformVersion::latest();

        let request = IdentityKeysRequest::new_document_type_encryption_keys_query(
            identity_id,
            contract_id,
            "doc".to_string(),
        );
        let cost = request
            .processing_cost(platform_version)
            .expect("expected cost for doc type bound key");

        assert_eq!(
            cost,
            platform_version
                .fee_version
                .processing
                .fetch_single_identity_key_processing_cost
        );
    }

    #[test]
    fn test_processing_cost_contract_doc_type_bound_all_keys_not_allowed() {
        let identity_id: [u8; 32] = [39u8; 32];
        let contract_id: [u8; 32] = [40u8; 32];
        let platform_version = PlatformVersion::latest();

        let request = IdentityKeysRequest {
            identity_id,
            request_type: ContractDocumentTypeBoundKey(
                contract_id,
                "doc".to_string(),
                Purpose::ENCRYPTION,
                AllKeysOfKindRequest,
            ),
            limit: None,
            offset: None,
        };
        let result = request.processing_cost(platform_version);
        assert!(
            result.is_err(),
            "AllKeysOfKindRequest on doc-type bound should not allow cost"
        );
    }

    #[test]
    fn test_processing_cost_recent_withdrawal_keys() {
        let identity_id: [u8; 32] = [41u8; 32];
        let platform_version = PlatformVersion::latest();

        let request = IdentityKeysRequest {
            identity_id,
            request_type: KeyRequestType::RecentWithdrawalKeys,
            limit: Some(3),
            offset: None,
        };
        let cost = request
            .processing_cost(platform_version)
            .expect("expected cost for recent withdrawal keys");

        assert_eq!(
            cost,
            3u64 * platform_version
                .fee_version
                .processing
                .fetch_single_identity_key_processing_cost
        );
    }

    #[test]
    fn test_processing_cost_recent_withdrawal_keys_default_limit() {
        let identity_id: [u8; 32] = [42u8; 32];
        let platform_version = PlatformVersion::latest();

        let request = IdentityKeysRequest {
            identity_id,
            request_type: KeyRequestType::RecentWithdrawalKeys,
            limit: None,
            offset: None,
        };
        let cost = request
            .processing_cost(platform_version)
            .expect("expected cost for recent withdrawal keys with default limit");

        assert_eq!(
            cost,
            10u64
                * platform_version
                    .fee_version
                    .processing
                    .fetch_single_identity_key_processing_cost
        );
    }

    #[test]
    fn test_processing_cost_latest_authentication_master_key() {
        let identity_id: [u8; 32] = [43u8; 32];
        let platform_version = PlatformVersion::latest();

        let request = IdentityKeysRequest {
            identity_id,
            request_type: KeyRequestType::LatestAuthenticationMasterKey,
            limit: None,
            offset: None,
        };
        let cost = request
            .processing_cost(platform_version)
            .expect("expected cost for latest auth master key");

        assert_eq!(
            cost,
            platform_version
                .fee_version
                .processing
                .fetch_single_identity_key_processing_cost
        );
    }

    // --- Helper function tests ---

    #[test]
    fn test_element_to_identity_public_key_with_valid_item() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::serialization::PlatformSerializable;
        use rand::SeedableRng;

        let platform_version = PlatformVersion::latest();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let (key, _) = IdentityPublicKeyV0::random_ecdsa_master_authentication_key_with_rng(
            1,
            &mut rng,
            platform_version,
        )
        .expect("expected a random key");
        let key: dpp::identity::IdentityPublicKey = key.into();
        let serialized = key.serialize_to_bytes().expect("expected to serialize key");

        let element = Item(serialized, None);
        let result = element_to_identity_public_key(element);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), key);
    }

    #[test]
    fn test_element_to_identity_public_key_with_non_item_element() {
        let element = Element::empty_tree();
        let result = element_to_identity_public_key(element);
        assert!(result.is_err());
    }

    #[test]
    fn test_element_to_identity_public_key_id_with_valid_item() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::serialization::PlatformSerializable;
        use rand::SeedableRng;

        let platform_version = PlatformVersion::latest();
        let mut rng = rand::rngs::StdRng::seed_from_u64(99);
        let (key, _) = IdentityPublicKeyV0::random_ecdsa_master_authentication_key_with_rng(
            5,
            &mut rng,
            platform_version,
        )
        .expect("expected a random key");
        let key: dpp::identity::IdentityPublicKey = key.into();
        let serialized = key.serialize_to_bytes().expect("expected to serialize key");

        let element = Item(serialized, None);
        let result = element_to_identity_public_key_id(element);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5u32);
    }

    #[test]
    fn test_element_to_serialized_identity_public_key_valid() {
        let data = vec![1, 2, 3, 4, 5];
        let element = Item(data.clone(), None);
        let result = element_to_serialized_identity_public_key(element);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data);
    }

    #[test]
    fn test_element_to_serialized_identity_public_key_non_item() {
        let element = Element::empty_tree();
        let result = element_to_serialized_identity_public_key(element);
        assert!(result.is_err());
    }

    #[test]
    fn test_element_to_identity_public_key_id_and_object_pair() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::serialization::PlatformSerializable;
        use rand::SeedableRng;

        let platform_version = PlatformVersion::latest();
        let mut rng = rand::rngs::StdRng::seed_from_u64(123);
        let (key, _) = IdentityPublicKeyV0::random_ecdsa_master_authentication_key_with_rng(
            7,
            &mut rng,
            platform_version,
        )
        .expect("expected a random key");
        let key: dpp::identity::IdentityPublicKey = key.into();
        let serialized = key.serialize_to_bytes().expect("expected to serialize key");

        let element = Item(serialized, None);
        let result = element_to_identity_public_key_id_and_object_pair(element);
        assert!(result.is_ok());
        let (id, pk) = result.unwrap();
        assert_eq!(id, 7u32);
        assert_eq!(pk, key);
    }

    #[test]
    fn test_element_to_identity_public_key_id_and_some_object_pair() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::serialization::PlatformSerializable;
        use rand::SeedableRng;

        let platform_version = PlatformVersion::latest();
        let mut rng = rand::rngs::StdRng::seed_from_u64(456);
        let (key, _) = IdentityPublicKeyV0::random_ecdsa_master_authentication_key_with_rng(
            3,
            &mut rng,
            platform_version,
        )
        .expect("expected a random key");
        let key: dpp::identity::IdentityPublicKey = key.into();
        let serialized = key.serialize_to_bytes().expect("expected to serialize key");

        let element = Item(serialized, None);
        let result = element_to_identity_public_key_id_and_some_object_pair(element);
        assert!(result.is_ok());
        let (id, maybe_pk) = result.unwrap();
        assert_eq!(id, 3u32);
        assert!(maybe_pk.is_some());
        assert_eq!(maybe_pk.unwrap(), key);
    }

    #[test]
    fn test_key_and_optional_element_to_pair_with_element() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::serialization::PlatformSerializable;
        use rand::SeedableRng;

        let platform_version = PlatformVersion::latest();
        let mut rng = rand::rngs::StdRng::seed_from_u64(789);
        let (key, _) = IdentityPublicKeyV0::random_ecdsa_master_authentication_key_with_rng(
            10,
            &mut rng,
            platform_version,
        )
        .expect("expected a random key");
        let key: dpp::identity::IdentityPublicKey = key.into();
        let serialized = key.serialize_to_bytes().expect("expected to serialize key");

        let element = Item(serialized, None);
        let path: Vec<Vec<u8>> = vec![vec![1]];
        let encoded_key = 10u32.encode_var_vec();
        let trio = (path, encoded_key, Some(element));

        let result = key_and_optional_element_to_identity_public_key_id_and_object_pair(trio);
        assert!(result.is_ok());
        let (id, maybe_pk) = result.unwrap();
        assert_eq!(id, 10u32);
        assert!(maybe_pk.is_some());
    }

    #[test]
    fn test_key_and_optional_element_to_pair_without_element() {
        use integer_encoding::VarInt;

        let path: Vec<Vec<u8>> = vec![vec![1]];
        let encoded_key = 42u32.encode_var_vec();
        let trio = (path, encoded_key, None);

        let result = key_and_optional_element_to_identity_public_key_id_and_object_pair(trio);
        assert!(result.is_ok());
        let (id, maybe_pk) = result.unwrap();
        assert_eq!(id, 42u32);
        assert!(maybe_pk.is_none());
    }

    // --- IdentityPublicKeyResult trait impls tests ---

    #[test]
    fn test_key_vec_try_from_path_key_optional_with_elements() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::serialization::PlatformSerializable;
        use rand::SeedableRng;

        let platform_version = PlatformVersion::latest();
        let mut rng = rand::rngs::StdRng::seed_from_u64(100);

        let (key1, _) = IdentityPublicKeyV0::random_ecdsa_master_authentication_key_with_rng(
            0,
            &mut rng,
            platform_version,
        )
        .expect("expected a random key");
        let key1: dpp::identity::IdentityPublicKey = key1.into();
        let serialized1 = key1.serialize_to_bytes().expect("serialize");

        let (key2, _) = IdentityPublicKeyV0::random_ecdsa_master_authentication_key_with_rng(
            1,
            &mut rng,
            platform_version,
        )
        .expect("expected a random key");
        let key2: dpp::identity::IdentityPublicKey = key2.into();
        let serialized2 = key2.serialize_to_bytes().expect("serialize");

        let trios: Vec<PathKeyOptionalElementTrio> = vec![
            (vec![vec![1]], vec![0], Some(Item(serialized1, None))),
            (vec![vec![1]], vec![1], Some(Item(serialized2, None))),
            (vec![vec![1]], vec![2], None),
        ];

        let result = KeyVec::try_from_path_key_optional(trios, platform_version);
        assert!(result.is_ok());
        let keys = result.unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_key_id_vec_try_from_path_key_optional_empty() {
        let platform_version = PlatformVersion::latest();
        let trios: Vec<PathKeyOptionalElementTrio> = vec![];

        let result = KeyIDVec::try_from_path_key_optional(trios, platform_version);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_single_key_try_from_path_key_optional_empty_returns_error() {
        let platform_version = PlatformVersion::latest();
        let trios: Vec<PathKeyOptionalElementTrio> = vec![];

        let result =
            SingleIdentityPublicKeyOutcome::try_from_path_key_optional(trios, platform_version);
        assert!(result.is_err());
    }

    #[test]
    fn test_optional_single_key_try_from_path_key_optional_empty_returns_none() {
        let platform_version = PlatformVersion::latest();
        let trios: Vec<PathKeyOptionalElementTrio> = vec![];

        let result = OptionalSingleIdentityPublicKeyOutcome::try_from_path_key_optional(
            trios,
            platform_version,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_key_id_optional_pair_vec_try_from_query_results_not_supported() {
        use grovedb::query_result_type::QueryResultElements;

        let platform_version = PlatformVersion::latest();
        let elements = QueryResultElements { elements: vec![] };

        let result = KeyIDOptionalIdentityPublicKeyPairVec::try_from_query_results(
            elements,
            platform_version,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_query_path_trio_vec_try_from_query_results_not_supported() {
        use grovedb::query_result_type::QueryResultElements;

        let platform_version = PlatformVersion::latest();
        let elements = QueryResultElements { elements: vec![] };

        let result = QueryKeyPathOptionalIdentityPublicKeyTrioVec::try_from_query_results(
            elements,
            platform_version,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_query_path_trio_btree_map_try_from_query_results_not_supported() {
        use grovedb::query_result_type::QueryResultElements;

        let platform_version = PlatformVersion::latest();
        let elements = QueryResultElements { elements: vec![] };

        let result = QueryKeyPathOptionalIdentityPublicKeyTrioBTreeMap::try_from_query_results(
            elements,
            platform_version,
        );
        assert!(result.is_err());
    }

    // --- Integration tests that exercise fetch through drive ---

    #[test]
    fn test_fetch_identity_keys_as_key_id_hash_set() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(77777), platform_version)
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
            request_type: SpecificKeys(vec![0, 1]),
            limit: Some(2),
            offset: None,
        };

        let key_ids: KeyIDHashSet = drive
            .fetch_identity_keys(key_request, Some(&transaction), platform_version)
            .expect("expected to fetch key ids");

        assert_eq!(key_ids.len(), 2);
        assert!(key_ids.contains(&0));
        assert!(key_ids.contains(&1));
    }

    #[test]
    fn test_fetch_identity_keys_as_key_id_vec() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(88888), platform_version)
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

        let key_ids: KeyIDVec = drive
            .fetch_identity_keys(key_request, Some(&transaction), platform_version)
            .expect("expected to fetch key id vec");

        assert_eq!(key_ids.len(), 1);
    }

    #[test]
    fn test_fetch_identity_keys_as_serialized_key_vec() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(99999), platform_version)
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

        let serialized_keys: SerializedKeyVec = drive
            .fetch_identity_keys(key_request, Some(&transaction), platform_version)
            .expect("expected to fetch serialized keys");

        assert_eq!(serialized_keys.len(), 1);
        assert!(!serialized_keys[0].is_empty());
    }
}
