//! The finite page in a proved voting response uses the configured request default.

#[path = "../../rs-drive/tests/support/voting_default_limits.rs"]
mod fixture;

use dapi_grpc::platform::v0::{
    get_contested_resource_identity_votes_request as identity_request,
    get_vote_polls_by_end_date_request as polls_request, GetContestedResourceIdentityVotesRequest,
    GetVotePollsByEndDateRequest,
};
use dpp::version::PlatformVersion;
use drive::config::DEFAULT_QUERY_LIMIT;
use drive::error::query::QuerySyntaxError;
use drive_abci::config::PlatformConfig;
use drive_abci::error::query::QueryError;
use drive_abci::test::helpers::setup::TestPlatformBuilder;

#[test]
fn should_prove_omitted_poll_limit_as_the_explicit_configured_limit() {
    let version = PlatformVersion::latest();
    for limit in [DEFAULT_QUERY_LIMIT, DEFAULT_QUERY_LIMIT / 2] {
        let mut config = PlatformConfig::default();
        config.drive.default_query_limit = limit;
        let platform = TestPlatformBuilder::new()
            .with_config(config)
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        platform
            .drive
            .create_initial_state_structure(None, version)
            .expect("Drive structure");
        fixture::populate(&platform.drive, DEFAULT_QUERY_LIMIT as usize + 1, version);
        let state = platform.state.load();
        let omitted = GetVotePollsByEndDateRequest {
            version: Some(polls_request::Version::V0(
                polls_request::GetVotePollsByEndDateRequestV0 {
                    prove: true,
                    ascending: true,
                    ..Default::default()
                },
            )),
        };
        let mut explicit = omitted;
        let Some(polls_request::Version::V0(v0)) = explicit.version.as_mut() else {
            panic!("versioned request")
        };
        v0.limit = Some(limit as u32);
        let omitted = platform
            .query_vote_polls_by_end_date_query(omitted, &state, version)
            .expect("omitted handler limit");
        let explicit = platform
            .query_vote_polls_by_end_date_query(explicit, &state, version)
            .expect("explicit handler limit");
        assert!(omitted.is_valid());
        assert!(omitted.data.is_some());
        assert_eq!(omitted.data, explicit.data);
    }
}

#[test]
fn should_prove_omitted_identity_vote_limit_as_the_explicit_configured_limit() {
    let version = PlatformVersion::latest();
    for limit in [DEFAULT_QUERY_LIMIT, DEFAULT_QUERY_LIMIT / 2] {
        let mut config = PlatformConfig::default();
        config.drive.default_query_limit = limit;
        let platform = TestPlatformBuilder::new()
            .with_config(config)
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        platform
            .drive
            .create_initial_state_structure(None, version)
            .expect("Drive structure");
        fixture::populate(&platform.drive, DEFAULT_QUERY_LIMIT as usize + 1, version);
        let state = platform.state.load();
        let omitted = GetContestedResourceIdentityVotesRequest {
            version: Some(identity_request::Version::V0(
                identity_request::GetContestedResourceIdentityVotesRequestV0 {
                    identity_id: fixture::VOTER.to_vec(),
                    prove: true,
                    order_ascending: true,
                    ..Default::default()
                },
            )),
        };
        let mut explicit = omitted.clone();
        let Some(identity_request::Version::V0(v0)) = explicit.version.as_mut() else {
            panic!("versioned request")
        };
        v0.limit = Some(limit as u32);
        let omitted = platform
            .query_contested_resource_identity_votes(omitted, &state, version)
            .expect("omitted handler limit");
        let explicit = platform
            .query_contested_resource_identity_votes(explicit, &state, version)
            .expect("explicit handler limit");
        assert!(omitted.is_valid());
        assert!(omitted.data.is_some());
        assert_eq!(omitted.data, explicit.data);
    }
}

#[test]
fn should_refuse_the_sdk_default_above_a_lower_server_cap_without_clamping() {
    let version = PlatformVersion::latest();
    let mut config = PlatformConfig::default();
    config.drive.default_query_limit = DEFAULT_QUERY_LIMIT / 2;
    let platform = TestPlatformBuilder::new()
        .with_config(config)
        .with_latest_protocol_version()
        .build_with_mock_rpc();
    platform
        .drive
        .create_initial_state_structure(None, version)
        .expect("Drive structure");
    fixture::populate(&platform.drive, DEFAULT_QUERY_LIMIT as usize + 1, version);
    let state = platform.state.load();
    let mut polls = GetVotePollsByEndDateRequest {
        version: Some(polls_request::Version::V0(
            polls_request::GetVotePollsByEndDateRequestV0 {
                prove: true,
                ascending: true,
                limit: Some(DEFAULT_QUERY_LIMIT as u32),
                ..Default::default()
            },
        )),
    };
    let mut votes = GetContestedResourceIdentityVotesRequest {
        version: Some(identity_request::Version::V0(
            identity_request::GetContestedResourceIdentityVotesRequestV0 {
                identity_id: fixture::VOTER.to_vec(),
                prove: true,
                order_ascending: true,
                limit: Some(DEFAULT_QUERY_LIMIT as u32),
                ..Default::default()
            },
        )),
    };
    let result = platform
        .query_vote_polls_by_end_date_query(polls, &state, version)
        .expect("request refusal");
    assert!(result.data.is_none());
    assert!(
        matches!(result.errors.as_slice(), [QueryError::InvalidArgument(message)] if message.contains("limit 100 out of bounds of [1, 50]"))
    );
    let result = platform
        .query_contested_resource_identity_votes(votes.clone(), &state, version)
        .expect("request refusal");
    assert!(result.data.is_none());
    assert!(
        matches!(result.errors.as_slice(), [QueryError::Query(QuerySyntaxError::InvalidLimit(message))] if message.contains("limit 100 out of bounds of [1, 50]"))
    );
    let Some(polls_request::Version::V0(v0)) = polls.version.as_mut() else {
        panic!("versioned request")
    };
    v0.limit = Some((DEFAULT_QUERY_LIMIT / 2) as u32);
    let Some(identity_request::Version::V0(v0)) = votes.version.as_mut() else {
        panic!("versioned request")
    };
    v0.limit = Some((DEFAULT_QUERY_LIMIT / 2) as u32);
    assert!(platform
        .query_vote_polls_by_end_date_query(polls, &state, version)
        .expect("explicit lower limit")
        .is_valid());
    assert!(platform
        .query_contested_resource_identity_votes(votes, &state, version)
        .expect("explicit lower limit")
        .is_valid());
}
