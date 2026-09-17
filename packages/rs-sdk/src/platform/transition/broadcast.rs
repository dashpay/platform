use super::broadcast_request::BroadcastRequestForStateTransition;
use super::put_settings::PutSettings;
use crate::error::StateTransitionBroadcastError;
use crate::sync::retry;
use crate::sync::retry_with_additional_error;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0;
use dapi_grpc::platform::v0::BroadcastStateTransitionResponse;
use dapi_grpc::platform::v0::{
    wait_for_state_transition_result_response, BroadcastStateTransitionRequest, ResponseMetadata,
    WaitForStateTransitionResultResponse,
};
use dash_context_provider::ContextProviderError;
use dpp::consensus::signature::SignatureError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::proof_result::{
    StateTransitionProofOutcome, StateTransitionProofResult,
};
use dpp::state_transition::StateTransition;
use dpp::system_data_contracts::SystemDataContract;
use dpp::util::hash::hash_single;
use dpp::ProtocolError;
use drive_proof_verifier::FromProof;
use rs_dapi_client::AddressList;
use rs_dapi_client::CanRetry;
use rs_dapi_client::ExecutionResult;
use rs_dapi_client::WrapToExecutionResult;
use rs_dapi_client::{DapiRequest, ExecutionError, InnerInto, IntoInner, RequestSettings};
use std::future::Future;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use tracing::{info, trace, warn};

const DPNS_REGISTRATION_BROADCAST_RETRIES: usize = 2;

#[async_trait::async_trait]
pub trait BroadcastStateTransition {
    async fn broadcast(&self, sdk: &Sdk, settings: Option<PutSettings>) -> Result<(), Error>;
    /// Waits for the transition's result and verifies its proof STRICTLY:
    /// succeeds only when the proof establishes that this specific
    /// transition executed. For the transition families whose proofs can
    /// only authenticate the affected state (balance top-ups, credit
    /// transfers and withdrawals, address funds movements, shields,
    /// no-history token operations), this returns
    /// [`Error::ExecutionNotProved`] — use
    /// [`wait_for_affected_state`](Self::wait_for_affected_state) for those
    /// flows and treat the result as a height-pinned snapshot.
    async fn wait_for_response<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error>;
    /// Like [`wait_for_response`](Self::wait_for_response), but also
    /// returns the quorum-authenticated response metadata.
    /// `metadata.height` is the committed block the proof attests —
    /// callers that persist proof-attested absolute balances need it as
    /// the balance's height pin
    /// (`dash_sdk::platform::address_sync::AddressFunds::as_of_height`).
    async fn wait_for_response_with_metadata<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error>;
    /// Waits for the transition's result, accepting proofs that only
    /// authenticate the state the transition affects (keys derived from the
    /// transition, values as of the proof's block). The result is a
    /// verified, height-pinned snapshot — NOT evidence that this specific
    /// transition executed. Execution-proved outcomes are accepted too,
    /// since they carry the strictly stronger guarantee.
    async fn wait_for_affected_state<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error>;
    /// Like [`wait_for_affected_state`](Self::wait_for_affected_state), but
    /// also returns the quorum-authenticated response metadata (see
    /// [`wait_for_response_with_metadata`](Self::wait_for_response_with_metadata)
    /// for why callers persisting balances need `metadata.height`).
    async fn wait_for_affected_state_with_metadata<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error>;
    /// Broadcasts and then waits STRICTLY (see
    /// [`wait_for_response`](Self::wait_for_response)).
    async fn broadcast_and_wait<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error>;
    /// Like [`broadcast_and_wait`](Self::broadcast_and_wait), but also
    /// returns the quorum-authenticated response metadata (see
    /// [`wait_for_response_with_metadata`](Self::wait_for_response_with_metadata)).
    async fn broadcast_and_wait_with_metadata<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error>;
    /// Broadcasts and then waits, accepting affected-state snapshots (see
    /// [`wait_for_affected_state`](Self::wait_for_affected_state)).
    async fn broadcast_and_wait_for_affected_state<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error>;
    /// Like [`broadcast_and_wait_for_affected_state`](Self::broadcast_and_wait_for_affected_state),
    /// but also returns the quorum-authenticated response metadata.
    async fn broadcast_and_wait_for_affected_state_with_metadata<
        T: TryFrom<StateTransitionProofResult> + Send,
    >(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error>;
}

#[async_trait::async_trait]
impl BroadcastStateTransition for StateTransition {
    async fn broadcast(&self, sdk: &Sdk, settings: Option<PutSettings>) -> Result<(), Error> {
        trace!(
            state_transition = %self.name(),
            transaction_id = %self
                .transaction_id()
                .map(hex::encode)
                .unwrap_or("UNKNOWN".to_string()),
            "broadcast: start"
        );

        let retry_settings = match settings {
            Some(s) => sdk.dapi_client_settings.override_by(s.request_settings),
            None => sdk.dapi_client_settings,
        };

        let execute = |request: BroadcastStateTransitionRequest, request_settings| async move {
            trace!("broadcast: executing request");
            let result = request
                .execute(sdk, request_settings)
                .await
                .map_err(|e| e.inner_into());

            match &result {
                Ok(_) => trace!("broadcast: request succeeded"),
                Err(e) => warn!(error = ?e, "broadcast: request failed"),
            }
            result
        };

        // response is empty for a broadcast, result comes from the stream wait for state transition result
        trace!("broadcast: starting retry mechanism");
        let result = broadcast_with_retries(self, sdk.address_list(), retry_settings, execute)
            .await
            .into_inner()
            .map(|_| ());

        match &result {
            Ok(_) => trace!("broadcast: completed successfully"),
            Err(e) => {
                warn!(error = ?e, "broadcast: failed after retries");
                if let Some(owner_id) = self.owner_id() {
                    sdk.refresh_identity_nonce(&owner_id).await;
                }
            }
        }
        result
    }
    async fn wait_for_response<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        self.wait_for_response_with_metadata::<T>(sdk, settings)
            .await
            .map(|(result, _metadata)| result)
    }

