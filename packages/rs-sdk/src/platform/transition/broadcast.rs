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
use dapi_grpc::tonic::{Code, ConnectError};
use dash_context_provider::ContextProviderError;
use dpp::consensus::signature::SignatureError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::proof_result::{
    StateTransitionProofGuarantee, StateTransitionProofOutcome, StateTransitionProofResult,
};
use dpp::state_transition::StateTransition;
use dpp::system_data_contracts::SystemDataContract;
use dpp::util::hash::hash_single;
use dpp::ProtocolError;
use rs_dapi_client::transport::TransportError;
use rs_dapi_client::AddressList;
use rs_dapi_client::CanRetry;
use rs_dapi_client::DapiClientError;
use rs_dapi_client::ExecutionResult;
use rs_dapi_client::WrapToExecutionResult;
use rs_dapi_client::{DapiRequest, ExecutionError, InnerInto, IntoInner, RequestSettings};
use std::error::Error as StdError;
use std::future::Future;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;
use tracing::{debug, trace, warn};

const DPNS_REGISTRATION_BROADCAST_RETRIES: usize = 2;
// rs-dapi emits this only after finding the transition in neither the local
// mempool nor chain and successfully re-running CheckTx. It refuses this attempt;
// it cannot rule out acceptance by another node during an earlier attempt.
const DAPI_PROCESSING_REJECTION: &str = "State Transition processing error. Please report faulty state transition and try to create a new state transition with different hash as a workaround.";

#[cfg(test)]
pub(super) mod retry_tests;

#[async_trait::async_trait]
pub trait BroadcastStateTransition {
    async fn broadcast(&self, sdk: &Sdk, settings: Option<PutSettings>) -> Result<(), Error>;
    /// Waits for the transition's result and verifies its proof STRICTLY:
    /// succeeds only when the proof establishes that this specific
    /// transition executed. For the transition families whose proofs can
    /// only authenticate the affected state (balance top-ups, credit
    /// transfers and withdrawals, address funds movements, shields,
    /// no-history token operations, key limits updates, contract updates,
    /// contract moderation and fee claims, and creates and deletes of
    /// indexOnly documents), this returns
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
        broadcast_with_observation(self, sdk, settings).await.result
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
        let result = wait_after_broadcast(
            broadcast_with_observation(self, sdk, settings).await,
            self.wait_for_affected_state_with_metadata::<T>(sdk, settings),
        )
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
        let result = wait_after_broadcast(
            broadcast_with_observation(self, sdk, settings).await,
            self.wait_for_response_with_metadata::<T>(sdk, settings),
        )
        .await;
        match &result {
            Ok(_) => trace!("broadcast_and_wait: complete success"),
            Err(e) => warn!(error = ?e, "broadcast_and_wait: failed"),
        }
        result
    }
}

/// The ordinary broadcast result plus evidence from every transport attempt.
/// Kept private so broadcast-only APIs retain their existing final error.
pub(super) struct BroadcastOutcome {
    result: Result<(), Error>,
    possibly_accepted: bool,
}

impl BroadcastOutcome {
    /// The final relay error, retained for legacy combined APIs with a specific fallback.
    pub(super) fn error(&self) -> Option<&Error> {
        self.result.as_ref().err()
    }
}

impl From<Result<(), Error>> for BroadcastOutcome {
    fn from(result: Result<(), Error>) -> Self {
        let possibly_accepted = result
            .as_ref()
            .err()
            .is_some_and(broadcast_may_have_been_accepted);
        Self {
            result,
            possibly_accepted,
        }
    }
}

