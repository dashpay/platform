use super::{broadcast_with_retries, BroadcastRequestForStateTransition};
use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::consensus::state::address_funds::AddressInvalidNonceError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use rs_dapi_client::{AddressList, ExecutionError, RequestSettings};
use std::collections::BTreeMap;
use std::future::ready;

/// A concrete nonce refusal ends sending; uncertainty from an earlier attempt is retained.
#[tokio::test]
async fn should_stop_on_nonce_rejection_without_rebuilding_or_resending() {
    let addresses: AddressList = "http://127.0.0.1:3001,http://127.0.0.1:3002"
        .parse()
        .unwrap();
    let address = PlatformAddress::P2pkh([0x24; 20]);
    for earlier_uncertainty in [false, true] {
        let transition = shield(address, 2);
        let expected = transition.broadcast_request_for_state_transition().unwrap();
        let mut sends = 0;
        let error = broadcast_with_retries(
            &transition,
            &addresses,
            RequestSettings {
                retries: Some(2),
                ..Default::default()
            },
            |request, _| {
                assert_eq!(request, expected);
                sends += 1;
                ready(Err(ExecutionError {
                    possibly_accepted: earlier_uncertainty,
                    inner: ConsensusError::from(AddressInvalidNonceError::new(address, 2, 1))
                        .into(),
                    address: addresses.get_live_addresses().first().cloned(),
                    retries: 0,
                }))
            },
            PlatformVersion::latest(),
        )
        .await
        .unwrap_err();
        assert_eq!(sends, 1);
        assert_eq!(error.is_possibly_accepted(), earlier_uncertainty);
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
                possibly_accepted: false,
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
