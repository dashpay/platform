use super::{common::setup_logs, config::Config};
use dpp::version::ProtocolVersionVoteCount;
use rs_sdk::platform::FetchMany;

/// Given some existing identity ID, when I fetch the identity keys, I get some of them indexed by key ID.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_protocol_version_vote_count() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_protocol_version_vote_count").await;

    let votings = ProtocolVersionVoteCount::fetch_many(&sdk, ())
        .await
        .expect("fetch protocol version votes");

    println!("votings: {:?}", votings);

    assert!(!votings.is_empty());
}
