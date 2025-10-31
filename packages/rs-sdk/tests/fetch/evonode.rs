//! Test evo node status and other node-related functionality.

use super::{common::setup_logs, config::Config};
use dash_sdk::platform::{types::evonode::EvoNode, FetchUnproved};
use dpp::dashcore::{hashes::Hash, ProTxHash};
use drive_proof_verifier::types::evonode_status::EvoNodeStatus;
use rs_dapi_client::Address;
use std::time::Duration;
/// Given some existing evonode URIs, WHEN we connect to them, THEN we get status.
use tokio::time::timeout;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_evonode_status() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_evonode_status").await;

    for (address, _status) in cfg.address_list() {
        let node = EvoNode::new(address.clone());
        match timeout(
            Duration::from_secs(3),
            EvoNodeStatus::fetch_unproved(&sdk, node),
        )
        .await
        {
            Ok(Ok(Some(status))) => {
                tracing::debug!(?status, ?address, "evonode status");
                // Add assertions here to verify the status contents
                assert!(
                    status.chain.latest_block_height > 0,
                    "latest block height must be positive"
                );
                let drive_protocol = status
                    .version
                    .protocol
                    .as_ref()
                    .and_then(|protocol| protocol.drive.as_ref())
                    .expect("drive protocol info must be present");
                assert!(
                    drive_protocol.current > 0,
                    "drive current protocol version must be set"
                );
                assert!(
                    drive_protocol.latest > 0,
                    "drive latest supported protocol version must be set"
                );
                assert!(
                    drive_protocol.next_epoch > 0,
                    "drive next epoch protocol version must be set"
                );
                assert_eq!(
                    status.node.pro_tx_hash.unwrap_or_default().len(),
                    ProTxHash::LEN,
                    "latest block hash must be non-empty"
                );
                // Add more specific assertions based on expected status properties
            }
            Ok(Ok(None)) => {
                tracing::warn!(?address, "No status found for evonode");
                panic!("No status found for evonode");
            }
            Ok(Err(e)) => {
                tracing::error!(?address, error = ?e, "Error fetching evonode status");
            }
            Err(_) => {
                tracing::error!(?address, "Timeout while fetching evonode status");
            }
        }
    }
}

/// Given invalid evonode URI, when we request status, we get error.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_evonode_status_refused() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_evonode_status_refused").await;

    let address: Address = "http://127.0.0.1:1".parse().expect("valid address");

    let node = EvoNode::new(address.clone());
    let result = EvoNodeStatus::fetch_unproved(&sdk, node).await;
    tracing::debug!(?result, ?address, "evonode status");

    assert!(result.is_err());
}
