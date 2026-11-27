//! A wait for a state transition recorded before `prove_state_before_block` existed replays.
// The Platform messages exist with `platform` and a generated client, or on wasm32
#![cfg(all(
    feature = "mocks",
    feature = "platform",
    any(feature = "client", target_arch = "wasm32")
))]

use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::wait_for_state_transition_result_request::{
    Version, WaitForStateTransitionResultRequestV0,
};
use dapi_grpc::platform::v0::WaitForStateTransitionResultRequest;

#[test]
fn should_replay_a_wait_recorded_before_the_state_before_block_flag() {
    let request = WaitForStateTransitionResultRequest {
        version: Some(Version::V0(WaitForStateTransitionResultRequestV0 {
            state_transition_hash: vec![7; 32],
            prove: true,
            request_user_balance: false,
            prove_state_before_block: false,
        })),
    };
    let mut recorded: serde_json::Value =
        serde_json::from_slice(&request.mock_serialize().expect("expected to serialize"))
            .expect("expected JSON");

    // As recorded before the flag: field 4 has no key
    let v0 = &mut recorded["version"]["v0"];
    assert!(
        v0.as_object_mut()
            .expect("expected the v0 request as an object")
            .remove("prove_state_before_block")
            .is_some(),
        "the key must be there to remove: {recorded}"
    );

    let replayed = WaitForStateTransitionResultRequest::mock_deserialize(
        &serde_json::to_vec(&recorded).expect("expected JSON bytes"),
    )
    .expect("expected a recording without the flag to replay");

    assert_eq!(replayed, request);
}
