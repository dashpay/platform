use std::collections::BTreeMap;

use crate::Error;
use dapi_grpc::platform::v0::{self as platform};
use dpp::document::Document;
use dpp::identity::PartialIdentity;
use dpp::prelude::{DataContract, Identifier, Identity, Revision};
pub use drive::drive::verify::RootHash;
use drive::drive::Drive;
use drive::query::DriveQuery;

use super::verify::verify_tenderdash_proof;

pub type Identities = Vec<Option<Identity>>;
pub type IdentitiesByPublicKeyHashes = Vec<([u8; 20], Option<Identity>)>;
pub type DataContractHistory = BTreeMap<u64, DataContract>;
pub type DataContracts = Vec<Option<DataContract>>;
pub type IdentityBalance = u64;
pub type IdentityBalanceAndRevision = (u64, Revision);
pub type Documents = Vec<Document>;

// #[cfg(feature = "mockall")]

/// Create an object based on proof received from DAPI
pub trait FromProof<Req, Resp> {
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
    /// * `Err(Error)` when either the provided data is invalid or proof validation failed.
    fn maybe_from_proof(
        request: &Req,
        response: &Resp,
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
        response: &Resp,
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
impl FromProof<platform::GetIdentityRequest, platform::GetIdentityResponse> for Identity {
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
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_identity)
    }
}