    async fn wait_for_response_with_metadata<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error> {
        let (outcome, metadata) = self.wait_for_outcome_with_metadata(sdk, settings).await?;
        require_execution_proved(outcome)
            .and_then(convert_proof_result::<T>)
            .map(|converted| (converted, metadata))
    }

    async fn wait_for_affected_state<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        self.wait_for_affected_state_with_metadata::<T>(sdk, settings)
            .await
            .map(|(result, _metadata)| result)
    }

    async fn wait_for_affected_state_with_metadata<
        T: TryFrom<StateTransitionProofResult> + Send,
    >(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error> {
        let (outcome, metadata) = self.wait_for_outcome_with_metadata(sdk, settings).await?;
        // An execution-proved outcome carries a strictly stronger guarantee
        // than the requested snapshot, so both tags are accepted here.
        convert_proof_result::<T>(outcome.into_result()).map(|converted| (converted, metadata))
    }

    async fn broadcast_and_wait_for_affected_state<
        T: TryFrom<StateTransitionProofResult> + Send,
    >(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        self.broadcast_and_wait_for_affected_state_with_metadata::<T>(sdk, settings)
            .await
            .map(|(result, _metadata)| result)
    }

    async fn broadcast_and_wait_for_affected_state_with_metadata<
        T: TryFrom<StateTransitionProofResult> + Send,
    >(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error> {
        trace!(state_transition = %self.name(), "broadcast_and_wait_for_affected_state: start");
        self.broadcast(sdk, settings).await?;
        let result = self
            .wait_for_affected_state_with_metadata::<T>(sdk, settings)
            .await;
        match &result {
            Ok(_) => trace!("broadcast_and_wait_for_affected_state: complete success"),
            Err(e) => warn!(error = ?e, "broadcast_and_wait_for_affected_state: failed"),
        }
        result
    }

    async fn broadcast_and_wait<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        self.broadcast_and_wait_with_metadata::<T>(sdk, settings)
            .await
            .map(|(result, _metadata)| result)
    }

    async fn broadcast_and_wait_with_metadata<T: TryFrom<StateTransitionProofResult> + Send>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(T, ResponseMetadata), Error> {
        trace!(state_transition = %self.name(), "broadcast_and_wait: start");
        trace!("broadcast_and_wait: step 1 - broadcasting");
        self.broadcast(sdk, settings).await?;
        trace!("broadcast_and_wait: step 2 - waiting for response");
        let result = self
            .wait_for_response_with_metadata::<T>(sdk, settings)
            .await;
        match &result {
            Ok(_) => trace!("broadcast_and_wait: complete success"),
            Err(e) => warn!(error = ?e, "broadcast_and_wait: failed"),
        }
        result
    }
}

