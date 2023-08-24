use std::collections::BTreeMap;

use crate::Error;
use dapi_grpc::platform::v0::get_identities_keys_request::security_level_map::KeyKindRequestType as GrpcKeyKind;
use dapi_grpc::platform::v0::{self as platform, key_request_type, KeyRequestType as GrpcKeyType};
use dpp::document::Document;
use dpp::identity::KeyID;
use dpp::prelude::{DataContract, Identifier, Identity, IdentityPublicKey, Revision};
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyKindRequestType, KeyRequestType, PurposeU8, SecurityLevelU8,
};
pub use drive::drive::verify::RootHash;
use drive::drive::Drive;
use drive::query::DriveQuery;

use super::verify::verify_tenderdash_proof;

pub type DataContractHistory = BTreeMap<u64, DataContract>;
pub type DataContracts = BTreeMap<[u8; 32], Option<DataContract>>;
pub type IdentityBalance = u64;
pub type IdentityBalanceAndRevision = (u64, Revision);
pub type IdentityPublicKeys = BTreeMap<KeyID, Option<IdentityPublicKey>>;
pub type Documents = Vec<Document>;

lazy_static::lazy_static! {
    pub static ref PLATFORM_VERSION: PlatformVersion = PlatformVersion::latest().to_owned();
}

