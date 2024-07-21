//! Test GetContestedResourceIdentityVotesRequest

use crate::fetch::{common::setup_logs, config::Config};
use dash_sdk::platform::FetchMany;
use dpp::{
    dashcore::{hashes::Hash, ProTxHash},
    identifier::Identifier,
    voting::votes::resource_vote::ResourceVote,
};
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;

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
/// See [check_mn_voting_prerequisities](super::contested_resource::check_mn_voting_prerequisities) for more details.
///
/// 1. Grep log output of `yarn setup` to find `ProRegTx transaction ID` and `Owner Private Key`.
/// Use `ProRegTx transaction ID` to set `DASH_SDK_MASTERNODE_OWNER_PRO_REG_TX_HASH`  in `packages/rs-sdk/tests/.env`.
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
async fn contested_resource_identity_votes_ok() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("contested_resource_identity_votes_ok").await;

    // Given some existing proTxHash of some Validator that already voted
    // Note: we hardcode default protxhash for offline testing in github actions
    let protx = cfg.existing_protxhash().unwrap_or_else(|_| {
        ProTxHash::from_byte_array(
            hex::decode("eddaa22953d348b86f2044e6582a383ba9fb90cec4ef8c7f0f231e248bd00916")
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
}