/// DPNS registration has two separate creates. Fail over each signed create,
/// never restart the registration (which would generate a new preorder salt).
/// This applies only to the immediate CheckTx rejection, not wait outcomes.
async fn broadcast_with_retries<Fut, Execute>(
    transition: &StateTransition,
    addresses: &AddressList,
    mut settings: RequestSettings,
    mut execute: Execute,
) -> ExecutionResult<BroadcastStateTransitionResponse, Error>
where
    Fut: Future<Output = ExecutionResult<BroadcastStateTransitionResponse, Error>> + Send,
    Execute: FnMut(BroadcastStateTransitionRequest, RequestSettings) -> Fut + Send,
{
    let dpns_stage = dpns_registration_document_type(transition);
    let is_dpns_registration = dpns_stage.is_some();
    if is_dpns_registration {
        // Include any lower-level transport retries in the same three-send
        // budget. An explicitly smaller caller budget must still win.
        settings.retries = Some(
            settings
                .finalize()
                .retries
                .min(DPNS_REGISTRATION_BROADCAST_RETRIES),
        );
    }
    let request = transition
        .broadcast_request_for_state_transition()
        .map_err(|inner| ExecutionError {
            inner,
            address: None,
            retries: 0,
        })?;
    // Same DPP helper and signed serialization as StateTransition::transaction_id().
    let dpns_transaction_id =
        dpns_stage.map(|_| hex::encode(hash_single(&request.state_transition)));
    let transaction_id = dpns_transaction_id.as_deref();
    let broadcast_attempts = AtomicUsize::new(0);
    let attempts = &broadcast_attempts;
    let factory = |settings| {
        let response = execute(request.clone(), settings);
        async move {
            let result = response.await;
            // Successful retry results only report the lower layer's latest
            // attempt count. Accumulate every dispatch, including its transport
            // retries; running out of addresses adds no new dispatch.
            if let (Some(stage), Some(transaction_id)) = (dpns_stage, transaction_id) {
                let sent = match &result {
                    Ok(response) => response.retries.saturating_add(1),
                    Err(error) => error
                        .retries
                        .saturating_add(usize::from(!error.is_no_available_addresses())),
                };
                let attempts = attempts
                    .fetch_add(sent, Ordering::Relaxed)
                    .saturating_add(sent);
                match &result {
                    Ok(response) => info!(
                        stage,
                        transaction_id,
                        node = %response.address,
                        attempts,
                        result = "accepted_for_processing",
                        "DPNS registration: broadcast attempt finished"
                    ),
                    Err(error) => warn!(
                        stage,
                        transaction_id,
                        node = ?error.address,
                        attempts,
                        result = "failed",
                        "DPNS registration: broadcast attempt finished"
                    ),
                }
            }
            result
        }
    };
    retry_with_additional_error(addresses, settings, factory, |error| {
        is_dpns_registration && is_missing_transition_owner(transition, error)
    })
    .await
}

