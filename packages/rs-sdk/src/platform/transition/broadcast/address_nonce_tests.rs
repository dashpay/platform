use super::{broadcast_with_retries, BroadcastRequestForStateTransition};
use crate::SdkBuilder;
use dapi_grpc::platform::v0::{BroadcastStateTransitionResponse, ResponseMetadata};
use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::consensus::state::address_funds::AddressInvalidNonceError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use rs_dapi_client::{AddressList, ExecutionError, ExecutionResponse, RequestSettings};
use std::collections::BTreeMap;
use std::future::ready;

/// Testnet SH-024 (2026-10-23): a sequential Shield gets provided=2, expected=1.
/// Model one-block node skew; transport is mocked, including proof/signature admission.
#[tokio::test]
async fn should_retry_sequential_address_funded_transition_on_caught_up_node() {
    let addresses: AddressList = "http://127.0.0.1:3001,http://127.0.0.1:3002"
        .parse()
        .unwrap();
    let nodes = addresses.get_live_addresses();
    let lagging = nodes[0].clone();
    let caught_up = nodes[1].clone();
    let address = PlatformAddress::P2pkh([0x24; 20]);
    let platform_version = PlatformVersion::latest();
    let sdk = SdkBuilder::new_mock()
        .with_version(platform_version)
        .with_time_tolerance(Some(31 * 60 * 1000))
        .build()
        .unwrap();
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let settings = RequestSettings {
        retries: Some(1),
        ban_failed_address: Some(true),
        ..Default::default()
    };

    // Two sequential calls use nonce snapshots 0 and 1. After the first
    // inclusion, only the caught-up node has committed the nonce-1 block.
    for fetched_nonce in [0, 1] {
        let transition = StateTransition::Shield(
            ShieldTransitionV0 {
                inputs: BTreeMap::from([(address, (fetched_nonce + 1, 1_000_000_000))]),
                actions: vec![],
                amount: 200_000_000,
                anchor: [0; 32],
                proof: vec![],
                binding_signature: [0; 64],
                fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                user_fee_increase: 0,
                input_witnesses: vec![],
            }
            .into(),
        );
        let expected_request = transition.broadcast_request_for_state_transition().unwrap();
        let mut requests = Vec::new();
        let mut used_nodes = Vec::new();
        let result = broadcast_with_retries(&transition, &addresses, settings, |request, _| {
            requests.push(request);
            let reject = fetched_nonce == 1 && requests.len() == 1;
            let node = if reject { &lagging } else { &caught_up };
            used_nodes.push(node.clone());
            ready(if reject {
                Err(ExecutionError {
                    inner: ConsensusError::from(AddressInvalidNonceError::new(address, 2, 1))
                        .into(),
                    address: Some(node.clone()),
                    retries: 0,
                })
            } else {
                Ok(ExecutionResponse {
                    inner: BroadcastStateTransitionResponse {},
                    address: node.clone(),
                    retries: 0,
                })
            })
        })
        .await;
        assert!(
            result.is_ok(),
            "sequential nonce {} should reach the caught-up node unchanged; got {result:?}",
            fetched_nonce + 1
        );

        if fetched_nonce == 0 {
            // Model the first result wait / next AddressInfo fetch at height 100.
            let committed = ResponseMetadata {
                height: 100,
                time_ms: now,
                protocol_version: platform_version.protocol_version,
                ..Default::default()
            };
            sdk.verify_response_metadata("wait_for_state_transition_result", &committed)
                .unwrap();
            sdk.verify_response_metadata("get_addresses_infos", &committed)
                .unwrap();
            // Even if queried, the lagging node passes the production freshness
            // checks. Broadcasting itself has no response height to check.
            sdk.verify_response_metadata(
                "get_addresses_infos",
                &ResponseMetadata {
                    height: 99,
                    time_ms: now - 3_000,
                    ..committed
                },
            )
            .expect("one-block lag with a recent timestamp is within tolerance");
        } else {
            assert_eq!(requests, vec![expected_request.clone(), expected_request]);
            assert_eq!(used_nodes, vec![lagging.clone(), caught_up.clone()]);
        }
    }
}
