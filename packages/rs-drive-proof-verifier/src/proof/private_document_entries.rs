//! Proof verification of a page of the entries of a private document type's store.

use crate::error::MapGroveDbError;
use crate::types::contract_moderation::identifier_from_request;
use crate::types::private_document_entries::PrivateDocumentEntries;
use crate::verify::{document_query_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_private_document_entries_request, GetPrivateDocumentEntriesRequest,
    GetPrivateDocumentEntriesResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProviderError;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

impl FromProof<GetPrivateDocumentEntriesRequest> for PrivateDocumentEntries {
    type Request = GetPrivateDocumentEntriesRequest;
    type Response = GetPrivateDocumentEntriesResponse;

    /// A request asking for no proof is answered without one, which no proof can verify: it
    /// is read with `FromUnproved` instead.
    fn check_request(
        request: &Self::Request,
        _platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match &request.version {
            Some(get_private_document_entries_request::Version::V0(v0)) if !v0.prove => {
                Err(Error::RequestError {
                    error: "the request asks for no proof: read it unproved".to_string(),
                })
            }
            _ => Ok(()),
        }
    }

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

        // The type and its entry size come from the contract, which the client trusts; the
        // store's chunk power, which settles the page, from the store element the proof binds.
        // A contract the client's context does not hold with this private type is the
        // client's to refresh, not a fault of the node answering.
        let data_contract = provider
            .get_data_contract(&contract_id, platform_version)?
            .ok_or_else(|| {
                Error::ContextProviderError(ContextProviderError::DataContractFailure(format!(
                    "the context holds no data contract {contract_id}"
                )))
            })?;
        let document_type = data_contract
            .document_type_for_name(&v0.document_type_name)
            .ok()
            .filter(|document_type| document_type.private_document().is_some())
            .ok_or_else(|| {
                Error::ContextProviderError(ContextProviderError::DataContractFailure(format!(
                    "the context's data contract {contract_id} has no private document type \
                     \"{}\"",
                    v0.document_type_name
                )))
            })?;

        let (root_hash, page) = Drive::verify_private_document_entries(
            document_query_proof_bytes(&proof, &metadata, platform_version)?,
            contract_id.to_buffer(),
            document_type,
            v0.start_position,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        // A page always carries the store's count, so a proved page is never absent
        Ok((Some(page), metadata, proof))
    }
}
