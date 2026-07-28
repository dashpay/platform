//! Generic state transition broadcast functionality.
//!
//! This module provides methods to broadcast any state transition
//! to the network and wait for the result.

use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use crate::settings::{parse_put_settings, PutSettingsJs};
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::ContextProvider;
use std::collections::BTreeSet;
use wasm_bindgen::prelude::*;
use wasm_dpp2::state_transitions::proof_result::{
    convert_proof_result, StateTransitionProofResultTypeJs,
};
use wasm_dpp2::StateTransitionWasm;

fn referenced_contract_ids(state_transition: &StateTransition) -> BTreeSet<Identifier> {
    match state_transition {
        StateTransition::Batch(batch_transition) => batch_transition
            .transitions_iter()
            .map(|transition| transition.data_contract_id())
            .collect(),
        _ => BTreeSet::new(),
    }
}

impl WasmSdk {
    /// Whether the context provider can already supply this contract, either
    /// from its cache or from a definition compiled into the SDK.
    fn can_resolve_contract(&self, contract_id: Identifier) -> bool {
        self.trusted_context()
            .and_then(|context| {
                context
                    .get_data_contract(&contract_id, self.version())
                    .ok()
                    .flatten()
            })
            .is_some()
    }
}

impl WasmSdk {
    async fn prepare_state_transition_context(
        &self,
        state_transition: &StateTransition,
    ) -> Result<(), WasmSdkError> {
        if let Some(context) = self.trusted_context() {
            if let Err(error) = context.refresh_quorums().await {
                tracing::warn!(
                    error = %error,
                    "Failed to refresh trusted quorum cache before proof verification; using cached keys"
                );
            }
        }

        for contract_id in referenced_contract_ids(state_transition) {
            // A contract the provider already resolves is left alone: fetching
            // it would let a node-supplied copy shadow a cached or compiled-in
            // definition, which decides verification when proofs are disabled.
            // Which system contracts are compiled in depends on cargo features
            // — the withdrawals contract is not among the wasm defaults — so
            // this asks the provider rather than assuming.
            if self.can_resolve_contract(contract_id) {
                continue;
            }

            // Only an unresolvable contract reaches the network. This runs
            // after the transition is broadcast, where refetching what we
            // already hold would let one transient failure discard a result
            // the network already accepted. A contract that can be neither
            // resolved nor fetched stays fatal: the proof needs it.
            self.refresh_contract(contract_id).await?;
        }

        Ok(())
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Broadcasts a state transition to the network.
    ///
    /// This method only broadcasts but does not wait for the result.
    /// Use `waitForResponse` to wait for confirmation after broadcasting,
    /// or use `broadcastAndWait` to do both in one call.
    ///
    /// @param stateTransition - The state transition to broadcast
    /// @param settings - Optional put settings (retries, timeout)
    #[wasm_bindgen(js_name = "broadcastStateTransition")]
    pub async fn broadcast_state_transition(
        &self,
        #[wasm_bindgen(js_name = "stateTransition")] state_transition: &StateTransitionWasm,
        settings: Option<PutSettingsJs>,
    ) -> Result<(), WasmSdkError> {
        let st: StateTransition = state_transition.into();
        let put_settings = parse_put_settings(settings)?;

        st.broadcast(self.as_ref(), put_settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast: {}", e)))?;

        Ok(())
    }

    /// Waits for a state transition response after it has been broadcast.
    ///
    /// Use this after calling `broadcastStateTransition` to wait for the transition
    /// to be processed by the network. This is useful when you want to broadcast
    /// and wait separately (e.g., for monitoring or progress tracking).
    ///
    /// Note: This differs from `waitForStateTransitionResult` which takes a hash string.
    /// This method takes the full state transition object and performs proof verification.
    ///
    /// @param stateTransition - The state transition that was broadcast
    /// @param settings - Optional put settings (retries, timeout, waitTimeoutMs)
    /// @returns The verified state transition result
    #[wasm_bindgen(js_name = "waitForResponse")]
    pub async fn wait_for_response(
        &self,
        #[wasm_bindgen(js_name = "stateTransition")] state_transition: &StateTransitionWasm,
        settings: Option<PutSettingsJs>,
    ) -> Result<StateTransitionProofResultTypeJs, WasmSdkError> {
        let st: StateTransition = state_transition.into();
        let put_settings = parse_put_settings(settings)?;
        self.prepare_state_transition_context(&st).await?;

        // Preserve the error kind: callers distinguish `ExecutionNotProved`,
        // which reports that this transition family has no execution proof
        // rather than that anything went wrong, from a genuine failure.
        let result = st
            .wait_for_response::<StateTransitionProofResult>(self.as_ref(), put_settings)
            .await
            .map_err(WasmSdkError::from)?;

        convert_proof_result(result).map_err(WasmSdkError::from)
    }

    /// Broadcasts a state transition and waits for the result.
    ///
    /// This method prepares proof context, broadcasts the transition, and waits
    /// for confirmation from the network. Returns once the transition has been
    /// processed or fails. Unlike separate broadcast and wait calls, proof
    /// context preparation happens before broadcasting.
    ///
    /// @param stateTransition - The state transition to broadcast
    /// @param settings - Optional put settings (retries, timeout, waitTimeoutMs)
    /// @returns The verified state transition result
    #[wasm_bindgen(js_name = "broadcastAndWait")]
    pub async fn broadcast_and_wait(
        &self,
        #[wasm_bindgen(js_name = "stateTransition")] state_transition: &StateTransitionWasm,
        settings: Option<PutSettingsJs>,
    ) -> Result<StateTransitionProofResultTypeJs, WasmSdkError> {
        let st: StateTransition = state_transition.into();
        let put_settings = parse_put_settings(settings)?;
        self.prepare_state_transition_context(&st).await?;

        let result = st
            .broadcast_and_wait::<StateTransitionProofResult>(self.as_ref(), put_settings)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast: {}", e)))?;

        convert_proof_result(result).map_err(WasmSdkError::from)
    }

    /// Waits for a state transition response, accepting proofs that only
    /// authenticate the state the transition affects.
    ///
    /// `waitForResponse` is strict: it fails for the transition families
    /// whose proofs cannot be bound to the execution of one specific
    /// transition (balance top-ups, credit transfers and withdrawals,
    /// address funds movements, shields, no-history token operations). This
    /// method accepts those outcomes instead. The result is a verified,
    /// height-pinned snapshot of the affected state — NOT evidence that this
    /// specific transition executed.
    ///
    /// @param stateTransition - The state transition that was broadcast
    /// @param settings - Optional put settings (retries, timeout, waitTimeoutMs)
    /// @returns The verified affected-state result
    #[wasm_bindgen(js_name = "waitForAffectedState")]
    pub async fn wait_for_affected_state(
        &self,
        #[wasm_bindgen(js_name = "stateTransition")] state_transition: &StateTransitionWasm,
        settings: Option<PutSettingsJs>,
    ) -> Result<StateTransitionProofResultTypeJs, WasmSdkError> {
        let st: StateTransition = state_transition.into();
        let put_settings = parse_put_settings(settings)?;
        self.prepare_state_transition_context(&st).await?;

        let result = st
            .wait_for_affected_state::<StateTransitionProofResult>(self.as_ref(), put_settings)
            .await
            .map_err(|e| {
                WasmSdkError::generic(format!("Failed to wait for state transition result: {}", e))
            })?;

        convert_proof_result(result).map_err(WasmSdkError::from)
    }

    /// Broadcasts a state transition and waits for the result, accepting
    /// proofs that only authenticate the affected state (see
    /// `waitForAffectedState` for the semantics).
    ///
    /// @param stateTransition - The state transition to broadcast
    /// @param settings - Optional put settings (retries, timeout, waitTimeoutMs)
    /// @returns The verified affected-state result
    #[wasm_bindgen(js_name = "broadcastAndWaitForAffectedState")]
    pub async fn broadcast_and_wait_for_affected_state(
        &self,
        #[wasm_bindgen(js_name = "stateTransition")] state_transition: &StateTransitionWasm,
        settings: Option<PutSettingsJs>,
    ) -> Result<StateTransitionProofResultTypeJs, WasmSdkError> {
        let st: StateTransition = state_transition.into();
        let put_settings = parse_put_settings(settings)?;
        self.prepare_state_transition_context(&st).await?;

        let result = st
            .broadcast_and_wait_for_affected_state::<StateTransitionProofResult>(
                self.as_ref(),
                put_settings,
            )
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast: {}", e)))?;

        convert_proof_result(result).map_err(WasmSdkError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_provider::WasmTrustedContext;
    use crate::sdk::WasmSdkBuilder;
    use dash_sdk::dpp::state_transition::batch_transition::batched_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dash_sdk::dpp::state_transition::batch_transition::batched_transition::document_base_transition::DocumentBaseTransition;
    use dash_sdk::dpp::state_transition::batch_transition::batched_transition::document_delete_transition::v0::DocumentDeleteTransitionV0;
    use dash_sdk::dpp::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransition;
    use dash_sdk::dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
    use dash_sdk::dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV0};
    use dash_sdk::dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use dash_sdk::dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dash_sdk::dpp::data_contract::accessors::v0::{
        DataContractV0Getters, DataContractV0Setters,
    };
    use dash_sdk::dpp::platform_value::BinaryData;
    use dash_sdk::dpp::prelude::DataContract;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::Sdk;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::{Duration, Instant};

    fn accept_before(listener: &TcpListener, deadline: Instant) -> TcpStream {
        loop {
            match listener.accept() {
                Ok((stream, _)) => return stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("accept quorum request: {}", error),
            }
        }
    }

    fn spawn_rotated_quorum_endpoint() -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock quorum endpoint");
        listener
            .set_nonblocking(true)
            .expect("make mock endpoint bounded");
        let address = listener.local_addr().expect("read mock endpoint address");

        let handle = thread::spawn(move || {
            let current = serde_json::json!({
                "success": true,
                "data": [{
                    "quorum_hash": hex::encode([0x88; 32]),
                    "key": hex::encode([0x98; 48]),
                    "height": 1,
                    "valid_members_count": 3
                }]
            })
            .to_string();
            let previous = serde_json::json!({
                "success": true,
                "data": {
                    "height": 1,
                    "quorums": []
                }
            })
            .to_string();

            for (expected_path, body) in [("/quorums", current), ("/previous", previous)] {
                let mut stream = accept_before(&listener, Instant::now() + Duration::from_secs(5));
                let mut reader =
                    BufReader::new(stream.try_clone().expect("clone quorum request stream"));
                let mut request_line = String::new();
                reader
                    .read_line(&mut request_line)
                    .expect("read quorum request line");
                assert_eq!(request_line.split_whitespace().nth(1), Some(expected_path));

                loop {
                    let mut header = String::new();
                    reader
                        .read_line(&mut header)
                        .expect("read quorum request header");
                    if header == "\r\n" || header.is_empty() {
                        break;
                    }
                }

                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write quorum response");
                stream.flush().expect("flush quorum response");
            }
        });

        (format!("http://{}", address), handle)
    }

    fn delete_transition(contract_id: Identifier, nonce: u64) -> DocumentTransition {
        DocumentTransition::Delete(DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                id: Identifier::new([nonce as u8; 32]),
                identity_contract_nonce: nonce,
                document_type_name: "note".to_string(),
                data_contract_id: contract_id,
            }),
        }))
    }

    #[test]
    fn should_collect_each_referenced_contract_of_a_document_batch_once_in_order() {
        let first_contract_id = Identifier::new([0x11; 32]);
        let second_contract_id = Identifier::new([0x22; 32]);
        let state_transition = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Identifier::new([0x33; 32]),
            transitions: vec![
                delete_transition(first_contract_id, 1),
                delete_transition(second_contract_id, 2),
                delete_transition(first_contract_id, 3),
            ],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }));

        assert_eq!(
            referenced_contract_ids(&state_transition)
                .into_iter()
                .collect::<Vec<_>>(),
            vec![first_contract_id, second_contract_id],
        );
    }

    #[test]
    fn should_not_prepare_contracts_for_non_batch_transition() {
        let state_transition = StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
            IdentityTopUpTransitionV0::default(),
        ));

        assert!(referenced_contract_ids(&state_transition).is_empty());
    }

    #[tokio::test]
    async fn should_refresh_rotated_quorums_through_proof_preparation() {
        let (base_url, server) = spawn_rotated_quorum_endpoint();
        let context = WasmTrustedContext::for_testing_with_url(vec![], base_url);
        let sdk = WasmSdkBuilder::new_local()
            .with_trusted_context(&context)
            .build()
            .expect("build local SDK with trusted context");
        let state_transition = StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
            IdentityTopUpTransitionV0::default(),
        ));

        assert!(context.get_quorum_public_key(1, [0x88; 32], 1).is_err());
        sdk.prepare_state_transition_context(&state_transition)
            .await
            .expect("proof preparation must refresh quorums");
        assert_eq!(
            context
                .get_quorum_public_key(1, [0x88; 32], 1)
                .expect("rotated quorum must be available after preparation"),
            [0x98; 48]
        );
        server.join().expect("mock quorum server must finish");
    }

    fn custom_contract(id_byte: u8, version: u32) -> DataContract {
        let mut contract =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("DPNS contract fixture should load");
        contract.set_id(Identifier::new([id_byte; 32]));
        contract.set_version(version);
        contract
    }

    #[tokio::test]
    async fn should_cache_a_referenced_contract_missing_from_the_context() {
        let expected = custom_contract(0x66, 1);
        let contract_id = expected.id();
        let mut inner_sdk = Sdk::new_mock();
        inner_sdk
            .mock()
            .expect_fetch(contract_id, Some(expected.clone()))
            .await
            .expect("mock contract response should be configured");

        let context = WasmTrustedContext::for_testing(vec![]);
        let sdk = WasmSdk::new_for_testing(inner_sdk, Some(context));
        let state_transition = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Identifier::new([0x33; 32]),
            transitions: vec![delete_transition(contract_id, 1)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }));

        assert!(sdk.get_cached_contract(&contract_id).is_none());
        sdk.prepare_state_transition_context(&state_transition)
            .await
            .expect("preparation must fetch the referenced contract");
        assert_eq!(
            sdk.get_cached_contract(&contract_id)
                .expect("referenced contract must be cached after preparation")
                .as_ref(),
            &expected,
        );
    }

    #[tokio::test]
    async fn should_fetch_a_referenced_system_contract_the_sdk_does_not_compile_in() {
        // Which system contracts are compiled in is a build-time choice, and
        // the withdrawals contract is not among the wasm defaults. The provider
        // cannot serve it, so preparation has to fetch it like any other
        // contract or document verification fails on an unknown contract.
        let withdrawals_id = SystemDataContract::Withdrawals.id();
        let mut expected =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("DPNS contract fixture should load");
        expected.set_id(withdrawals_id);

        let mut inner_sdk = Sdk::new_mock();
        inner_sdk
            .mock()
            .expect_fetch(withdrawals_id, Some(expected.clone()))
            .await
            .expect("mock contract response should be configured");

        let sdk = WasmSdk::new_for_testing(inner_sdk, Some(WasmTrustedContext::for_testing(vec![])));
        let state_transition = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Identifier::new([0x33; 32]),
            transitions: vec![delete_transition(withdrawals_id, 1)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }));

        sdk.prepare_state_transition_context(&state_transition)
            .await
            .expect("preparation must fetch a system contract the SDK cannot resolve");
        assert_eq!(
            sdk.get_cached_contract(&withdrawals_id)
                .expect("fetched system contract must be cached after preparation")
                .as_ref(),
            &expected,
        );
    }

    #[tokio::test]
    async fn should_keep_the_compiled_in_definition_of_a_referenced_system_contract() {
        let dpns_id = SystemDataContract::DPNS.id();
        // The mock SDK has no response configured, so any fetch of this id fails
        // the preparation and proves the system contract was not requested.
        let sdk = WasmSdk::new_for_testing(
            Sdk::new_mock(),
            Some(WasmTrustedContext::for_testing(vec![])),
        );
        let state_transition = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id: Identifier::new([0x33; 32]),
            transitions: vec![delete_transition(dpns_id, 1)],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }));

        sdk.prepare_state_transition_context(&state_transition)
            .await
            .expect("preparation must skip compiled-in system contracts");
        assert!(sdk.get_cached_contract(&dpns_id).is_none());
    }
}