fn dpns_registration_document_type(transition: &StateTransition) -> Option<&str> {
    let StateTransition::Batch(batch) = transition else {
        return None;
    };
    if batch.transitions_len() != 1 {
        return None;
    }
    let BatchedTransitionRef::Document(document @ DocumentTransition::Create(_)) =
        batch.first_transition()?
    else {
        return None;
    };
    if document.data_contract_id() != SystemDataContract::DPNS.id() {
        return None;
    }
    match document.document_type_name().as_str() {
        name @ ("preorder" | "domain") => Some(name),
        _ => None,
    }
}

fn is_missing_transition_owner(transition: &StateTransition, error: &Error) -> bool {
    let Error::Protocol(ProtocolError::ConsensusError(consensus)) = error else {
        return false;
    };
    matches!(consensus.as_ref(),
        ConsensusError::SignatureError(SignatureError::IdentityNotFoundError(missing))
            if Some(missing.identity_id()) == transition.owner_id()
    )
}

/// Reject snapshot outcomes for the strict wait APIs with a typed error.
fn require_execution_proved(
    outcome: StateTransitionProofOutcome,
) -> Result<StateTransitionProofResult, Error> {
    match outcome {
        StateTransitionProofOutcome::ExecutionProved(result) => Ok(result),
        StateTransitionProofOutcome::AffectedState(result) => Err(Error::ExecutionNotProved(
            format!(
                "received a verified {} snapshot for this transition family; use the *_affected_state wait APIs and treat the result as a height-pinned snapshot",
                result
            ),
        )),
    }
}

/// Convert the verified inner result into the caller's expected type.
fn convert_proof_result<T: TryFrom<StateTransitionProofResult>>(
    result: StateTransitionProofResult,
) -> Result<T, Error> {
    let variant_name = result.to_string();
    T::try_from(result).map_err(|_| {
        Error::InvalidProvedResponse(format!(
            "invalid proved response: cannot convert from {} to {}",
            variant_name,
            std::any::type_name::<T>(),
        ))
    })
}

