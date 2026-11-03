use crate::error::WasmSdkErrorKind;
use crate::sdk::WasmSdk;
use dapi_grpc::platform::v0::{
    wait_for_state_transition_result_response::{
        wait_for_state_transition_result_response_v0::Result as WaitResult, Version,
    },
    BroadcastStateTransitionRequest, BroadcastStateTransitionResponse, Proof,
    WaitForStateTransitionResultRequest, WaitForStateTransitionResultResponse,
};
use dapi_grpc::tonic::{Code, Status};
use dapi_grpc::Message;
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::data_contract::{DataContract, TokenConfiguration};
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted;
use dash_sdk::dpp::state_transition::proof_result::{
    StateTransitionProofGuarantee, StateTransitionProofResult,
};
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::platform::transition::broadcast::{BroadcastStateTransition, WaitForOutcome};
use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use dash_sdk::platform::ContextProvider;
use dash_sdk::{error::ContextProviderError, Error as SdkError, SdkBuilder};
use rs_dapi_client::mock::MockResult;
use rs_dapi_client::transport::{TransportError, TransportRequest};
use rs_dapi_client::{
    DapiClientError, DumpData, ExecutionError, ExecutionResponse, RequestSettings,
};
use std::path::Path;
use std::sync::Arc;
use wasm_dpp2::StateTransitionWasm;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Vectors {
    fixture_format_version: u32,
    protocol_version: u32,
    generator: String,
    inputs: Inputs,
    quorum_public_key: Vec<u8>,
    contract_bytes: Vec<u8>,
    snapshot: Case,
    execution: Case,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Inputs {
    height: u64,
    time_ms: u64,
    core_chain_locked_height: u32,
}

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    transition_bytes: Vec<u8>,
    broadcast_request_bytes: Vec<u8>,
    wait_request_bytes: Vec<u8>,
    response: WaitForStateTransitionResultResponse,
    response_bytes: Vec<u8>,
    state_root: Vec<u8>,
}

fn vectors() -> Vectors {
    let vectors: Vectors = serde_json::from_str(include_str!(
        "../../tests/fixtures/state-transition-waits/signed-responses.json"
    ))
    .expect("signed wait vectors");
    assert_eq!(vectors.fixture_format_version, 1);
    assert_eq!(
        vectors.protocol_version,
        PlatformVersion::latest().protocol_version
    );
    assert_eq!(
        vectors.generator,
        "packages/rs-drive-abci/examples/generate_wasm_state_transition_wait_vectors.rs"
    );
    vectors
}

struct Provider {
    contract: Arc<DataContract>,
    quorum_key: [u8; 48],
}

impl ContextProvider for Provider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok((*id == Identifier::new([0x41; 32])).then(|| self.contract.clone()))
    }

    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        assert_eq!(quorum_type, 106);
        assert_eq!(quorum_hash, [0x46; 32]);
        assert_eq!(core_chain_locked_height, 1);
        Ok(self.quorum_key)
    }

    fn get_platform_activation_height(&self) -> Result<u32, ContextProviderError> {
        Ok(1)
    }
}

enum Failure {
    None,
    Broadcast,
    Wait,
}

struct Setup {
    sdk: WasmSdk,
    transition: StateTransition,
    broadcast: BroadcastStateTransitionRequest,
    wait: WaitForStateTransitionResultRequest,
    _directory: tempfile::TempDir,
}

fn dump<R: TransportRequest>(directory: &Path, request: &R, result: MockResult<R>) {
    let data = DumpData::new(request, &result);
    data.save(&directory.join(data.filename().expect("request filename")))
        .expect("raw transport expectation");
}

fn transport_failure() -> ExecutionError<DapiClientError> {
    ExecutionError {
        inner: DapiClientError::Transport(TransportError::Grpc(Status::new(
            Code::InvalidArgument,
            "request refused",
        ))),
        retries: 0,
        address: Some("http://127.0.0.1:9000".parse().expect("transport address")),
    }
}