pub(super) async fn broadcast_with_observation(
    transition: &StateTransition,
    sdk: &Sdk,
    settings: Option<PutSettings>,
) -> BroadcastOutcome {
    trace!(
        state_transition = %transition.name(),
        transaction_id = %transition.transaction_id().map(hex::encode)
            .unwrap_or("UNKNOWN".to_string()),
        "broadcast: start"
    );
    let retry_settings = match settings {
        Some(s) => sdk.dapi_client_settings.override_by(s.request_settings),
        None => sdk.dapi_client_settings,
    };
    let outcome = broadcast_and_observe_with_retries(
        transition,
        sdk.address_list(),
        retry_settings,
        |request, request_settings, possibly_accepted| async move {
            sdk.execute_with_transport_error_observer(request, request_settings, &|error| {
                if transport_broadcast_may_have_been_accepted(error) {
                    possibly_accepted.store(true, Ordering::Relaxed);
                }
            })
            .await
            .map_err(|error| error.inner_into())
        },
    )
    .await;
    match &outcome.result {
        Ok(_) => trace!("broadcast: completed successfully"),
        Err(error) => {
            warn!(?error, "broadcast: failed after retries");
            if let Some(owner_id) = transition.owner_id() {
                sdk.refresh_identity_nonce(&owner_id).await;
            }
        }
    }
    outcome
}

/// Observe one executor invocation, including its existing transport retries.
/// Raw-dispatch convenience APIs retain their retry budget and nonce-cache policy.
pub(super) async fn execute_broadcast_with_observation(
    transition: &StateTransition,
    sdk: &Sdk,
    settings: RequestSettings,
) -> BroadcastOutcome {
    let request = match transition.broadcast_request_for_state_transition() {
        Ok(request) => request,
        Err(error) => return Err(error).into(),
    };
    let possibly_accepted = AtomicBool::new(false);
    let result = sdk
        .execute_with_transport_error_observer(request, settings, &|error| {
            if transport_broadcast_may_have_been_accepted(error) {
                possibly_accepted.store(true, Ordering::Relaxed);
            }
        })
        .await
        .into_inner()
        .map(|_| ())
        .map_err(Error::from);
    let mut outcome = BroadcastOutcome::from(result);
    outcome.possibly_accepted |= possibly_accepted.load(Ordering::Relaxed);
    outcome
}

async fn broadcast_and_observe_with_retries<Fut, Execute>(
    transition: &StateTransition,
    addresses: &AddressList,
    settings: RequestSettings,
    mut execute: Execute,
) -> BroadcastOutcome
where
    Fut: Future<Output = ExecutionResult<BroadcastStateTransitionResponse, Error>> + Send,
    Execute: FnMut(BroadcastStateTransitionRequest, RequestSettings, Arc<AtomicBool>) -> Fut + Send,
{
    // Invocation-local: never shared with another transition or another call.
    let possibly_accepted = Arc::new(AtomicBool::new(false));
    let result = broadcast_with_retries(transition, addresses, settings, |request, settings| {
        let observation = Arc::clone(&possibly_accepted);
        let response = execute(request, settings, Arc::clone(&observation));
        async move {
            let result = response.await;
            // This also covers SDK-level retries and mock executors, which
            // return one result rather than invoking a transport observer.
            if result
                .as_ref()
                .err()
                .is_some_and(|error| broadcast_may_have_been_accepted(&error.inner))
            {
                observation.store(true, Ordering::Relaxed);
            }
            result
        }
    })
    .await
    .into_inner()
    .map(|_| ());
    BroadcastOutcome {
        result,
        possibly_accepted: possibly_accepted.load(Ordering::Relaxed),
    }
}

/// A lost relay acknowledgement or cache hit says nothing about execution.
/// Keep waiting on the SAME signed transition through the caller's existing
/// proof-verification path. Neither an ambiguous broadcast nor a failed proof
/// is promoted to success, and this helper never builds or broadcasts a retry.
pub(super) async fn wait_after_broadcast<T>(
    broadcast: impl Into<BroadcastOutcome>,
    wait: impl Future<Output = Result<T, Error>>,
) -> Result<T, Error> {
    let broadcast = broadcast.into();
    if let Err(error) = broadcast.result {
        if !broadcast.possibly_accepted {
            return Err(error);
        }
        debug!(error = ?error, "broadcast acknowledgement unavailable; awaiting verified result");
    }
    wait.await
}

