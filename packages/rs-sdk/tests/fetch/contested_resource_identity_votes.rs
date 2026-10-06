//! Test GetContestedResourceIdentityVotesRequest

use crate::fetch::{common::setup_logs, config::Config};
use dash_sdk::platform::resource_votes_with_counts::{
    ResourceVoteWithCount, ResourceVotesWithCountsByIdentity,
};
use dash_sdk::platform::FetchMany;
use dash_sdk::Sdk;
use dpp::{
    dashcore::{hashes::Hash, ProTxHash},
    identifier::Identifier,
    platform_value::Value,
    voting::{
        vote_choices::resource_vote_choice::ResourceVoteChoice,
        vote_polls::{
            contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll, VotePoll,
        },
        votes::resource_vote::{v0::ResourceVoteV0, ResourceVote},
    },
};
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;

/// A vote of the mock tests: `choice` on the contested DPNS name `label`.
fn dpns_name_vote(label: &str, choice: ResourceVoteChoice) -> ResourceVote {
    ResourceVote::V0(ResourceVoteV0 {
        vote_poll: VotePoll::ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll {
            contract_id: Identifier::new([1; 32]),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text(label.to_string()),
            ],
        }),
        resource_vote_choice: choice,
    })
}

/// Given the votes of an identity with their counts, when I fetch them using mock API, then I
/// get the same votes and counts, and no entry for a vote poll the identity has not voted on.
#[tokio::test]
async fn test_mock_fetch_many_resource_votes_with_counts() {
    let mut sdk = Sdk::new_mock();

    let query = ContestedResourceVotesGivenByIdentityQuery {
        identity_id: Identifier::new([7; 32]),
        limit: Some(10),
        offset: None,
        order_ascending: true,
        start_at: None,
    };
    let voted_once = Identifier::new([2; 32]);
    let voted_three_times = Identifier::new([3; 32]);
    let not_voted = Identifier::new([4; 32]);
    let expected: ResourceVotesWithCountsByIdentity = [
        (
            voted_once,
            Some(ResourceVoteWithCount {
                resource_vote: dpns_name_vote(
                    "quantum",
                    ResourceVoteChoice::TowardsIdentity(Identifier::new([5; 32])),
                ),
                vote_count: 1,
            }),
        ),
        (
            voted_three_times,
            Some(ResourceVoteWithCount {
                resource_vote: dpns_name_vote("cooldog", ResourceVoteChoice::Lock),
                vote_count: 3,
            }),
        ),
    ]
    .into_iter()
    .collect();

    sdk.mock()
        .expect_fetch_many::<Identifier, ResourceVoteWithCount, _, ResourceVotesWithCountsByIdentity>(
            query.clone(),
            Some(expected.clone()),
        )
        .await
        .expect("register the expectation");

    let retrieved = ResourceVoteWithCount::fetch_many(&sdk, query)
        .await
        .expect("fetch votes with counts");

    assert_eq!(retrieved, expected);
    assert_eq!(
        retrieved
            .get(&voted_three_times)
            .and_then(Option::as_ref)
            .map(|vote| vote.vote_count),
        Some(3)
    );
    assert!(
        !retrieved.contains_key(&not_voted),
        "a vote poll that was not voted on has no entry"
    );
}

/// When we request votes for a non-existing identity, we should get no votes.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn contested_resource_identity_votes_not_found() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("contested_resource_identity_votes_not_found")
        .await;

    // Given some non-existing identity ID
    let identity_id = Identifier::new([0xff; 32]);

    // When I query for votes given by this identity
    let query = ContestedResourceVotesGivenByIdentityQuery {
        identity_id,
        limit: None,
        offset: None,
        order_ascending: true,
        start_at: None,
    };
    let votes = ResourceVote::fetch_many(&sdk, query)
        .await
        .expect("fetch votes for identity");

    // Then I get no votes
    assert!(votes.is_empty(), "no votes expected for this query");
}

/// When we request votes for an existing identity, we should get some votes.
///
/// ## Preconditions
///
/// 1. At least one vote exists for the given masternode identity (protx hash).
///
/// ## Setup process
///
/// In order to setup this test, you need to:
///
/// 0. Ensure you have at least 1 contested DPNS name in the system.
/// See [check_mn_voting_prerequisites](super::contested_resource::check_mn_voting_prerequisites) for more details.
///
/// 1. Grep log output of `yarn setup` (see logs/setup.log) to find `ProRegTx transaction ID` and `Owner Private Key`:
///  ```bash
///  egrep '(ProRegTx transaction ID|Owner Private Key)' logs/setup.log|head -n2
///  ```
///  Hardcode `ProRegTx transaction ID` in [Config::default_protxhash].
///
/// 2. Load masternode identity into [rs-platform-explorer](https://github.com/dashpay/rs-platform-explorer/):
///
///  * ensure `.env` file contains correct configuration
///  * start tui with `cargo run`
///  * select `w - wallet`
///  * ensure a wallet with positive balance is loaded; if not - load it (getting a wallet is out of scope of this document)
///  * select `p - Load Evonode Identity`.
///  * enter `ProRegTx transaction ID`  and `Owner Private Key` from step 1.
///  * top up the identity balance using `t - Identity top up` option (1 DASH will be OK).
///  * exit Wallet screen using `q - Back to Main`
///
/// 3. Vote for some contested resource using the masternode identity:
///
///  * select `csnq`:  `c - Contracts` -> `s - Fetch system contract` -> `n - Fetch DPNS contract` -> `q - Back to Contracts `
///  * press ENTER to enter the fetched contract, then select `domain` -> `c - Query Contested Resources`
///  * Select one of displayed names, use `v - Vote`, select some identity.
///
/// Now, vote should be casted and you can run this test.
///
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see docs of contested_resource_identity_votes_ok()"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
pub(super) async fn contested_resource_identity_votes_ok() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("contested_resource_identity_votes_ok").await;

    // Given some existing proTxHash of some Validator that already voted
    // Note: we hardcode default protxhash for offline testing in github actions
    let protx = cfg.existing_protxhash().unwrap_or_else(|_| {
        ProTxHash::from_byte_array(
            hex::decode("74e26f433328be4b833b8958c04c51615e03853378e0d56fbe5ecf24977f884b")
                .expect("valid hex-encoded protx hash")
                .try_into()
                .expect("valid protx hash length"),
        )
    });

    // When I query for votes given by this identity
    let votes = ResourceVote::fetch_many(&sdk, protx)
        .await
        .expect("fetch votes for identity");

    tracing::debug!(?protx, ?votes, "votes of masternode");

    // Then I get some votes
    assert!(!votes.is_empty(), "votes expected for this query");

    // When I read the same votes with their counts
    let votes_with_counts = ResourceVoteWithCount::fetch_many(&sdk, protx)
        .await
        .expect("fetch votes with counts for identity");

    // Then I get the same votes, each counted at least once
    assert_eq!(votes_with_counts.len(), votes.len());
    for (vote_poll_id, vote) in &votes {
        let vote_with_count = votes_with_counts
            .get(vote_poll_id)
            .and_then(Option::as_ref)
            .expect("every vote is returned with its count");
        assert_eq!(Some(&vote_with_count.resource_vote), vote.as_ref());
        assert!(
            vote_with_count.vote_count >= 1,
            "an existing vote was given at least once"
        );
    }
}
