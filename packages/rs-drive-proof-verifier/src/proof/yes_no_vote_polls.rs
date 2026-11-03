//! Proof verification of the yes/no vote poll queries: a poll's state and a masternode's answer
//! on a poll, both by the poll's id.

use crate::error::MapGroveDbError;
use crate::types::contract_moderation::identifier_from_request;
use crate::types::yes_no_vote_polls::{MasternodeYesNoVote, YesNoVotePollState};
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_masternode_yes_no_vote_request, get_yes_no_vote_poll_state_request,
    GetMasternodeYesNoVoteRequest, GetMasternodeYesNoVoteResponse, GetYesNoVotePollStateRequest,
    GetYesNoVotePollStateResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::query::masternode_yes_no_vote_query::MasternodeYesNoVoteDriveQuery;
use drive::query::yes_no_vote_poll_state_query::YesNoVotePollStateDriveQuery;

impl FromProof<GetYesNoVotePollStateRequest> for YesNoVotePollState {
    type Request = GetYesNoVotePollStateRequest;
    type Response = GetYesNoVotePollStateResponse;

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

        let get_yes_no_vote_poll_state_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let vote_poll_id = identifier_from_request(&v0.vote_poll_id, "vote_poll_id")?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, state) = YesNoVotePollStateDriveQuery { vote_poll_id }
            .verify_yes_no_vote_poll_state_proof(
                supported_grovedb_proof_bytes(&proof)?,
                platform_version,
            )
            .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        // A poll opens with its stored info, so one without it was never opened under this id
        // and has no votes either.
        let state = state.stored_info.is_some().then_some(state);
        Ok((state, metadata, proof))
    }
}

impl FromProof<GetMasternodeYesNoVoteRequest> for MasternodeYesNoVote {
    type Request = GetMasternodeYesNoVoteRequest;
    type Response = GetMasternodeYesNoVoteResponse;

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

        let get_masternode_yes_no_vote_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let query = MasternodeYesNoVoteDriveQuery {
            pro_tx_hash: identifier_from_request(&v0.pro_tx_hash, "pro_tx_hash")?,
            vote_poll_id: identifier_from_request(&v0.vote_poll_id, "vote_poll_id")?,
        };

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, vote) = query
            .verify_identity_yes_no_vote(
                supported_grovedb_proof_bytes(&proof)?,
                false,
                platform_version,
            )
            .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        // `None` when the masternode has not voted on the poll, or the poll finished and its
        // votes were removed.
        Ok((vote, metadata, proof))
    }
}
