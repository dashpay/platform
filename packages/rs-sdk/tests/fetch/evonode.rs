//! Test evo node status and other node-related functionality.

use dash_sdk::platform::{types::evonode::EvoNode, FetchUnproved};
use drive_proof_verifier::types::EvonodeStatus;
use http::Uri;

use super::{common::setup_logs, config::Config};

/// Given some existing evonode URIs, WHEN we connect to them, THEN we get status.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_evonode_status() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_evonode_status").await;

    let addresses = sdk.address_list().unwrap();

    for address in addresses.addresses() {
        let node = EvoNode::new(address.clone());
        let status = EvonodeStatus::fetch_unproved(&sdk, node)
            .await
            .expect("fetch evonode status")
            .expect("found evonode status");

        tracing::debug!(?status, ?address, "evonode status");
    }
}

/// Given invalid evonode URI, when we request status, we get error.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_evonode_status_refused() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_evonode_status_refused").await;

    let uri: Uri = "http://127.0.0.1:1".parse().unwrap();

    let node = EvoNode::new(uri.clone().into());
    let result = EvonodeStatus::fetch_unproved(&sdk, node).await;

    assert!(result.is_err());

    tracing::debug!(?result, ?uri, "evonode status");
}