// TODO: figure out how to deal with mock::automock
impl
    FromProof<
        platform::GetIdentityByPublicKeyHashesRequest,
        platform::GetIdentityByPublicKeyHashesResponse,
    > for Identity
{
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
        let (root_hash, maybe_identity) =
            Drive::verify_full_identity_by_public_key_hash(&proof.grovedb_proof, public_key_hash)
                .map_err(|e| Error::DriveError {
                error: e.to_string(),
            })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_identity)
    }
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetIdentitiesRequest, platform::GetIdentitiesResponse> for Identities {
    fn maybe_from_proof(
        request: &platform::GetIdentitiesRequest,
        response: &platform::GetIdentitiesResponse,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error> {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::NoResultInResponse)? {
            platform::get_identities_response::Result::Proof(p) => p,
            platform::get_identities_response::Result::Identities(_) => {
                return Err(Error::NoProofInResult)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        let identity_ids = request
            .ids
            .iter()
            .map(|id| {
                Identifier::from_bytes(id).map_err(|e| Error::ProtocolError {
                    error: e.to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let maybe_identities = identity_ids
            .iter()
            .map(|id| {
                // Extract content from proof and verify Drive/GroveDB proofs
                let (root_hash, maybe_identity) = Drive::verify_full_identity_by_identity_id(
                    &proof.grovedb_proof,
                    false,
                    id.into_buffer(),
                )
                .map_err(|e| Error::DriveError {
                    error: e.to_string(),
                })?;

                verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

                Ok(maybe_identity)
            })
            .collect::<Result<Identities, Error>>()?;

        todo!("We need verify_full_identities_by_identity_ids in Drive to implement this method");

        Ok(Some(maybe_identities))
    }
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetIdentityKeysRequest, platform::GetIdentityKeysResponse>
    for PartialIdentity
{
    fn maybe_from_proof(
        request: &platform::GetIdentityKeysRequest,
        response: &platform::GetIdentityKeysResponse,
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
        let id =
            Identifier::from_bytes(&request.identity_id).map_err(|e| Error::ProtocolError {
                error: e.to_string(),
            })?;

        tracing::debug!(?id, "checking proof of identity keys");

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) = Drive::verify_identity_keys_by_identity_id(
            &proof.grovedb_proof,
            false,
            id.into_buffer(),
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_identity)
    }
}

// TODO: figure out how to deal with mock::automock
impl
    FromProof<
        platform::GetIdentitiesByPublicKeyHashesRequest,
        platform::GetIdentitiesByPublicKeyHashesResponse,
    > for IdentitiesByPublicKeyHashes
{
    fn maybe_from_proof(
        request: &platform::GetIdentitiesByPublicKeyHashesRequest,
        response: &platform::GetIdentitiesByPublicKeyHashesResponse,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error> {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::NoResultInResponse)? {
            platform::get_identities_by_public_key_hashes_response::Result::Proof(p) => p,
            platform::get_identities_by_public_key_hashes_response::Result::Identities(_) => {
                return Err(Error::NoProofInResult)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        let identity_public_key_hashes = request
            .public_key_hashes
            .iter()
            .map(|pk_hash| {
                pk_hash.to_vec().try_into().map_err(|_| Error::DriveError {
                    error: "Invalid public key hash length".to_string(),
                })
            })
            .collect::<Result<Vec<[u8; 20]>, Error>>()?;

        let (root_hash, maybe_identities_with_public_key_hashes) =
            Drive::verify_full_identities_by_public_key_hashes(
                &proof.grovedb_proof,
                &identity_public_key_hashes,
            )
            .map_err(|e| Error::DriveError {
                error: e.to_string(),
            })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(Some(maybe_identities_with_public_key_hashes))
    }
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetIdentityRequest, platform::GetIdentityBalanceResponse>
    for IdentityBalance
{
    fn maybe_from_proof(
        request: &platform::GetIdentityRequest,
        response: &platform::GetIdentityBalanceResponse,
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
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_identity)
    }
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetIdentityRequest, platform::GetIdentityBalanceAndRevisionResponse>
    for IdentityBalanceAndRevision
{
    fn maybe_from_proof(
        request: &platform::GetIdentityRequest,
        response: &platform::GetIdentityBalanceAndRevisionResponse,
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
        let (root_hash, maybe_identity) = Drive::verify_full_identity_by_identity_id(
            &proof.grovedb_proof,
            false,
            id.into_buffer(),
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        todo!("we need Drive to implement verify_identity_balance_and_revision_for_identity_id");
        #[allow(unreachable_code)]
        Ok(maybe_identity.map(|i| (i.balance, i.revision)))
    }
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetDataContractRequest, platform::GetDataContractResponse>
    for DataContract
{
    fn maybe_from_proof(
        request: &platform::GetDataContractRequest,
        response: &platform::GetDataContractResponse,
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
        let (root_hash, maybe_contract) =
            Drive::verify_contract(&proof.grovedb_proof, None, false, id.into_buffer()).map_err(
                |e| Error::DriveError {
                    error: e.to_string(),
                },
            )?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_contract)
    }
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetDataContractsRequest, platform::GetDataContractsResponse>
    for DataContracts
{
    fn maybe_from_proof(
        request: &platform::GetDataContractsRequest,
        response: &platform::GetDataContractsResponse,
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

        let contract_ids = request
            .ids
            .iter()
            .map(|id| {
                Identifier::from_bytes(id).map_err(|e| Error::ProtocolError {
                    error: e.to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let maybe_contracts = contract_ids
            .iter()
            .map(|id| {
                // Extract content from proof and verify Drive/GroveDB proofs
                let (root_hash, maybe_contract) =
                    Drive::verify_contract(&proof.grovedb_proof, None, false, id.into_buffer())
                        .map_err(|e| Error::DriveError {
                            error: e.to_string(),
                        })?;

                verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

                Ok(maybe_contract)
            })
            .collect::<Result<DataContracts, Error>>()?;

        Ok(Some(maybe_contracts))
    }
}

impl FromProof<platform::GetDataContractHistoryRequest, platform::GetDataContractHistoryResponse>
    for DataContractHistory
{
    fn maybe_from_proof(
        request: &platform::GetDataContractHistoryRequest,
        response: &platform::GetDataContractHistoryResponse,
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

        let limit = u32_to_u16_opt(request.limit)?;
        let offset = u32_to_u16_opt(request.offset)?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_history) = Drive::verify_contract_history(
            &proof.grovedb_proof,
            id.into_buffer(),
            request.start_at_ms,
            limit,
            offset,
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, &provider)?;

        Ok(maybe_history)
    }
}

// #[cfg_attr(feature = "mock", mockall::automock)]
impl<'dq> FromProof<DriveQuery<'dq>, platform::GetDocumentsResponse> for Documents {
    fn maybe_from_proof(
        request: &DriveQuery<'dq>,
        response: &platform::GetDocumentsResponse,
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

        let (root_hash, documents) =
            request
                .verify_proof(&proof.grovedb_proof)
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