fn broadcast_may_have_been_accepted(error: &Error) -> bool {
    let status = match error {
        Error::AlreadyExists(_) | Error::TimeoutReached(..) => return true,
        Error::NoAvailableAddressesToRetry(inner) => {
            return broadcast_may_have_been_accepted(inner);
        }
        Error::DapiClientError(DapiClientError::Transport(transport)) => transport,
        Error::DapiClientError(DapiClientError::NoAvailableAddressesToRetry(transport)) => {
            transport
        }
        _ => return false,
    };
    transport_broadcast_may_have_been_accepted(status)
}

fn transport_broadcast_may_have_been_accepted(error: &TransportError) -> bool {
    let TransportError::Grpc(status) = error;
    // tonic preserves its typed connector failure through Status's source chain.
    // DNS, TCP and TLS establishment errors happen before this request is sent.
    // A bare Unavailable (including one received from a node) does not carry
    // that guarantee: it can also mean a lost acknowledgement after dispatch.
    let mut source = status.source();
    while let Some(error) = source {
        if error.is::<ConnectError>() {
            return false;
        }
        source = error.source();
    }
    if status.code() == Code::Internal
        && status
            .message()
            .strip_prefix("Internal error: ")
            .unwrap_or(status.message())
            == DAPI_PROCESSING_REJECTION
    {
        return false;
    }
    // DAPI maps Tenderdash's confirmation timeout to this exact status.
    // Other ResourceExhausted responses are mempool-full or rate-limit
    // refusals and must not be treated as a possibly accepted submission.
    (status.code() == Code::ResourceExhausted
        && status.message() == "broadcasting state transition is timed out")
        || matches!(
            status.code(),
            Code::AlreadyExists
                | Code::DeadlineExceeded
                | Code::Cancelled
                | Code::Unknown
                | Code::Internal
                | Code::Aborted
                | Code::DataLoss
                | Code::Unavailable
        )
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
                    Ok(response) => debug!(
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
        (is_dpns_registration && is_missing_transition_owner(transition, error))
            || is_address_nonce_ahead(transition, error)
    })
    .await
}

