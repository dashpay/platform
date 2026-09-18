//! Proof verification of the contract moderation queries.

use crate::error::MapGroveDbError;
use crate::types::contract_moderation::{
    entries_query_from_request, lists_from_request, ContractModerationEntries,
    ContractModerationStatus,
};
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_contract_moderation_entries_request, get_contract_moderation_status_request,
    GetContractModerationEntriesRequest, GetContractModerationEntriesResponse,
    GetContractModerationStatusRequest, GetContractModerationStatusResponse, Proof,
    ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

fn identifier_from_request(bytes: Vec<u8>, what: &str) -> Result<Identifier, Error> {
    Identifier::from_bytes(&bytes).map_err(|_| Error::RequestError {
        error: format!(
            "{what} must be a 32 byte identifier, got {} bytes",
            bytes.len()
        ),
    })
}

impl FromProof<GetContractModerationStatusRequest> for ContractModerationStatus {
    type Request = GetContractModerationStatusRequest;
    type Response = GetContractModerationStatusResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let get_contract_moderation_status_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_id = identifier_from_request(v0.contract_id, "contract_id")?;
        let identity_id = identifier_from_request(v0.identity_id, "identity_id")?;
        let lists = lists_from_request(&v0.lists)?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, status) = Drive::verify_contract_moderation_status(
            supported_grovedb_proof_bytes(&proof, platform_version)?,
            contract_id,
            identity_id,
            &lists,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider, platform_version)?;

        // An absent entry is a status too: the identity is neither banned nor suspended.
        Ok((Some(status), metadata, proof))
    }
}

impl FromProof<GetContractModerationEntriesRequest> for ContractModerationEntries {
    type Request = GetContractModerationEntriesRequest;
    type Response = GetContractModerationEntriesResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let get_contract_moderation_entries_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_id = identifier_from_request(v0.contract_id, "contract_id")?;
        let query = entries_query_from_request(v0.list, v0.start_after.as_deref(), v0.limit)?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, entries) = Drive::verify_contract_moderation_entries(
            supported_grovedb_proof_bytes(&proof, platform_version)?,
            contract_id,
            &query,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider, platform_version)?;

        // An empty list proves as an empty page, so the page itself is always the answer.
        Ok((Some(ContractModerationEntries(entries)), metadata, proof))
    }
}
