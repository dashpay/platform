use crate::{common::setup_logs, config::Config};
use drive_proof_verifier::types::ProtocolVersionVoteCount;
use rs_sdk::platform::FetchMany;

/// Given some existing identity ID, when I fetch the identity keys, I get some of them indexed by key ID.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_protocol_version_votes() {
    setup_logs();

    let cfg = Config::new();
    let mut sdk = cfg.setup_api().await;

    let votings = ProtocolVersionVoteCount::fetch_many(&mut sdk, ())
        .await
        .expect("fetch protocol version votes");

    println!("votings: {:?}", votings);

    assert!(!votings.is_empty());
}
