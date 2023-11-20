use crate::{common::setup_logs, config::Config};
use dashcore_rpc::dashcore::{hashes::Hash, ProTxHash};
use dpp::util::deserializer::ProtocolVersion;
use rs_sdk::platform::FetchMany;

/// Given some existing identity ID, when I fetch the identity keys, I get some of them indexed by key ID.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_protocol_version_votes() {
    setup_logs();

    let cfg = Config::new();
    let mut sdk = cfg.setup_api().await;

    let query = ProTxHash::from_slice(&[0u8; 32]).expect("zero protxhash");
    let votings = ProtocolVersion::fetch_many(&mut sdk, query)
        .await
        .expect("fetch protocol version votes by node");

    println!("votings: {:?}", votings);

    assert!(!votings.is_empty());
}
