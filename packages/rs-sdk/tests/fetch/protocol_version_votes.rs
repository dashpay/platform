use super::{common::setup_logs, config::Config};
use dashcore_rpc::dashcore::{hashes::Hash, ProTxHash};
use drive_proof_verifier::types::MasternodeProtocolVote;
use rs_sdk::platform::{types::version_votes::MasternodeProtocolVoteEx, FetchMany};

/// Given protxhash with only zeros, when I fetch protocol version votes for nodes, I can retrieve them.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_protocol_version_votes_zeros() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_protocol_version_votes_zeros").await;

    let starting_protxhash = ProTxHash::from_slice(&[0u8; 32]).expect("zero protxhash");
    let votings = MasternodeProtocolVote::fetch_many(&sdk, starting_protxhash)
        .await
        .expect("fetch protocol version votes by node");

    println!("votings: {:?}", votings);

    assert!(!votings.is_empty());
}

/// Given protxhash with only zeros, when I fetch protocol version votes for nodes, I can retrieve them.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_protocol_version_votes_none() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_protocol_version_votes_none").await;

    let votings = MasternodeProtocolVote::fetch_many(&sdk, None)
        .await
        .expect("fetch protocol version votes by node");

    println!("votings: {:?}", votings);

    assert!(!votings.is_empty());
}

/// Given protxhash with only zeros, when I fetcg protocol version votes for nodes with limit 2, I get exactly 2 items.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_protocol_version_votes_limit_2() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_protocol_version_votes_limit_2").await;

    let starting_protxhash = ProTxHash::from_slice(&[0u8; 32]).expect("zero protxhash");
    let votings = MasternodeProtocolVote::fetch_many_with_limit(&sdk, starting_protxhash, 2)
        .await
        .expect("fetch protocol version votes by node");

    println!("votings: {:?}", votings);

    assert!(votings.len() == 2);
}

/// Given protxhash with only `0xFF`s, when I fetch protocol version votes for nodes, I get nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_protocol_version_votes_nx() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_protocol_version_votes_nx").await;

    let starting_protxhash = Some(ProTxHash::from_slice(&[0xffu8; 32]).expect("zero protxhash"));
    let votings = MasternodeProtocolVote::fetch_votes(&sdk, starting_protxhash, Some(2))
        .await
        .expect("fetch protocol version votes by node");

    println!("votings: {:?}", votings);

    assert!(votings.is_empty());
}

/// Given None as a  protxhash, when I fetch protocol version votes for 0 nodes, I get error.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_protocol_version_votes_limit_0() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_protocol_version_votes_limit_0").await;

    let result = MasternodeProtocolVote::fetch_votes(&sdk, None, Some(0)).await;

    assert!(result.is_err());
}
