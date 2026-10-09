//! Voting pages bind their effective limit to the authenticated SDK request.

#[path = "../../../rs-drive/tests/support/voting_default_limits.rs"]
mod fixture;

use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::{
    get_contested_resource_identity_votes_response as identity_response,
    get_vote_polls_by_end_date_response as polls_response,
    GetContestedResourceIdentityVotesRequest, GetContestedResourceIdentityVotesResponse,
    GetVotePollsByEndDateRequest, GetVotePollsByEndDateResponse, Proof, ResponseMetadata,
};
use dash_context_provider::{ContextProvider, ContextProviderError};
use dash_sdk::platform::QuerySettings;
use dash_sdk::platform::{FetchMany, Query};
use dash_sdk::{Sdk, SdkBuilder};
use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
use dpp::dashcore::{hashes::Hash, ProTxHash};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{DataContract, Identifier};
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use drive::config::DEFAULT_QUERY_LIMIT;
use drive::query::{
    contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery,
    VotePollsByEndDateDriveQuery,
};
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use rs_dapi_client::{transport::TransportRequest, DumpData, ExecutionResponse, RequestSettings};
use std::sync::Arc;
use tenderdash_abci::{
    proto::types::{CanonicalVote, SignedMsgType, StateId},
    signatures::{Hashable, Signable},
};

struct Provider(Arc<DataContract>);

impl ContextProvider for Provider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        _version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok((*id == self.0.id()).then(|| self.0.clone()))
    }

    fn get_token_configuration(
        &self,
        _id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let bytes: Vec<u8> = (&signing_key().public_key()).into();
        Ok(bytes.try_into().expect("BLS public key length"))
    }

    fn get_platform_activation_height(&self) -> Result<u32, ContextProviderError> {
        Ok(1)
    }
}

fn signing_key() -> SecretKey<Bls12381G2Impl> {
    SecretKey::from_hash(b"voting-default-limit-test")
}

fn authenticated(proof_bytes: Vec<u8>, root: [u8; 32]) -> (Proof, ResponseMetadata) {
    let metadata = ResponseMetadata {
        height: 100,
        core_chain_locked_height: 1,
        epoch: 0,
        time_ms: fixture::END_TIME,
        protocol_version: PlatformVersion::latest().protocol_version,
        chain_id: "test-chain".into(),
    };
    let mut proof = Proof {
        grovedb_proof: proof_bytes,
        quorum_hash: vec![2; 32],
        signature: vec![],
        round: 1,
        block_id_hash: vec![3; 32],
        quorum_type: 1,
    };
    let state = StateId {
        app_version: metadata.protocol_version as u64,
        core_chain_locked_height: metadata.core_chain_locked_height,
        time: metadata.time_ms,
        app_hash: root.to_vec(),
        height: metadata.height,
    };
    let vote = CanonicalVote {
        r#type: SignedMsgType::Precommit.into(),
        block_id: proof.block_id_hash.clone(),
        chain_id: metadata.chain_id.clone(),
        height: metadata.height as i64,
        round: proof.round as i64,
        state_id: state
            .calculate_msg_hash(
                &metadata.chain_id,
                metadata.height as i64,
                proof.round as i32,
            )
            .expect("signed state digest"),
    };
    let digest = vote
        .calculate_sign_hash(
            &metadata.chain_id,
            proof.quorum_type as u8,
            &[2; 32],
            metadata.height as i64,
            proof.round as i32,
        )
        .expect("vote digest");
    proof.signature = signing_key()
        .sign(SignatureSchemes::Basic, &digest)
        .expect("test signature")
        .as_raw_value()
        .to_compressed()
        .to_vec();
    (proof, metadata)
}

fn sdk_for<R: TransportRequest>(
    requests: &[R],
    response: R::Response,
    contract: DataContract,
) -> Sdk {
    let dir = tempfile::TempDir::new().expect("raw response fixtures");
    let response = Ok(ExecutionResponse {
        inner: response,
        retries: 0,
        address: "http://127.0.0.1:9000".parse().expect("mock address"),
    });
    for request in requests {
        let dump = DumpData::new(request, &response);
        dump.save(&dir.path().join(dump.filename().expect("request key")))
            .expect("raw DAPI response");
    }
    SdkBuilder::new_mock()
        .with_version(PlatformVersion::latest())
        .with_context_provider(Provider(Arc::new(contract)))
        .with_settings(RequestSettings {
            retries: Some(0),
            ..Default::default()
        })
        .with_dump_dir(dir.path())
        .build()
        .expect("SDK with real proof verification")
}

fn polls_query(limit: Option<u16>) -> VotePollsByEndDateDriveQuery {
    VotePollsByEndDateDriveQuery {
        start_time: None,
        end_time: None,
        limit,
        offset: None,
        order_ascending: true,
    }
}

fn identity_query(limit: Option<u16>) -> ContestedResourceVotesGivenByIdentityQuery {
    ContestedResourceVotesGivenByIdentityQuery {
        identity_id: Identifier::from(fixture::VOTER),
        limit,
        offset: None,
        start_at: None,
        order_ascending: true,
    }
}