/// Internal wait primitive shared by the strict and affected-state public
/// APIs.
#[async_trait::async_trait]
trait WaitForOutcome {
    async fn wait_for_outcome_with_metadata(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(StateTransitionProofOutcome, ResponseMetadata), Error>;
}

#[async_trait::async_trait]
impl WaitForOutcome for StateTransition {
    /// Wait for the transition's result, verify the proof and quorum
    /// signature, and return the tagged outcome plus the authenticated
    /// response metadata. The tag distinguishes execution evidence from an
    /// affected-state snapshot; the public wait APIs enforce it.
    async fn wait_for_outcome_with_metadata(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(StateTransitionProofOutcome, ResponseMetadata), Error> {
        trace!(
            transaction_id = %self
                .transaction_id()
                .map(hex::encode)
                .unwrap_or("UNKNOWN".to_string()),
            "wait: start"
        );

        let retry_settings = match settings {
            Some(s) => sdk.dapi_client_settings.override_by(s.request_settings),
            None => sdk.dapi_client_settings,
        };

        // prepare a factory that will generate closure which executes actual code
        let factory = |request_settings: RequestSettings| async move {
            trace!("wait: creating request");
            let request = self
                .wait_for_state_transition_result_request()
                .map_err(|e| ExecutionError {
                    inner: e,
                    address: None,
                    retries: 0,
                })?;

            trace!("wait: executing request");
            let response = request.execute(sdk, request_settings).await.inner_into()?;
            trace!("wait: received response");

            let grpc_response: &WaitForStateTransitionResultResponse = &response.inner;

            // We use match here to have a compilation error if a new version of the response is introduced
            let state_transition_broadcast_error = match &grpc_response.version {
                Some(wait_for_state_transition_result_response::Version::V0(result)) => {
                    match &result.result {
                        Some(wait_for_state_transition_result_response_v0::Result::Error(e)) => {
                            Some(e)
                        }
                        _ => None,
                    }
                }
                None => None,
            };

            if let Some(e) = state_transition_broadcast_error {
                warn!(error=?e, "wait: state transition broadcast error detected");
                let state_transition_broadcast_error: StateTransitionBroadcastError =
                    StateTransitionBroadcastError::try_from(e.clone())
                        .wrap_to_execution_result(&response)?
                        .inner;

                return Err(Error::from(state_transition_broadcast_error))
                    .wrap_to_execution_result(&response);
            }

            let context_provider = sdk.context_provider().ok_or(ExecutionError {
                inner: Error::from(ContextProviderError::Config(
                    "Context provider not initialized".to_string(),
                )),
                address: Some(response.address.clone()),
                retries: response.retries,
            })?;

            // Verify through the `FromProof` impl: it runs the GroveDB structural check AND
            // `verify_tenderdash_proof` (the quorum BLS signature gate) that authenticates
            // `metadata`. The request must be reconstructed to feed that verifier.
            let request: BroadcastStateTransitionRequest = self
                .broadcast_request_for_state_transition()
                .wrap_to_execution_result(&response)?
                .inner;

            trace!("wait: verifying proof and quorum signature");
            let (maybe_outcome, metadata, _proof) = <StateTransitionProofOutcome as FromProof<
                BroadcastStateTransitionRequest,
            >>::maybe_from_proof_with_metadata(
                request,
                grpc_response.clone(),
                sdk.network,
                sdk.version(),
                &context_provider,
            )
            .map_err(Error::from)
            .wrap_to_execution_result(&response)?
            .inner;

            // The current `FromProof` impl always yields `Some`; this guards only a future
            // impl change, so it stays a typed error rather than an unwrap.
            let outcome: StateTransitionProofOutcome = maybe_outcome
                .ok_or_else(|| {
                    Error::InvalidProvedResponse(
                        "state transition result missing from verified proof".to_string(),
                    )
                })
                .wrap_to_execution_result(&response)?
                .inner;

            // `metadata` is quorum-authenticated only after the verification above, so the
            // protocol-version ratchet must run here, never before. A `StaleNode` error is
            // retryable and prompts another server.
            let _: () = sdk
                .verify_response_metadata("wait_for_state_transition_result", &metadata)
                .wrap_to_execution_result(&response)?
                .inner;

            trace!("wait: proof verification successful");
            trace!(
                result_variant = %outcome.result().to_string(),
                execution_proved = outcome.is_execution_proved(),
                "wait: result variant"
            );

            Ok::<_, Error>((outcome, metadata)).wrap_to_execution_result(&response)
        };

        let future = retry(sdk.address_list(), retry_settings, factory);
        // run the future with or without timeout, depending on the settings
        let wait_timeout = settings.and_then(|s| s.wait_timeout);

        trace!(timeout = ?wait_timeout, "wait: starting retry mechanism");

        match wait_timeout {
            Some(timeout) => {
                trace!(?timeout, "wait: waiting with timeout");
                tokio::time::timeout(timeout, future)
                    .await
                    .map_err(|e| {
                        warn!(?timeout, "wait: timeout reached");
                        Error::TimeoutReached(
                            timeout,
                            format!("Timeout waiting for result of {} (tx id: {}) affecting object {}: {:?}",
                            self.name(),
                            self.transaction_id().map(hex::encode).unwrap_or("UNKNOWN".to_string()),
                            self.unique_identifiers().join(","),
                             e),
                        )
                    })?
                    .into_inner()
            }
            None => {
                trace!("wait: waiting without timeout");
                future.await.into_inner()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::tonic::Status;
    use dpp::consensus::signature::IdentityNotFoundError;
    use dpp::prelude::Identifier;
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
    use dpp::state_transition::batch_transition::document_replace_transition::DocumentReplaceTransitionV0;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use rs_dapi_client::transport::TransportError;
    use rs_dapi_client::DapiClientError;
    use rs_dapi_client::ExecutionResponse;
    use std::future::ready;
    use std::time::Duration;

    fn owner_id() -> Identifier {
        Identifier::from([1; 32])
    }

    fn document_create(contract_id: Identifier, document_type: &str) -> StateTransition {
        StateTransition::Batch(
            BatchTransitionV0 {
                owner_id: owner_id(),
                transitions: vec![DocumentTransition::Create(
                    DocumentCreateTransitionV0 {
                        base: DocumentBaseTransitionV0 {
                            id: Identifier::from([2; 32]),
                            identity_contract_nonce: 17,
                            document_type_name: document_type.to_string(),
                            data_contract_id: contract_id,
                        }
                        .into(),
                        entropy: [3; 32],
                        ..Default::default()
                    }
                    .into(),
                )],
                signature: vec![4; 65].into(),
                ..Default::default()
            }
            .into(),
        )
    }

    fn missing_owner(id: Identifier) -> Error {
        Error::Protocol(ProtocolError::ConsensusError(Box::new(
            IdentityNotFoundError::new(id).into(),
        )))
    }

    fn addresses() -> AddressList {
        "http://127.0.0.1:3001,http://127.0.0.1:3002,http://127.0.0.1:3003,http://127.0.0.1:3004"
            .parse()
            .expect("valid addresses")
    }

    #[test_case::test_case("preorder")]
    #[test_case::test_case("domain")]
    #[tokio::test]
    async fn should_retry_identical_dpns_bytes_on_another_node(document_type: &str) {
        let transition = document_create(SystemDataContract::DPNS.id(), document_type);
        let expected_request = transition.broadcast_request_for_state_transition().unwrap();
        let addresses = addresses();
        let mut requests = Vec::new();
        let mut used_addresses = Vec::new();

        broadcast_with_retries(
            &transition,
            &addresses,
            RequestSettings::default(),
            |request, _| {
                let address = addresses.get_live_addresses().into_iter().next().unwrap();
                requests.push(request);
                used_addresses.push(address.clone());
                ready(if requests.len() == 1 {
                    Err(ExecutionError {
                        inner: missing_owner(owner_id()),
                        address: Some(address),
                        retries: 0,
                    })
                } else {
                    Ok(ExecutionResponse {
                        inner: BroadcastStateTransitionResponse {},
                        address,
                        retries: 0,
                    })
                })
            },
        )
        .await
        .expect("second node accepts the same signed transition");

        assert_eq!(requests, vec![expected_request.clone(), expected_request]);
        assert_ne!(used_addresses[0], used_addresses[1]);
        assert!(addresses.is_banned(&used_addresses[0]));
    }

    #[tokio::test]
    async fn should_retry_only_domain_after_successful_preorder() {
        let preorder = document_create(SystemDataContract::DPNS.id(), "preorder");
        let domain = document_create(SystemDataContract::DPNS.id(), "domain");
        let preorder_request = preorder.broadcast_request_for_state_transition().unwrap();
        let domain_request = domain.broadcast_request_for_state_transition().unwrap();
        let addresses = addresses();
        let mut requests = Vec::new();
        let mut execute = |request, _| {
            let address = addresses.get_live_addresses().into_iter().next().unwrap();
            requests.push(request);
            ready(if requests.len() == 2 {
                Err(ExecutionError {
                    inner: missing_owner(owner_id()),
                    address: Some(address),
                    retries: 0,
                })
            } else {
                Ok(ExecutionResponse {
                    inner: BroadcastStateTransitionResponse {},
                    address,
                    retries: 0,
                })
            })
        };

        // The two registration stages use separate signed creates. Failure in
        // the second broadcast must not replay the already accepted first one.
        broadcast_with_retries(
            &preorder,
            &addresses,
            RequestSettings::default(),
            &mut execute,
        )
        .await
        .expect("preorder is accepted on its first attempt");
        broadcast_with_retries(
            &domain,
            &addresses,
            RequestSettings::default(),
            &mut execute,
        )
        .await
        .expect("only the rejected domain is retried");

        assert_eq!(
            requests,
            vec![preorder_request, domain_request.clone(), domain_request]
        );
    }

    #[tokio::test]
    async fn should_not_classify_timeouts_as_missing_owner_or_override_zero_retries() {
        let transition = document_create(SystemDataContract::DPNS.id(), "domain");
        let timeouts = [
            Error::TimeoutReached(Duration::from_secs(10), "broadcast timed out".to_string()),
            Error::from(DapiClientError::Transport(TransportError::Grpc(
                Status::deadline_exceeded("broadcast timed out"),
            ))),
        ];
        for timeout in timeouts {
            assert!(!is_missing_transition_owner(&transition, &timeout));
            let addresses = addresses();
            let mut timeout = Some(timeout);
            let mut attempts = 0;
            let error = broadcast_with_retries(
                &transition,
                &addresses,
                RequestSettings {
                    retries: Some(0),
                    ..Default::default()
                },
                |_, _| {
                    attempts += 1;
                    ready(Err(ExecutionError {
                        inner: timeout.take().expect("zero retries permits only one send"),
                        address: addresses.get_live_addresses().into_iter().next(),
                        retries: 0,
                    }))
                },
            )
            .await
            .expect_err("ambiguous timeout is returned under the existing retry budget");
            assert_eq!(attempts, 1);
            assert!(!is_missing_transition_owner(&transition, &error.inner));
        }
    }

    #[test_case::test_case(None, 3)]
    #[test_case::test_case(Some(0), 1)]
    #[test_case::test_case(Some(1), 2)]
    #[test_case::test_case(Some(9), 3)]
    #[tokio::test]
    async fn should_bound_dpns_broadcast_attempts(retries: Option<usize>, expected: usize) {
        let transition = document_create(SystemDataContract::DPNS.id(), "preorder");
        let addresses = addresses();
        let mut attempts = 0;
        let error = broadcast_with_retries(
            &transition,
            &addresses,
            RequestSettings {
                retries,
                ..Default::default()
            },
            |_, settings| {
                attempts += 1;
                assert_eq!(settings.retries, Some(expected - attempts));
                ready(Err(ExecutionError {
                    inner: missing_owner(owner_id()),
                    address: addresses.get_live_addresses().into_iter().next(),
                    retries: 0,
                }))
            },
        )
        .await
        .expect_err("all nodes reject the owner");
        assert_eq!(attempts, expected);
        assert_eq!(error.retries, expected);
        assert!(is_missing_transition_owner(&transition, &error.inner));
        assert_eq!(addresses.get_live_addresses().len(), 4 - (expected - 1));
    }

    #[tokio::test]
    async fn should_count_lower_level_attempts_in_dpns_budget() {
        let transition = document_create(SystemDataContract::DPNS.id(), "domain");
        let addresses = addresses();
        let mut attempts = 0;
        let error = broadcast_with_retries(
            &transition,
            &addresses,
            RequestSettings::default(),
            |_, settings| {
                attempts += 1;
                assert_eq!(settings.retries, Some(2));
                ready(Err(ExecutionError {
                    inner: missing_owner(owner_id()),
                    address: addresses.get_live_addresses().into_iter().next(),
                    retries: 2,
                }))
            },
        )
        .await
        .expect_err("lower layer already consumed all three sends");
        assert_eq!(attempts, 1);
        assert_eq!(error.retries, 3);
    }

    #[test_case::test_case(false, true)]
    #[test_case::test_case(true, false)]
    #[test_case::test_case(true, true)]
    #[tokio::test]
    async fn should_stop_when_rejected_node_cannot_be_avoided(ban: bool, known_address: bool) {
        let transition = document_create(SystemDataContract::DPNS.id(), "domain");
        let addresses: AddressList = "http://127.0.0.1:3001".parse().unwrap();
        let address = addresses.get_live_addresses()[0].clone();
        let mut attempts = 0;
        let error = broadcast_with_retries(
            &transition,
            &addresses,
            RequestSettings {
                ban_failed_address: Some(ban),
                ..Default::default()
            },
            |_, _| {
                attempts += 1;
                ready(Err(ExecutionError {
                    inner: missing_owner(owner_id()),
                    address: known_address.then(|| address.clone()),
                    retries: 0,
                }))
            },
        )
        .await
        .expect_err("cannot fail over safely");
        assert_eq!(attempts, 1);
        assert!(is_missing_transition_owner(&transition, &error.inner));
        assert_eq!(addresses.get_live_addresses().len(), 1);
    }

    #[tokio::test]
    async fn should_not_retry_unrelated_or_wait_phase_rejections() {
        let dpns = SystemDataContract::DPNS.id();
        let cases = [
            (
                document_create(Identifier::from([9; 32]), "preorder"),
                missing_owner(owner_id()),
            ),
            (document_create(dpns, "other"), missing_owner(owner_id())),
            (
                document_create(dpns, "domain"),
                missing_owner(Identifier::from([9; 32])),
            ),
            (
                document_create(dpns, "domain"),
                Error::AlreadyExists("already in mempool".into()),
            ),
            (
                document_create(dpns, "domain"),
                Error::Generic("Identity not found".into()),
            ),
            (
                document_create(dpns, "domain"),
                Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                    code: 20000,
                    message: "Identity not found".into(),
                    cause: Some(IdentityNotFoundError::new(owner_id()).into()),
                }),
            ),
        ];
        for (transition, error) in cases {
            let addresses = addresses();
            let mut error = Some(error);
            let mut attempts = 0;
            broadcast_with_retries(
                &transition,
                &addresses,
                RequestSettings::default(),
                |_, _| {
                    attempts += 1;
                    ready(Err(ExecutionError {
                        inner: error
                            .take()
                            .expect("unrelated rejection must not be retried"),
                        address: addresses.get_live_addresses().into_iter().next(),
                        retries: 0,
                    }))
                },
            )
            .await
            .expect_err("rejection is returned unchanged");
            assert_eq!(attempts, 1);
            assert_eq!(addresses.get_live_addresses().len(), 4);
        }
    }

    #[test]
    fn should_exclude_multi_document_batches_and_non_create_actions() {
        let StateTransition::Batch(mut batch) =
            document_create(SystemDataContract::DPNS.id(), "domain")
        else {
            unreachable!();
        };
        let BatchTransition::V0(ref mut v0) = batch else {
            unreachable!();
        };
        v0.transitions.push(v0.transitions[0].clone());
        assert!(dpns_registration_document_type(&StateTransition::Batch(batch)).is_none());

        let transition = StateTransition::Batch(
            BatchTransitionV0 {
                owner_id: owner_id(),
                transitions: vec![DocumentTransition::Replace(
                    DocumentReplaceTransitionV0::default().into(),
                )],
                ..Default::default()
            }
            .into(),
        );
        assert!(dpns_registration_document_type(&transition).is_none());
    }

    #[test]
    fn strict_wait_rejects_affected_state_outcomes() {
        let snapshot =
            StateTransitionProofResult::VerifiedTokenBalanceAbsence(Identifier::from([1u8; 32]));
        let err = require_execution_proved(StateTransitionProofOutcome::AffectedState(snapshot))
            .expect_err("affected-state outcomes must be rejected by the strict wait");
        assert!(matches!(err, Error::ExecutionNotProved(_)));

        let proved =
            StateTransitionProofResult::VerifiedTokenBalanceAbsence(Identifier::from([1u8; 32]));
        require_execution_proved(StateTransitionProofOutcome::ExecutionProved(proved))
            .expect("execution-proved outcomes must pass the strict wait");
    }
}
