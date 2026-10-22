//! Exercise the SDK lifecycle on the browser executor, without network I/O.
#![cfg(target_arch = "wasm32")]

use dash_sdk::platform::MockContextProvider;
use dash_sdk::{ProtocolCompatibilityState, ProtocolCompatibilityUnavailableReason, SdkBuilder};
use rs_dapi_client::AddressList;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn should_begin_and_refresh_on_browser_executor_without_tokio_timers() {
    // No addresses: query failure is immediate and entirely local. The
    // refresh still starts its budget timer before polling the query.
    let sdk = SdkBuilder::new(AddressList::default())
        .with_context_provider(MockContextProvider::new())
        .build()
        .unwrap();
    let generation = sdk.begin_protocol_compatibility_check();
    assert_eq!(
        sdk.protocol_compatibility().state,
        ProtocolCompatibilityState::Checking
    );
    let snapshot = sdk.refresh_protocol_compatibility(generation).await;
    assert_eq!(snapshot.generation, generation);
    assert_eq!(
        snapshot.state,
        ProtocolCompatibilityState::Unavailable {
            reason: ProtocolCompatibilityUnavailableReason::CheckFailed,
        }
    );
    assert_eq!(snapshot.verified_protocol_version, None);
    assert!(!snapshot.newer_protocol_available());
}

#[wasm_bindgen_test]
async fn should_report_proofs_disabled_on_browser_executor() {
    let sdk = SdkBuilder::new(AddressList::default())
        .with_context_provider(MockContextProvider::new())
        .with_proofs(false)
        .build()
        .unwrap();
    let generation = sdk.begin_protocol_compatibility_check();
    let snapshot = sdk.refresh_protocol_compatibility(generation).await;
    assert_eq!(
        snapshot.state,
        ProtocolCompatibilityState::Unavailable {
            reason: ProtocolCompatibilityUnavailableReason::ProofsDisabled,
        }
    );
}