fn fixture(execution_proved: bool, failure: Failure, mutate: impl FnOnce(&mut Proof)) -> Setup {
    let vectors = vectors();
    let pv = PlatformVersion::latest();
    let contract =
        DataContract::versioned_deserialize_untrusted(&vectors.contract_bytes, false, pv)
            .expect("fixture contract");
    let mut inner = SdkBuilder::new_mock()
        .with_version(pv)
        .with_network(Network::Testnet)
        .with_context_provider(Provider {
            contract: Arc::new(contract),
            quorum_key: vectors
                .quorum_public_key
                .try_into()
                .expect("quorum public key"),
        })
        .with_time_tolerance(None)
        .with_settings(RequestSettings {
            retries: Some(0),
            ..Default::default()
        })
        .build()
        .expect("mock SDK");
    let case = if execution_proved {
        vectors.execution
    } else {
        vectors.snapshot
    };
    assert_eq!(case.state_root.len(), 32);
    assert_eq!(case.response.encode_to_vec(), case.response_bytes);
    let transition = StateTransition::deserialize_from_bytes_untrusted_exact_in_version(
        &case.transition_bytes,
        pv,
    )
    .expect("signed fixture transition");
    let broadcast = transition
        .broadcast_request_for_state_transition()
        .expect("broadcast request");
    let wait = transition
        .wait_for_state_transition_result_request()
        .expect("proof wait request");
    assert_eq!(broadcast.encode_to_vec(), case.broadcast_request_bytes);
    assert_eq!(wait.encode_to_vec(), case.wait_request_bytes);
    let mut response = case.response;
    mutate(proof(&mut response));
    let directory = tempfile::TempDir::new().expect("transport fixture directory");
    let broadcast_result: MockResult<BroadcastStateTransitionRequest> = match failure {
        Failure::Broadcast => Err(transport_failure()),
        _ => Ok(ExecutionResponse {
            inner: BroadcastStateTransitionResponse::default(),
            retries: 0,
            address: "http://127.0.0.1:9000".parse().expect("transport address"),
        }),
    };
    dump(directory.path(), &broadcast, broadcast_result);
    let wait_result: MockResult<WaitForStateTransitionResultRequest> = match failure {
        Failure::Wait => Err(transport_failure()),
        _ => Ok(ExecutionResponse {
            inner: response,
            retries: 0,
            address: "http://127.0.0.1:9000".parse().expect("transport address"),
        }),
    };
    dump(directory.path(), &wait, wait_result);
    inner
        .mock()
        .load_expectations_sync(directory.path())
        .expect("raw broadcast and wait responses");
    Setup {
        sdk: WasmSdk::new_for_testing(inner, None),
        transition,
        broadcast,
        wait,
        _directory: directory,
    }
}

fn proof(response: &mut WaitForStateTransitionResultResponse) -> &mut Proof {
    let Version::V0(response) = response.version.as_mut().expect("response version");
    match response.result.as_mut().expect("wait result") {
        WaitResult::Proof(proof) => proof,
        _ => panic!("fixture must carry a proof"),
    }
}

async fn assert_counts(setup: &Setup, broadcasts: usize, waits: usize) {
    assert_eq!(
        setup
            .sdk
            .inner_sdk()
            .clone()
            .mock()
            .request_count(&setup.broadcast)
            .await,
        broadcasts
    );
    assert_eq!(
        setup
            .sdk
            .inner_sdk()
            .clone()
            .mock()
            .request_count(&setup.wait)
            .await,
        waits
    );
}

#[tokio::test]
async fn should_authenticate_wait_vectors_before_applying_a_guarantee_policy() {
    for execution in [false, true] {
        let setup = fixture(execution, Failure::None, |_| {});
        let (outcome, metadata) = setup
            .transition
            .wait_for_outcome_with_metadata(setup.sdk.inner_sdk(), None)
            .await
            .expect("GroveDB proof and quorum signature must authenticate");
        assert_eq!(
            outcome.guarantee(),
            if execution {
                StateTransitionProofGuarantee::ExecutionProved
            } else {
                StateTransitionProofGuarantee::AffectedState
            }
        );
        assert_eq!(metadata.height, 100);
        assert_eq!(metadata.height, vectors().inputs.height);
        assert_eq!(metadata.time_ms, vectors().inputs.time_ms);
        assert_eq!(
            metadata.core_chain_locked_height,
            vectors().inputs.core_chain_locked_height
        );
        assert_counts(&setup, 0, 1).await;
    }
}

#[tokio::test]
async fn should_report_execution_not_proved_after_broadcast() {
    let setup = fixture(false, Failure::None, |_| {});
    let error = setup
        .sdk
        .broadcast_and_wait(&StateTransitionWasm::from(setup.transition.clone()), None)
        .await
        .err()
        .expect("strict wait must refuse an authenticated snapshot");
    assert_eq!(error.kind(), WasmSdkErrorKind::ExecutionNotProved);
    assert_eq!(error.code(), -1);
    assert!(!error.is_retriable());
    assert!(error.message().contains("waitForAffectedState"));
    assert_counts(&setup, 1, 1).await;
    assert!(
        !error.message().contains("Failed to broadcast"),
        "accepted submission must not be labelled a broadcast failure: {}",
        error.message()
    );
}