/// Create an object based on proof received from DAPI
pub trait FromProof<Req> {
    type Response;
    /// Parse and verify the received proof and retrieve the requested object, if any.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `provider`: A callback implementing [QuorumInfoProvider] that provides quorum details required to verify the proof.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(object))` when the requested object was found in the proof.
    /// * `Ok(None)` when the requested object was not found in the proof; this can be interpreted as proof of non-existence.
    /// For collections, returns Ok(None) if none of the requested objects were found.
    /// * `Err(Error)` when either the provided data is invalid or proof validation failed.
    fn maybe_from_proof(
        request: &Req,
        response: &Self::Response,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error>
    where
        Self: Sized;

    /// Retrieve the requested object from the proof.
    ///
    /// Runs full verification of the proof and retrieves enclosed objects.
    ///
    /// This method uses [`maybe_from_proof()`] internally and throws an error if the requested object does not exist in the proof.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `provider`: A callback implementing [QuorumInfoProvider] that provides quorum details required to verify the proof.
    ///
    /// # Returns
    ///
    /// * `Ok(object)` when the requested object was found in the proof.
    /// * `Err(Error::DocumentMissingInProof)` when the requested object was not found in the proof.
    /// * `Err(Error)` when either the provided data is invalid or proof validation failed.
    fn from_proof(
        request: &Req,
        response: &Self::Response,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::maybe_from_proof(request, response, provider)?.ok_or(Error::DocumentMissingInProof)
    }
}

/// `QuorumInfoProvider` trait provides an interface to fetch quorum related information, required to verify the proof.
///
/// Developers should implement this trait to provide required quorum details to [FromProof] implementations.
///
/// It defines a single method `get_quorum_public_key` which retrieves the public key of a given quorum.
#[cfg_attr(feature = "uniffi", uniffi::export(callback_interface))]
#[cfg_attr(feature = "mock", mockall::automock)]
pub trait QuorumInfoProvider: Send + Sync {
    /// Fetches the public key for a specified quorum.
    ///
    /// # Arguments
    ///
    /// * `quorum_type`: The type of the quorum.
    /// * `quorum_hash`: The hash of the quorum. This is used to determine which quorum's public key to fetch.
    /// * `core_chain_locked_height`: Core chain locked height for which the quorum must be valid
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)`: On success, returns a byte vector representing the public key of the quorum.
    /// * `Err(Error)`: On failure, returns an error indicating why the operation failed.
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: Vec<u8>, // TODO: once we get rid of uniffi, we should take [u8;32] here
        core_chain_locked_height: u32,
    ) -> Result<Vec<u8>, Error>; // TODO: When we get rid of uniffi, we should return 48 bytes instead of Vec
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetIdentityRequest> for Identity {
    type Response = platform::GetIdentityResponse;

    fn maybe_from_proof(
        request: &platform::GetIdentityRequest,
        response: &platform::GetIdentityResponse,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error> {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::NoResultInResponse)? {
            platform::get_identity_response::Result::Proof(p) => p,
            platform::get_identity_response::Result::Identity(_) => {
                return Err(Error::NoProofInResult)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        // Load some info from request
        let id = Identifier::from_bytes(&request.id).map_err(|e| Error::ProtocolError {
            error: e.to_string(),
        })?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) = Drive::verify_full_identity_by_identity_id(
            &proof.grovedb_proof,
            false,
            id.into_buffer(),
            &PLATFORM_VERSION,
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_identity)
    }
}

// TODO: figure out how to deal with mock::automock
impl FromProof<platform::GetIdentityByPublicKeyHashesRequest> for Identity {
    type Response = platform::GetIdentityByPublicKeyHashesResponse;

    fn maybe_from_proof(
        request: &platform::GetIdentityByPublicKeyHashesRequest,
        response: &platform::GetIdentityByPublicKeyHashesResponse,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error> {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::NoResultInResponse)? {
            platform::get_identity_by_public_key_hashes_response::Result::Proof(p) => p,
            platform::get_identity_by_public_key_hashes_response::Result::Identity(_) => {
                return Err(Error::NoProofInResult)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        // Load some info from request
        let public_key_hash: [u8; 20] =
            request
                .public_key_hash
                .clone()
                .try_into()
                .map_err(|_| Error::DriveError {
                    error: "Ivalid public key hash length".to_string(),
                })?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) = Drive::verify_full_identity_by_public_key_hash(
            &proof.grovedb_proof,
            public_key_hash,
            &PLATFORM_VERSION,
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_identity)
    }
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetIdentityKeysRequest> for IdentityPublicKeys {
    type Response = platform::GetIdentityKeysResponse;

    fn maybe_from_proof(
        request: &platform::GetIdentityKeysRequest,
        response: &Self::Response,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error> {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::NoResultInResponse)? {
            platform::get_identity_keys_response::Result::Proof(p) => p,
            platform::get_identity_keys_response::Result::Keys(_) => {
                return Err(Error::NoProofInResult)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        // Load some info from request
        let identity_id = Identifier::from_bytes(&request.identity_id)
            .map_err(|e| Error::ProtocolError {
                error: e.to_string(),
            })?
            .into_buffer();

        let key_request = match parse_key_request_type(&request.request_type)? {
            KeyRequestType::SpecificKeys(specific_keys) => {
                IdentityKeysRequest::new_specific_keys_query(&identity_id, specific_keys)
            }
            KeyRequestType::AllKeys => IdentityKeysRequest::new_all_keys_query(&identity_id, None),
            KeyRequestType::SearchKey(criteria) => IdentityKeysRequest {
                identity_id,
                request_type: KeyRequestType::SearchKey(criteria),
                limit: Some(1),
                offset: None,
            },
        };

        tracing::debug!(?identity_id, "checking proof of identity keys");

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) =
            Drive::verify_identity_keys_by_identity_id(&proof.grovedb_proof, key_request, false)
                .map_err(|e| Error::DriveError {
                    error: e.to_string(),
                })?;

        let maybe_keys: Option<IdentityPublicKeys> = if let Some(identity) = maybe_identity {
            if identity.loaded_public_keys.is_empty() {
                None
            } else {
                let mut keys = identity
                    .loaded_public_keys
                    .into_iter()
                    .map(|(k, v)| (k, Some(v.clone())))
                    .collect::<IdentityPublicKeys>();

                let mut not_found = identity
                    .not_found_public_keys
                    .into_iter()
                    .map(|k| (k, None))
                    .collect::<IdentityPublicKeys>();

                keys.append(&mut not_found);

                Some(keys)
            }
        } else {
            None
        };

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_keys)
    }
}

fn parse_key_request_type(request: &Option<GrpcKeyType>) -> Result<KeyRequestType, Error> {
    let key_request_type = request
        .to_owned()
        .ok_or(Error::RequestDecodeError {
            error: "missing key request type".to_string(),
        })?
        .request
        .ok_or(Error::RequestDecodeError {
            error: "empty request field in key request type".to_string(),
        })?;

    let request_type = match key_request_type {
        key_request_type::Request::AllKeys(_) => KeyRequestType::AllKeys,
        key_request_type::Request::SpecificKeys(specific_keys) => {
            KeyRequestType::SpecificKeys(specific_keys.key_ids)
        }
        key_request_type::Request::SearchKey(search_key) => {
            let purpose = search_key
                .purpose_map
                .iter()
                .map(|(k, v)| {
                    let v=  v.security_level_map
                            .iter()
                            .map(|(level, kind)| {
                                let kt = match GrpcKeyKind::from_i32(*kind) {
                                    Some(GrpcKeyKind::CurrentKeyOfKindRequest) => {
                                        Ok(KeyKindRequestType::CurrentKeyOfKindRequest)
                                    }
                                    None => Err(Error::RequestDecodeError {
                                        error: format!("missing requested key type: {}", *kind),
                                    }),
                                };

                                match kt  {
                                    Err(e) => Err(e),
                                    Ok(d) => Ok((*level as u8, d))
                                }
                            })
                            .collect::<Result<BTreeMap<SecurityLevelU8,KeyKindRequestType>,Error>>();

                            match v {
                                Err(e) =>Err(e),
                                Ok(d) => Ok(((*k as u8),d)),
                            }
                })
                .collect::<Result<BTreeMap<PurposeU8, BTreeMap<SecurityLevelU8, KeyKindRequestType>>,Error>>()?;

            KeyRequestType::SearchKey(purpose)
        }
    };

    Ok(request_type)
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetIdentityRequest> for IdentityBalance {
    type Response = platform::GetIdentityBalanceResponse;

    fn maybe_from_proof(
        request: &platform::GetIdentityRequest,
        response: &Self::Response,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error> {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::NoResultInResponse)? {
            platform::get_identity_balance_response::Result::Proof(p) => p,
            platform::get_identity_balance_response::Result::Balance(_) => {
                return Err(Error::NoProofInResult)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        // Load some info from request
        let id = Identifier::from_bytes(&request.id).map_err(|e| Error::ProtocolError {
            error: e.to_string(),
        })?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) = Drive::verify_identity_balance_for_identity_id(
            &proof.grovedb_proof,
            id.into_buffer(),
            false,
            &PLATFORM_VERSION,
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_identity)
    }
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetIdentityRequest> for IdentityBalanceAndRevision {
    type Response = platform::GetIdentityBalanceAndRevisionResponse;

    fn maybe_from_proof(
        request: &platform::GetIdentityRequest,
        response: &Self::Response,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error> {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::NoResultInResponse)? {
            platform::get_identity_balance_and_revision_response::Result::Proof(p) => p,
            platform::get_identity_balance_and_revision_response::Result::BalanceAndRevision(_) => {
                return Err(Error::NoProofInResult)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        // Load some info from request
        let id = Identifier::from_bytes(&request.id).map_err(|e| Error::ProtocolError {
            error: e.to_string(),
        })?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) =
            Drive::verify_identity_balance_and_revision_for_identity_id(
                &proof.grovedb_proof,
                id.into_buffer(),
                false,
            )
            .map_err(|e| Error::DriveError {
                error: e.to_string(),
            })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_identity)
    }
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetDataContractRequest> for DataContract {
    type Response = platform::GetDataContractResponse;

    fn maybe_from_proof(
        request: &platform::GetDataContractRequest,
        response: &Self::Response,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error> {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::NoResultInResponse)? {
            platform::get_data_contract_response::Result::Proof(p) => p,
            platform::get_data_contract_response::Result::DataContract(_) => {
                return Err(Error::NoProofInResult)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        // Load some info from request
        let id = Identifier::from_bytes(&request.id).map_err(|e| Error::ProtocolError {
            error: e.to_string(),
        })?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_contract) = Drive::verify_contract(
            &proof.grovedb_proof,
            None,
            false,
            id.into_buffer(),
            &PLATFORM_VERSION,
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_contract)
    }
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetDataContractsRequest> for DataContracts {
    type Response = platform::GetDataContractsResponse;

    fn maybe_from_proof(
        request: &platform::GetDataContractsRequest,
        response: &Self::Response,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error> {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::NoResultInResponse)? {
            platform::get_data_contracts_response::Result::Proof(p) => p,
            platform::get_data_contracts_response::Result::DataContracts(_) => {
                return Err(Error::NoProofInResult)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        // Load some info from request
        let ids = request
            .ids
            .iter()
            .map(|id| {
                id.clone()
                    .try_into()
                    .map_err(|_e| Error::RequestDecodeError {
                        error: format!("wrong id size: expected: {}, got: {}", 32, id.len()),
                    })
            })
            .collect::<Result<Vec<[u8; 32]>, Error>>()?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, contracts) = Drive::verify_contracts(
            &proof.grovedb_proof,
            false,
            ids.as_slice(),
            &PLATFORM_VERSION,
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        let maybe_contracts = if contracts.count_some() > 0 {
            Some(contracts)
        } else {
            None
        };

        Ok(maybe_contracts)
    }
}

impl FromProof<platform::GetDataContractHistoryRequest> for DataContractHistory {
    type Response = platform::GetDataContractHistoryResponse;

    fn maybe_from_proof(
        request: &platform::GetDataContractHistoryRequest,
        response: &Self::Response,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error>
    where
        Self: Sized,
    {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::NoResultInResponse)? {
            platform::get_data_contract_history_response::Result::Proof(p) => p,
            platform::get_data_contract_history_response::Result::DataContractHistory(_) => {
                return Err(Error::NoProofInResult)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        // Load some info from request
        let id = Identifier::from_bytes(&request.id).map_err(|e| Error::ProtocolError {
            error: e.to_string(),
        })?;

        let limit = u32_to_u16_opt(request.limit.unwrap_or_default())?;
        let offset = u32_to_u16_opt(request.offset.unwrap_or_default())?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_history) = Drive::verify_contract_history(
            &proof.grovedb_proof,
            id.into_buffer(),
            request.start_at_ms,
            limit,
            offset,
            &PLATFORM_VERSION,
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_history)
    }
}

// #[cfg_attr(feature = "mock", mockall::automock)]
impl<'dq> FromProof<DriveQuery<'dq>> for Documents {
    type Response = platform::GetDocumentsResponse;

    fn maybe_from_proof(
        request: &DriveQuery<'dq>,
        response: &Self::Response,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error> {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::NoResultInResponse)? {
            platform::get_documents_response::Result::Proof(p) => p,
            platform::get_documents_response::Result::Documents(_) => {
                return Err(Error::NoProofInResult)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        let (root_hash, documents) = request
            .verify_proof(&proof.grovedb_proof, &PLATFORM_VERSION)
            .map_err(|e| Error::DriveError {
                error: e.to_string(),
            })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        if documents.is_empty() {
            Ok(None)
        } else {
            Ok(Some(documents))
        }
    }
}

/// Convert u32, if 0 return None, otherwise return Some(u16).
/// Errors when value is out of range.
fn u32_to_u16_opt(i: u32) -> Result<Option<u16>, Error> {
    let i: Option<u16> = if i != 0 {
        let i: u16 =
            i.try_into()
                .map_err(|e: std::num::TryFromIntError| Error::RequestDecodeError {
                    error: format!("value {} out of range: {}", i, e.to_string()),
                })?;
        Some(i)
    } else {
        None
    };

    Ok(i)
}

/// Determine number of non-None elements
pub trait Length {
    /// Return number of non-None elements in the data structure
    fn count_some(&self) -> usize;
}

impl<T: Length> Length for Option<T> {
    fn count_some(&self) -> usize {
        match self {
            None => 0,
            Some(i) => i.count_some(),
        }
    }
}

impl<T> Length for Vec<Option<T>> {
    fn count_some(&self) -> usize {
        self.into_iter().filter(|v| v.is_some()).count()
    }
}

impl<K, T> Length for Vec<(K, Option<T>)> {
    fn count_some(&self) -> usize {
        self.into_iter().filter(|(_, v)| v.is_some()).count()
    }
}

impl<K, T> Length for BTreeMap<K, Option<T>> {
    fn count_some(&self) -> usize {
        self.into_iter().filter(|(_, v)| v.is_some()).count()
    }
}
