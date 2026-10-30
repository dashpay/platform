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
        let transition = shield(address, fetched_nonce + 1);
        let expected_request = transition.broadcast_request_for_state_transition().unwrap();
        let mut requests = Vec::new();
        let mut used_nodes = Vec::new();
        let result = broadcast_with_retries(
            &transition,
            &addresses,
            settings,
            |request, _| {
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
            },
            platform_version,
        )
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
        } else {
            assert_eq!(requests, vec![expected_request.clone(), expected_request]);
            assert_eq!(used_nodes, vec![lagging.clone(), caught_up.clone()]);
            assert!(addresses.is_banned(&lagging));
            assert!(!addresses.is_banned(&caught_up));
            let banned_until = addresses
                .ban_info()
                .into_iter()
                .find(|info| info.uri == lagging.uri().to_string())
                .and_then(|info| info.banned_until)
                .expect("lagging node should have a short ban");
            assert!(banned_until <= chrono::Utc::now() + chrono::Duration::seconds(2));
        }
    }
}

fn shield(address: PlatformAddress, nonce: u32) -> StateTransition {
    StateTransition::Shield(
        ShieldTransitionV0 {
            inputs: BTreeMap::from([(address, (nonce, 1_000_000_000))]),
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
    )
}

#[test_case::test_case(2, 2, true; "equal nonce")]
#[test_case::test_case(2, 3, true; "behind nonce")]
#[test_case::test_case(3, 1, true; "different submitted nonce")]
#[test_case::test_case(2, 1, false; "different address")]
#[tokio::test]
async fn should_keep_non_skew_nonce_rejections_terminal(
    provided: u32,
    expected: u32,
    same_address: bool,
) {
    let addresses: AddressList = "http://127.0.0.1:3001,http://127.0.0.1:3002"
        .parse()
        .unwrap();
    let input = PlatformAddress::P2pkh([0x24; 20]);
    let error_address = if same_address {
        input
    } else {
        PlatformAddress::P2pkh([0x25; 20])
    };
    let transition = shield(input, 2);
    let request = transition.broadcast_request_for_state_transition().unwrap();
    let mut sends = 0;
    let result = broadcast_with_retries(
        &transition,
        &addresses,
        RequestSettings {
            retries: Some(3),
            ban_failed_address: Some(true),
            ..Default::default()
        },
        |sent, _| {
            assert_eq!(sent, request);
            sends += 1;
            ready(Err(ExecutionError {
                inner: ConsensusError::from(AddressInvalidNonceError::new(
                    error_address,
                    provided,
                    expected,
                ))
                .into(),
                address: Some(addresses.get_live_address().unwrap()),
                retries: 0,
            }))
        },
        PlatformVersion::latest(),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(sends, 1);
    assert_eq!(addresses.get_live_addresses().len(), 2);
}
