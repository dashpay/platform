//! The wait for a delta contract update verifies its proof against the contract the delta
//! was built on, whatever the SDK's context provider holds by the time the proof arrives.

use super::{SdkBuilder, SdkInstance};
use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::waitable::{wait_for_data_contract_update, Waitable};
use crate::platform::ContextProvider;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
    wait_for_state_transition_result_response_v0::Result as WaitResult, Version as WaitVersion,
    WaitForStateTransitionResultResponseV0,
};
use dapi_grpc::platform::v0::{Proof, ResponseMetadata, WaitForStateTransitionResultResponse};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::PlatformVersion;
use rs_dapi_client::{ExecutionResponse, RequestSettings};
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;

/// A contract, the contract a delta turns it into, and the delta.
fn delta_update() -> (Arc<DataContract>, Arc<DataContract>, StateTransition) {
    let platform_version = PlatformVersion::latest();
    let original = load_system_data_contract(SystemDataContract::Dashpay, platform_version)
        .expect("expected the DashPay contract");
    let mut updated = original.clone();
    updated.set_version(original.version() + 1);
    updated.set_description(Some("payments between contacts".to_string()));

    let delta = DataContractUpdateTransition::from_contract_update(
        &original,
        &updated,
        1,
        platform_version,
    )
    .expect("expected a contract update");
    assert!(
        matches!(delta, DataContractUpdateTransition::V1(_)),
        "the latest protocol version builds a delta"
    );
    (Arc::new(original), Arc::new(updated), delta.into())
}

/// A context provider that keeps the latest version of a contract it is given, as the one a
/// wallet shares between its operations does.
fn shared_provider() -> Arc<TrustedHttpContextProvider> {
    Arc::new(
        TrustedHttpContextProvider::new_with_url(
            Network::Regtest,
            "http://127.0.0.1:1".to_string(),
            NonZeroUsize::new(10).expect("non-zero cache size"),
        )
        .expect("expected a trusted provider")
        .with_refetch_if_not_found(false),
    )
}

/// An SDK verifying with `provider`, whose node answers the wait for `transition` with a
/// well-formed proof of something else: verification gets as far as reading that proof only
/// when the verifier has the contract the delta is merged onto.
async fn sdk_answering_the_wait(
    transition: &StateTransition,
    provider: Arc<TrustedHttpContextProvider>,
) -> Sdk {
    let platform_version = PlatformVersion::latest();
    let recorded_proof = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../rs-drive-proof-verifier/tests/vectors/identity-balance/proof.hex"),
    )
    .expect("expected the recorded proof");
    let response = WaitForStateTransitionResultResponse {
        version: Some(WaitVersion::V0(WaitForStateTransitionResultResponseV0 {
            result: Some(WaitResult::Proof(Proof {
                grovedb_proof: hex::decode(recorded_proof.trim())
                    .expect("expected the recorded proof to be hex"),
                quorum_hash: vec![0; 32],
                signature: vec![0; 96],
                round: 0,
                block_id_hash: vec![0; 32],
                quorum_type: 106,
            })),
            metadata: Some(ResponseMetadata {
                height: 10,
                core_chain_locked_height: 1,
                epoch: 0,
                time_ms: 1_000,
                protocol_version: platform_version.protocol_version,
                chain_id: "test".to_string(),
            }),
        })),
    };

    let sdk = SdkBuilder::new_mock()
        .with_network(Network::Regtest)
        .with_version(platform_version)
        .build()
        .expect("expected a mock SDK");
    let SdkInstance::Mock { dapi, .. } = &sdk.inner else {
        unreachable!("the SDK was built as a mock")
    };
    dapi.lock()
        .await
        .expect(
            &transition
                .wait_for_state_transition_result_request()
                .expect("expected a wait request"),
            &Ok(ExecutionResponse {
                inner: response,
                address: "http://127.0.0.1:9000"
                    .parse()
                    .expect("expected an address"),
                retries: 0,
            }),
        )
        .expect("expected the mock node to take the response");
    sdk.set_context_provider(provider)
        .expect("expected the SDK to take the provider");
    sdk
}

fn no_retries() -> Option<PutSettings> {
    Some(PutSettings {
        request_settings: RequestSettings {
            retries: Some(0),
            ..Default::default()
        },
        ..Default::default()
    })
}

/// Whether verification stopped for want of the contract the delta is merged onto.
fn lacks_the_contract_the_delta_was_built_on(error: &Error) -> bool {
    error
        .to_string()
        .contains("is verified against the version before it")
}

/// The order of events this guards against: the wallet builds a delta on version N and
/// broadcasts it; the update is committed; another operation of the wallet fetches the
/// contract, now at version N + 1, and gives it to the shared provider; the proof of the
/// update arrives and is verified.
#[tokio::test]
async fn should_verify_a_delta_against_the_contract_it_was_built_on_after_the_provider_moved_on() {
    let (original, updated, transition) = delta_update();
    let provider = shared_provider();
    // What the wallet did before it broadcast, when the provider was the only way to tell
    // the verifier the contract
    provider.register_data_contract(Arc::clone(&original));
    let sdk = sdk_answering_the_wait(&transition, Arc::clone(&provider)).await;

    // Between the broadcast and the proof, another operation promotes the provider
    provider.register_data_contract(Arc::clone(&updated));
    // and the provider does not take the older contract back
    provider.register_data_contract(Arc::clone(&original));
    let held = provider
        .get_data_contract(&original.id(), PlatformVersion::latest())
        .expect("expected the provider to answer")
        .expect("expected the provider to know the contract");
    assert_eq!(held.version(), updated.version());

    // A wait that asks the provider has nothing to merge the delta onto
    let error = DataContract::wait_for_response(&sdk, transition.clone(), no_retries())
        .await
        .expect_err("the test's proof is not one of the update");
    assert!(
        lacks_the_contract_the_delta_was_built_on(&error),
        "got {error}"
    );

    // A wait that is given the contract merges the delta and goes on to read the proof,
    // which here proves something else
    let error = wait_for_data_contract_update(&sdk, &transition, &original, no_retries())
        .await
        .expect_err("the test's proof is not one of the update");
    assert!(
        !lacks_the_contract_the_delta_was_built_on(&error),
        "got {error}"
    );
    assert!(matches!(error, Error::Proof(_)), "got {error:?}");

    // The wait left the provider as other operations need it
    let held = provider
        .get_data_contract(&original.id(), PlatformVersion::latest())
        .expect("expected the provider to answer")
        .expect("expected the provider to know the contract");
    assert_eq!(held.version(), updated.version());
}
