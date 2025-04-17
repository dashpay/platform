use crate::error::MapGroveDbError;
use crate::types::token_status::TokenStatuses;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_token_statuses_request, GetTokenStatusesRequest, GetTokenStatusesResponse, Proof,
    ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

impl FromProof<GetTokenStatusesRequest> for TokenStatuses {
    type Request = GetTokenStatusesRequest;
    type Response = GetTokenStatusesResponse;

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

        let token_ids = match request.version.ok_or(Error::EmptyVersion)? {
            get_token_statuses_request::Version::V0(v0) => v0
                .token_ids
                .into_iter()
                .map(<[u8; 32]>::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| Error::RequestError {
                    error: "can't convert token_ids to [u8; 32]".to_string(),
                })?,
        };

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();

        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, result) =
            Drive::verify_token_statuses(&proof.grovedb_proof, &token_ids, false, platform_version)
                .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        Ok((Some(result), metadata, proof))
    }
}
