//! Voting pages bind their effective limit to the authenticated SDK request.

#[path = "../../../rs-drive/tests/support/voting_default_limits_data.rs"]
mod fixture;

use dapi_grpc::platform::v0::{
    GetContestedResourceIdentityVotesRequest, GetContestedResourceIdentityVotesResponse,
    GetVotePollsByEndDateRequest, GetVotePollsByEndDateResponse,
};
use dapi_grpc::{mock::Mockable, Message};
use dash_context_provider::{ContextProvider, ContextProviderError};
use dash_sdk::platform::QuerySettings;
use dash_sdk::platform::{FetchMany, Query};
use dash_sdk::{Sdk, SdkBuilder};
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
use rs_dapi_client::{transport::TransportRequest, DumpData, ExecutionResponse, RequestSettings};
use std::{fs, path::PathBuf, sync::Arc};

struct Provider(Arc<DataContract>, [u8; 48]);

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
        Ok(self.1)
    }

    fn get_platform_activation_height(&self) -> Result<u32, ContextProviderError> {
        Ok(1)
    }
}

fn corpus_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/voting_default_limits")
}

fn corpus_manifest() -> serde_json::Value {
    let bytes = fs::read(corpus_directory().join("manifest.json")).expect("fixture manifest");
    let manifest: serde_json::Value =
        serde_json::from_slice(&bytes).expect("fixture manifest JSON");
    assert_eq!(
        manifest["protocol_version"].as_u64(),
        Some(fixture::PROTOCOL_VERSION as u64)
    );
    manifest
}

fn quorum_key(wrong: bool) -> [u8; 48] {
    let manifest = corpus_manifest();
    let field = if wrong {
        "wrong_quorum_public_key"
    } else {
        "quorum_public_key"
    };
    let bytes: Vec<u8> = manifest[field]
        .as_array()
        .expect("quorum public key")
        .iter()
        .map(|value| u8::try_from(value.as_u64().expect("public key byte")).unwrap())
        .collect();
    bytes.try_into().expect("BLS public key length")
}

fn fixture_response<R: Message + Default>(
    endpoint: &str,
    count: usize,
    ascending: bool,
    limit: u16,
    kind: &str,
) -> R {
    let manifest = corpus_manifest();
    let case = manifest["cases"]
        .as_array()
        .expect("response cases")
        .iter()
        .find(|case| {
            case["endpoint"].as_str() == Some(endpoint)
                && case["count"].as_u64() == Some(count as u64)
                && case["ascending"].as_bool() == Some(ascending)
                && case["limit"].as_u64() == Some(limit as u64)
                && case["kind"].as_str() == Some(kind)
        })
        .expect("response parameters in corpus");
    let filename = case["filename"].as_str().expect("response filename");
    let bytes = fs::read(corpus_directory().join(filename)).expect("committed response bytes");
    R::decode(bytes.as_slice()).expect("protobuf response")
}

fn sdk_for<R: TransportRequest>(
    requests: &[R],
    response: R::Response,
    contract: DataContract,
    wrong_quorum_key: bool,
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
        .with_version(fixture::version())
        .with_context_provider(Provider(Arc::new(contract), quorum_key(wrong_quorum_key)))
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
    let version = fixture::version();
    let contract = fixture::contract(version);
    let response: GetVotePollsByEndDateResponse = fixture_response(
        "polls",
        DEFAULT_QUERY_LIMIT as usize + 1,
        true,
        DEFAULT_QUERY_LIMIT,
        "valid",
    );
    let query_sdk = Sdk::new_mock();
    let settings = query_sdk.query_settings();
    let omitted: GetVotePollsByEndDateRequest = polls_query(None).query(&settings).expect("query");
    let explicit: GetVotePollsByEndDateRequest = polls_query(Some(DEFAULT_QUERY_LIMIT))
        .query(&settings)
        .expect("query");
    let original = omitted.mock_serialize().expect("original caller bytes");
    let sdk = sdk_for(&[omitted, explicit], response, contract.clone(), false);
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
    let version = fixture::version();
    let contract = fixture::contract(version);
    let response: GetContestedResourceIdentityVotesResponse = fixture_response(
        "votes",
        DEFAULT_QUERY_LIMIT as usize + 1,
        true,
        DEFAULT_QUERY_LIMIT,
        "valid",
    );
    let query_sdk = Sdk::new_mock();
    let settings = query_sdk.query_settings();
    let omitted: GetContestedResourceIdentityVotesRequest =
        identity_query(None).query(&settings).expect("query");
    let explicit: GetContestedResourceIdentityVotesRequest =
        identity_query(Some(DEFAULT_QUERY_LIMIT))
            .query(&settings)
            .expect("query");
    let sdk = sdk_for(
        &[omitted.clone(), explicit],
        response,
        contract.clone(),
        false,
    );
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
