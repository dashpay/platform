//! Proof verification of a page of the entries of a private document type's store.

use crate::error::MapGroveDbError;
use crate::types::contract_moderation::identifier_from_request;
use crate::types::private_document_entries::PrivateDocumentEntries;
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_private_document_entries_request, GetPrivateDocumentEntriesRequest,
    GetPrivateDocumentEntriesResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

impl FromProof<GetPrivateDocumentEntriesRequest> for PrivateDocumentEntries {
    type Request = GetPrivateDocumentEntriesRequest;
    type Response = GetPrivateDocumentEntriesResponse;

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

        let get_private_document_entries_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let contract_id = identifier_from_request(&v0.data_contract_id, "data_contract_id")?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        // The type's entry size and the store's chunk size, which settle the page, come from
        // the contract, which the client trusts.
        let data_contract = provider
            .get_data_contract(&contract_id, platform_version)?
            .ok_or(Error::NotFound)?;
        let document_type = data_contract
            .document_type_for_name(&v0.document_type_name)
            .map_err(|e| Error::ProtocolError {
                error: e.to_string(),
            })?;

        let (root_hash, page) = Drive::verify_private_document_entries(
            supported_grovedb_proof_bytes(&proof)?,
            contract_id.to_buffer(),
            document_type,
            v0.start_position,
            v0.count,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        // A page always carries the store's count, so a proved page is never absent
        Ok((Some(page), metadata, proof))
    }
}