fn is_address_nonce_ahead(transition: &StateTransition, error: &Error) -> bool {
    let Error::Protocol(ProtocolError::ConsensusError(consensus)) = error else {
        return false;
    };
    // A lagging node may reject the signed input; replay/concurrent-spend errors stay terminal.
    matches!(consensus.as_ref(),
        ConsensusError::StateError(StateError::AddressInvalidNonceError(nonce_error))
            if nonce_error.provided_nonce() > nonce_error.expected_nonce()
                && transition.inputs().and_then(|inputs| inputs.get(nonce_error.address()))
                    .is_some_and(|(nonce, _)| *nonce == nonce_error.provided_nonce())
    )
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
pub fn require_execution_proved(
    outcome: StateTransitionProofOutcome,
) -> Result<StateTransitionProofResult, Error> {
    let (guarantee, result, _owner_balance) = outcome.into_parts();
    match guarantee {
        StateTransitionProofGuarantee::ExecutionProved => Ok(result),
        StateTransitionProofGuarantee::AffectedState => Err(Error::ExecutionNotProved(
            format!(
                "received a verified {} snapshot for this transition family; wait with the affected-state APIs instead (wait_for_affected_state in Rust, waitForAffectedState or broadcastAndWaitForAffectedState in JavaScript) and treat the result as a height-pinned snapshot",
                result
            ),
        )),
    }
}

/// Convert the verified inner result into the caller's expected type.
pub fn convert_proof_result<T: TryFrom<StateTransitionProofResult>>(
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
pub trait WaitForOutcome {
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
            // `metadata`. The request must be reconstructed to feed that verifier. A quorum
            // key the provider has not cached is fetched and this same response verified
            // again, so a transition that executed is not reported as failed.
            let request: BroadcastStateTransitionRequest = self
                .broadcast_request_for_state_transition()
                .wrap_to_execution_result(&response)?
                .inner;

            trace!("wait: verifying proof and quorum signature");
            let (maybe_outcome, metadata, _proof) = sdk
                .verify_fetching_quorum_key::<BroadcastStateTransitionRequest, StateTransitionProofOutcome>(
                    request,
                    grpc_response.clone(),
                    &context_provider,
                    "wait_for_state_transition_result",
                )
                .await
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
mod address_nonce_tests;

#[cfg(test)]
mod tests {
    use super::retry_tests::{retrying_broadcast, Attempt};
    use super::*;
    use dapi_grpc::tonic::metadata::MetadataValue;
    use dapi_grpc::tonic::Status;
    use dpp::consensus::signature::IdentityNotFoundError;
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializableWithPlatformVersion;
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
    use dpp::state_transition::batch_transition::document_replace_transition::DocumentReplaceTransitionV0;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::version::PlatformVersion;
    use rs_dapi_client::transport::TransportError;
    use rs_dapi_client::DapiClientError;
    use rs_dapi_client::ExecutionResponse;
    use std::future::ready;
    use std::time::Duration;

    #[tokio::test]
    async fn should_wait_for_verified_result_after_ambiguous_broadcast() {
        let errors = [
            Error::AlreadyExists("tx already exists in cache".into()),
            Error::TimeoutReached(Duration::from_secs(10), "lost acknowledgement".into()),
            Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(
                Status::deadline_exceeded("relay timed out"),
            ))),
            Error::NoAvailableAddressesToRetry(Box::new(Error::AlreadyExists("cached".into()))),
        ];
        for error in errors {
            let result = wait_after_broadcast(Err(error), async { Ok(42) }).await;
            assert_eq!(result.expect("verified wait result"), 42);
        }
    }

    #[tokio::test]
    async fn should_wait_after_dapi_confirmation_timeout_including_retry_wrappers() {
        let timeout = || {
            TransportError::Grpc(Status::resource_exhausted(
                "broadcasting state transition is timed out",
            ))
        };
        for error in [
            Error::DapiClientError(DapiClientError::Transport(timeout())),
            Error::DapiClientError(DapiClientError::NoAvailableAddressesToRetry(Box::new(
                timeout(),
            ))),
            Error::NoAvailableAddressesToRetry(Box::new(Error::DapiClientError(
                DapiClientError::Transport(timeout()),
            ))),
            Error::NoAvailableAddressesToRetry(Box::new(Error::DapiClientError(
                DapiClientError::NoAvailableAddressesToRetry(Box::new(timeout())),
            ))),
        ] {
            assert_eq!(
                wait_after_broadcast(Err(error), async { Ok(42) })
                    .await
                    .unwrap(),
                42
            );
        }
    }

    #[tokio::test]
    async fn should_not_wait_after_other_resource_exhausted_refusals() {
        for message in [
            "mempool is full",
            "tenderdash is not responding: too many requests",
            "rate limited",
        ] {
            let error = Error::NoAvailableAddressesToRetry(Box::new(Error::DapiClientError(
                DapiClientError::NoAvailableAddressesToRetry(Box::new(TransportError::Grpc(
                    Status::resource_exhausted(message),
                ))),
            )));
            let expected = error.to_string();
            let result: Result<(), Error> = wait_after_broadcast(Err(error), async {
                panic!("a definite capacity refusal must not poll the result wait")
            })
            .await;
            assert_eq!(result.unwrap_err().to_string(), expected);
        }
    }

    #[tokio::test]
    async fn should_never_substitute_cache_hit_for_a_valid_proof() {
        for wait_error in [
            Error::InvalidProvedResponse("bad signature".into()),
            Error::ExecutionNotProved("affected-state snapshot only".into()),
            Error::TimeoutReached(Duration::from_secs(10), "no verified result".into()),
        ] {
            let expected = wait_error.to_string();
            let result: Result<(), Error> = wait_after_broadcast(
                Err(Error::AlreadyExists("tx already exists in cache".into())),
                async { Err(wait_error) },
            )
            .await;
            assert_eq!(
                result.expect_err("proof is still mandatory").to_string(),
                expected
            );
        }
    }

    #[tokio::test]
    async fn should_not_poll_result_wait_after_definite_broadcast_rejection() {
        for error in [
            missing_owner(owner_id()),
            Error::DapiClientError(DapiClientError::NoAvailableAddresses),
            Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(
                Status::invalid_argument("invalid transition"),
            ))),
        ] {
            let expected = error.to_string();
            let result: Result<(), Error> = wait_after_broadcast(Err(error), async {
                panic!("a definitive rejection must not poll the wait future")
            })
            .await;
            assert_eq!(
                result.expect_err("original rejection").to_string(),
                expected
            );
        }
    }

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

    #[tokio::test(start_paused = true)]
    async fn should_preserve_transport_ambiguity_across_an_outer_dpns_retry() {
        let transition = document_create(SystemDataContract::DPNS.id(), "domain");
        let consensus: ConsensusError = IdentityNotFoundError::new(owner_id()).into();
        let bytes = consensus
            .serialize_to_bytes_with_platform_version(PlatformVersion::latest())
            .unwrap();
        let mut missing_owner = Status::invalid_argument("owner unavailable on this node");
        missing_owner.metadata_mut().insert_bin(
            "dash-serialized-consensus-error-bin",
            MetadataValue::from_bytes(&bytes),
        );
        let (outcome, sent, executions) = retrying_broadcast(
            &transition,
            vec![
                Attempt::Error(Status::unavailable("acknowledgement lost")),
                Attempt::Error(missing_owner),
                Attempt::Error(Status::resource_exhausted("mempool is full")),
            ],
            2,
        )
        .await;
        assert_eq!(
            sent.len(),
            3,
            "the original total dispatch budget is unchanged"
        );
        assert_eq!(executions, 2, "the owner rejection triggers the SDK retry");
        assert!(outcome
            .result
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("mempool is full"));
        assert_eq!(
            wait_after_broadcast(outcome, async { Ok(42) })
                .await
                .unwrap(),
            42
        );
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
        let bans = addresses.ban_info();
        let rejected = bans
            .iter()
            .find(|info| info.uri == used_addresses[0].to_string())
            .unwrap();
        assert_eq!(rejected.ban_count, 1);
        let remaining = rejected.banned_until.unwrap() - chrono::Utc::now();
        assert!(remaining <= chrono::Duration::seconds(2));
    }

    #[tokio::test]
    async fn should_keep_repeated_missing_owner_exclusions_short_and_flat() {
        let transition = document_create(SystemDataContract::DPNS.id(), "preorder");
        let addresses = addresses();
        for round in 0..2 {
            let _ = broadcast_with_retries(
                &transition,
                &addresses,
                RequestSettings::default(),
                |_, _| {
                    let address = addresses.get_live_addresses().into_iter().next().unwrap();
                    ready(Err(ExecutionError {
                        inner: missing_owner(owner_id()),
                        address: Some(address),
                        retries: 0,
                    }))
                },
            )
            .await;
            for ban in addresses.ban_info().iter().filter(|ban| ban.banned) {
                assert_eq!(
                    ban.ban_count, 1,
                    "rejections must not climb the health ladder"
                );
                assert!(
                    ban.banned_until.unwrap() - chrono::Utc::now() <= chrono::Duration::seconds(2)
                );
            }
            if round == 0 {
                // Bans expire by the wall clock, which a CI runner can step
                // back against the monotonic clock behind tokio's sleep, so
                // a fixed sleep races the expiry. Poll instead: the deadline
                // is far above the 2-second exclusion and far below the 60s
                // first rung of the health ladder.
                tokio::time::timeout(Duration::from_secs(10), async {
                    while addresses.get_live_addresses().len() < addresses.len() {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                })
                .await
                .expect("every short exclusion expires");
            }
        }
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
        let err = require_execution_proved(StateTransitionProofOutcome::affected_state(snapshot))
            .expect_err("affected-state outcomes must be rejected by the strict wait");
        assert!(matches!(err, Error::ExecutionNotProved(_)));

        let proved =
            StateTransitionProofResult::VerifiedTokenBalanceAbsence(Identifier::from([1u8; 32]));
        require_execution_proved(StateTransitionProofOutcome::execution_proved(proved))
            .expect("execution-proved outcomes must pass the strict wait");
    }
}