#[tokio::test]
async fn should_verify_omitted_limit_poll_page_when_more_than_default_polls_exist() {
    let version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = fixture::populate(&drive, DEFAULT_QUERY_LIMIT as usize + 1, version);
    let (bytes, _) = polls_query(Some(DEFAULT_QUERY_LIMIT))
        .execute_with_proof(&drive, None, None, version)
        .expect("finite server proof");
    let root = drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .unwrap()
        .expect("root");
    let (proof, metadata) = authenticated(bytes, root);
    let response = GetVotePollsByEndDateResponse {
        version: Some(polls_response::Version::V0(
            polls_response::GetVotePollsByEndDateResponseV0 {
                result: Some(
                    polls_response::get_vote_polls_by_end_date_response_v0::Result::Proof(proof),
                ),
                metadata: Some(metadata),
            },
        )),
    };
    let query_sdk = Sdk::new_mock();
    let settings = query_sdk.query_settings();
    let omitted: GetVotePollsByEndDateRequest = polls_query(None).query(&settings).expect("query");
    let explicit: GetVotePollsByEndDateRequest = polls_query(Some(DEFAULT_QUERY_LIMIT))
        .query(&settings)
        .expect("query");
    let original = omitted.mock_serialize().expect("original caller bytes");
    let sdk = sdk_for(&[omitted, explicit], response, contract.clone());
    let expected: Vec<_> = (0..DEFAULT_QUERY_LIMIT as usize)
        .map(|i| {
            (
                fixture::END_TIME + i as u64,
                vec![fixture::poll(&contract, i)],
            )
        })
        .collect();
    let polls = VotePoll::fetch_many(&sdk, polls_query(None))
        .await
        .expect("honest limited poll proof must verify");
    assert_eq!(polls.0, expected);
    let raw = VotePoll::fetch_many(&sdk, omitted)
        .await
        .expect("raw request");
    assert_eq!(raw.0, expected);
    assert_eq!(omitted.mock_serialize().expect("caller bytes"), original);
    let (custom, metadata, _) =
        VotePoll::fetch_many_with_metadata_and_proof(&sdk, CustomPolls(omitted), None)
            .await
            .expect("custom producer");
    assert_eq!(custom.0, expected);
    assert_eq!(metadata.height, 100);
}

#[tokio::test]
async fn should_verify_omitted_limit_identity_vote_page_when_more_than_default_votes_exist() {
    let version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = fixture::populate(&drive, DEFAULT_QUERY_LIMIT as usize + 1, version);
    let (bytes, _) = identity_query(Some(DEFAULT_QUERY_LIMIT))
        .execute_with_proof(&drive, None, None, version)
        .expect("finite server proof");
    let root = drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .unwrap()
        .expect("root");
    let (proof, metadata) = authenticated(bytes, root);
    let response = GetContestedResourceIdentityVotesResponse {
        version: Some(identity_response::Version::V0(identity_response::GetContestedResourceIdentityVotesResponseV0 {
            result: Some(identity_response::get_contested_resource_identity_votes_response_v0::Result::Proof(proof)),
            metadata: Some(metadata),
        })),
    };
    let query_sdk = Sdk::new_mock();
    let settings = query_sdk.query_settings();
    let omitted: GetContestedResourceIdentityVotesRequest =
        identity_query(None).query(&settings).expect("query");
    let explicit: GetContestedResourceIdentityVotesRequest =
        identity_query(Some(DEFAULT_QUERY_LIMIT))
            .query(&settings)
            .expect("query");
    let sdk = sdk_for(&[omitted.clone(), explicit], response, contract.clone());
    let mut expected: Vec<_> = (0..=DEFAULT_QUERY_LIMIT as usize)
        .map(|i| {
            let poll = fixture::poll(&contract, i);
            (
                poll.unique_id().expect("poll id"),
                Some(ResourceVote::V0(ResourceVoteV0 {
                    vote_poll: poll,
                    resource_vote_choice: ResourceVoteChoice::Abstain,
                })),
            )
        })
        .collect();
    expected.sort_by_key(|(id, _)| *id);
    expected.truncate(DEFAULT_QUERY_LIMIT as usize);
    let votes = ResourceVote::fetch_many(&sdk, identity_query(None))
        .await
        .expect("honest limited identity-vote proof must verify");
    assert_eq!(votes.into_iter().collect::<Vec<_>>(), expected);
    let raw = ResourceVote::fetch_many(&sdk, omitted)
        .await
        .expect("raw identity query");
    assert_eq!(raw.into_iter().collect::<Vec<_>>(), expected);
    let convenience = ResourceVote::fetch_many(&sdk, ProTxHash::from_byte_array(fixture::VOTER))
        .await
        .expect("ProTxHash query");
    assert_eq!(convenience.into_iter().collect::<Vec<_>>(), expected);
}

#[derive(Clone, Debug)]
struct CustomPolls(GetVotePollsByEndDateRequest);

impl Query<GetVotePollsByEndDateRequest> for CustomPolls {
    fn query(
        &self,
        _settings: &QuerySettings<'_>,
    ) -> Result<GetVotePollsByEndDateRequest, dash_sdk::Error> {
        Ok(self.0)
    }
}

#[path = "vote_poll_default_limit_controls.rs"]
mod controls;

#[path = "vote_poll_default_limit_vectors.rs"]
mod vectors;
