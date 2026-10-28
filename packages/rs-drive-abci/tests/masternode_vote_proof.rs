//! The proof of a masternode's vote on one vote poll, as the SDK reads it when the network
//! answers a vote broadcast with "already exists" and the SDK fetches the stored vote instead.

// Both modules are shared between suites, and this one uses only part of each.
#[allow(dead_code)]
mod signed_response;

#[path = "../../rs-drive/tests/support/voting_default_limits.rs"]
#[allow(dead_code, unused_imports)]
mod fixture;

use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::get_contested_resource_identity_votes_request_v0::StartAtVotePollIdInfo;
use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::GetContestedResourceIdentityVotesRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_identity_votes_response::{
    get_contested_resource_identity_votes_response_v0, GetContestedResourceIdentityVotesResponseV0,
    Version,
};
use dapi_grpc::platform::v0::{
    GetContestedResourceIdentityVotesRequest, GetContestedResourceIdentityVotesResponse,
    ResponseMetadata,
};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::Vote;
use drive_abci::rpc::core::MockCoreRPCLike;
use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use drive_proof_verifier::{ContextProvider, ContextProviderError, FromProof};
use signed_response::{signed, Provider};
use std::sync::Arc;

/// The number of vote polls the masternode has voted on.
const VOTED_POLLS: usize = 3;

/// Serves the voting fixture's DPNS contract, and the test quorum's public key.
struct ContractProvider(Arc<DataContract>);
impl ContextProvider for ContractProvider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok((*id == self.0.id()).then(|| self.0.clone()))
    }
    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
    fn get_quorum_public_key(
        &self,
        kind: u32,
        hash: [u8; 32],
        height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        Provider.get_quorum_public_key(kind, hash, height)
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

/// A platform where the fixture's masternode has voted to abstain on the first
/// [`VOTED_POLLS`] DPNS name contests.
fn platform_with_votes(version: &PlatformVersion) -> (TempPlatform<MockCoreRPCLike>, DataContract) {
    let platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc();
    platform
        .drive
        .create_initial_state_structure(None, version)
        .expect("Drive structure");
    let contract = fixture::populate(&platform.drive, VOTED_POLLS, version);
    (platform, contract)
}

/// The request the SDK's `VoteQuery` sends for the masternode's vote on `vote_poll_id`: the
/// masternode's votes from that poll on, at most one.
fn vote_request(vote_poll_id: Identifier) -> GetContestedResourceIdentityVotesRequest {
    GetContestedResourceIdentityVotesRequestV0 {
        identity_id: fixture::VOTER.to_vec(),
        limit: Some(1),
        offset: None,
        order_ascending: true,
        start_at_vote_poll_id_info: Some(StartAtVotePollIdInfo {
            start_at_poll_identifier: vote_poll_id.to_vec(),
            start_poll_identifier_included: true,
        }),
        prove: true,
    }
    .into()
}

/// Answers `request` with the node's query handler, its proof signed by the test quorum over
/// the platform's state.
fn answer(
    platform: &TempPlatform<MockCoreRPCLike>,
    request: GetContestedResourceIdentityVotesRequest,
    version: &PlatformVersion,
) -> GetContestedResourceIdentityVotesResponse {
    let state = platform.state.load();
    let response = platform
        .query_contested_resource_identity_votes(request, &state, version)
        .expect("query handled")
        .into_data()
        .expect("valid query");
    let Some(Version::V0(GetContestedResourceIdentityVotesResponseV0 {
        result: Some(get_contested_resource_identity_votes_response_v0::Result::Proof(proof)),
        ..
    })) = response.version
    else {
        panic!("expected a proof")
    };
    let metadata = ResponseMetadata {
        height: 10,
        core_chain_locked_height: 1,
        time_ms: 1_700_000_000_000,
        protocol_version: version.protocol_version,
        chain_id: "dash-testnet-51".into(),
        ..Default::default()
    };
    let root = platform
        .drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .unwrap()
        .expect("root hash");
    GetContestedResourceIdentityVotesResponse {
        version: Some(Version::V0(GetContestedResourceIdentityVotesResponseV0 {
            result: Some(
                get_contested_resource_identity_votes_response_v0::Result::Proof(signed(
                    proof.grovedb_proof,
                    &metadata,
                    root,
                )),
            ),
            metadata: Some(metadata),
        })),
    }
}

/// Fetches the masternode's vote on `vote_poll_id` the way the SDK does, and reads it from the
/// proof.
fn fetch_vote(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    vote_poll_id: Identifier,
    version: &PlatformVersion,
) -> Result<Option<Vote>, Box<drive_proof_verifier::Error>> {
    let request = vote_request(vote_poll_id);
    let response = answer(platform, request.clone(), version);
    <Vote as FromProof<GetContestedResourceIdentityVotesRequest>>::maybe_from_proof(
        request,
        response,
        Network::Testnet,
        version,
        &ContractProvider(Arc::new(contract.clone())),
    )
    .map_err(Box::new)
}

/// A vote transition broadcast again, as when a broadcast is retried after its answer was lost,
/// is answered that it already exists, and the SDK then confirms the vote from a proof of it.
/// The proved vote is keyed by its vote poll, so the SDK must read the vote of the requested
/// poll from it.
#[test]
fn should_read_the_proved_vote_of_a_masternode_vote_that_already_exists() {
    let version = PlatformVersion::latest();
    let (platform, contract) = platform_with_votes(version);

    for i in 0..VOTED_POLLS {
        let vote_poll = fixture::poll(&contract, i);
        let vote_poll_id = vote_poll.unique_id().expect("vote poll id");

        let vote = fetch_vote(&platform, &contract, vote_poll_id, version)
            .expect("the proof of an existing vote decodes");

        assert_eq!(
            vote,
            Some(Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll,
                resource_vote_choice: ResourceVoteChoice::Abstain,
            }))),
            "vote on poll {i}"
        );
    }
}

/// The request starts at the requested vote poll, so when the masternode has not voted on it
/// the proof carries its vote on the next poll instead. That vote is not the requested one:
/// the masternode is proved to have no vote on the requested poll.
#[test]
fn should_prove_no_vote_on_a_vote_poll_the_masternode_has_not_voted_on() {
    let version = PlatformVersion::latest();
    let (platform, contract) = platform_with_votes(version);
    // Below the id of every vote poll the masternode voted on.
    let not_voted_on = Identifier::new([0; 32]);

    let vote = fetch_vote(&platform, &contract, not_voted_on, version)
        .expect("the proof of the masternode's next vote decodes");

    assert_eq!(vote, None);
}
