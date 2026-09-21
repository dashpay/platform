//! Proof verification of the state of an identity contender vote poll.

use crate::error::MapGroveDbError;
use crate::types::identity_contender_vote_poll_state::{
    state_query_from_request, IdentityContenderVotePollState,
};
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_identity_contender_vote_poll_state_request, GetIdentityContenderVotePollStateRequest,
    GetIdentityContenderVotePollStateResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;

// rpc getIdentityContenderVotePollState(GetIdentityContenderVotePollStateRequest) returns (GetIdentityContenderVotePollStateResponse);
impl FromProof<GetIdentityContenderVotePollStateRequest> for IdentityContenderVotePollState {
    type Request = GetIdentityContenderVotePollStateRequest;
    type Response = GetIdentityContenderVotePollStateResponse;

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

        let get_identity_contender_vote_poll_state_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let query = state_query_from_request(&v0, platform_version)?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, state) = query
            .verify_identity_contender_vote_poll_state_proof(
                supported_grovedb_proof_bytes(&proof, platform_version)?,
                platform_version,
            )
            .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider, platform_version)?;

        // A poll that never opened proves as nothing at all
        let state: IdentityContenderVotePollState = state.into();
        let state = if state.is_empty() { None } else { Some(state) };
        Ok((state, metadata, proof))
    }
}