#[tokio::test]
async fn should_preserve_strict_and_affected_state_rust_waits() {
    for execution in [false, true] {
        for combined in [false, true] {
            let setup = fixture(execution, Failure::None, |_| {});
            let strict = if combined {
                setup
                    .transition
                    .broadcast_and_wait::<StateTransitionProofResult>(setup.sdk.inner_sdk(), None)
                    .await
            } else {
                setup
                    .transition
                    .wait_for_response::<StateTransitionProofResult>(setup.sdk.inner_sdk(), None)
                    .await
            };
            if execution {
                strict.expect("execution proof must satisfy a strict wait");
            } else {
                assert!(matches!(strict, Err(SdkError::ExecutionNotProved(_))));
            }
            let setup = fixture(execution, Failure::None, |_| {});
            let affected = if combined {
                setup.transition
                    .broadcast_and_wait_for_affected_state_with_metadata::<StateTransitionProofResult>(setup.sdk.inner_sdk(), None)
                    .await
            } else {
                setup
                    .transition
                    .wait_for_affected_state_with_metadata::<StateTransitionProofResult>(
                        setup.sdk.inner_sdk(),
                        None,
                    )
                    .await
            };
            let (_, metadata) =
                affected.expect("affected-state policy must accept both authenticated guarantees");
            assert_eq!(metadata.height, 100);
            assert_counts(&setup, usize::from(combined), 1).await;
        }
    }
}

#[tokio::test]
async fn should_keep_actual_broadcast_refusals_in_the_broadcast_phase() {
    let setup = fixture(false, Failure::Broadcast, |_| {});
    let error = setup
        .sdk
        .broadcast_and_wait(&StateTransitionWasm::from(setup.transition.clone()), None)
        .await
        .err()
        .expect("broadcast must fail");
    assert_eq!(error.kind(), WasmSdkErrorKind::DapiClientError);
    assert_eq!(error.code(), -1);
    assert!(!error.is_retriable());
    assert!(error.message().starts_with("Failed to broadcast:"));
    assert_counts(&setup, 1, 0).await;
}

async fn assert_wait_failure_phase(affected: bool) {
    let setup = fixture(false, Failure::Wait, |_| {});
    let transition = StateTransitionWasm::from(setup.transition.clone());
    let result = if affected {
        setup
            .sdk
            .broadcast_and_wait_for_affected_state(&transition, None)
            .await
    } else {
        setup.sdk.broadcast_and_wait(&transition, None).await
    };
    let error = result.err().expect("post-send transport must fail");
    assert_eq!(error.kind(), WasmSdkErrorKind::DapiClientError);
    assert_eq!(error.code(), -1);
    assert!(!error.is_retriable());
    assert_counts(&setup, 1, 1).await;
    assert!(
        error
            .message()
            .starts_with("Failed to wait for state transition result:"),
        "post-send failure must identify its wait phase: {}",
        error.message()
    );
}

#[tokio::test]
async fn should_report_strict_wait_failures_after_broadcast() {
    assert_wait_failure_phase(false).await;
}

#[tokio::test]
async fn should_report_affected_state_wait_failures_after_broadcast() {
    assert_wait_failure_phase(true).await;
}

#[tokio::test]
async fn should_reject_invalid_signature_and_structural_proofs_under_both_policies() {
    for invalid_signature in [false, true] {
        for affected in [false, true] {
            let setup = fixture(false, Failure::None, |p| {
                if invalid_signature {
                    let mut other = vectors().execution.response;
                    p.signature = proof(&mut other).signature.clone();
                } else {
                    p.grovedb_proof = vec![0xff];
                }
            });
            let result = if affected {
                setup
                    .transition
                    .wait_for_affected_state::<StateTransitionProofResult>(
                        setup.sdk.inner_sdk(),
                        None,
                    )
                    .await
            } else {
                setup
                    .transition
                    .wait_for_response::<StateTransitionProofResult>(setup.sdk.inner_sdk(), None)
                    .await
            };
            assert!(
                result.is_err(),
                "both policies must require authenticated proofs"
            );
            assert!(
                !matches!(result, Err(SdkError::ExecutionNotProved(_))),
                "invalid proofs must not be relabelled as a guarantee refusal"
            );
            assert_counts(&setup, 0, 1).await;
        }
    }
}
