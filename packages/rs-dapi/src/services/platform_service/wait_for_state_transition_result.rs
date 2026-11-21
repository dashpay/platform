use crate::clients::drive_client::DriveChannel;
use crate::error::DapiError;
use crate::services::platform_service::error_mapping::decode_consensus_error;
use crate::services::platform_service::{PlatformServiceImpl, TenderdashStatus};
use crate::services::streaming_service::FilterType;
use base64::Engine;
use dapi_grpc::drive::v0::drive_internal_client::DriveInternalClient;
use dapi_grpc::drive::v0::{
    GetStateTransitionExecutionProofRequest, GetStateTransitionExecutionProofResponse,
    RegisterStateTransitionExecutionProofRequest, UnregisterStateTransitionExecutionProofRequest,
};
use dapi_grpc::platform::v0::get_identity_balance_request::GetIdentityBalanceRequestV0;
use dapi_grpc::platform::v0::wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0;
use dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
    StateBeforeBlock, SuccessWithOwnerBalance,
};
use dapi_grpc::platform::v0::{
    GetIdentityBalanceRequest, Proof, ResponseMetadata, WaitForStateTransitionResultRequest,
    WaitForStateTransitionResultResponse, get_identity_balance_request,
    get_identity_balance_response, wait_for_state_transition_result_request,
    wait_for_state_transition_result_response,
};
use dapi_grpc::tonic::{Code, Request, Response};
use dpp::prelude::Identifier;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::StateTransition;
use dpp::util::hash::hash_single;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::mem;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::time::{Instant, timeout, timeout_at};
use tracing::{Instrument, debug, trace};

const STATE_TRANSITION_HASH_SIZE: usize = 32;

impl PlatformServiceImpl {
    /// Wait for a state transition result by subscribing to platform events and returning proofs when requested.
    pub async fn wait_for_state_transition_result_impl(
        &self,
        request: Request<WaitForStateTransitionResultRequest>,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, DapiError> {
        let inner = request.into_inner();
        let v0 = match inner.version {
            Some(wait_for_state_transition_result_request::Version::V0(v0)) => v0,
            None => {
                return Err(DapiError::InvalidArgument(
                    "wait_for_state_transition_result request must have v0".to_string(),
                ));
            }
        };

        // Validate state transition hash
        let state_transition_hash = v0.state_transition_hash;
        validate_state_transition_hash(&state_transition_hash)?;
        // The state before the transition's block is asked for explicitly: with it, the
        // response's proof can be of an older block than the node's latest
        let options = WaitOptions {
            prove: v0.prove,
            request_user_balance: v0.request_user_balance,
            prove_state_before_block: v0.prove && v0.prove_state_before_block,
        };

        let _wait_permit = self
            .state_transition_wait_permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                DapiError::ResourceExhausted(
                    "too many pending state transition result waits".to_string(),
                )
            })?;

        // Convert hash to commonly used representations
        let hash_hex = hex::encode_upper(&state_transition_hash);
        let hash_base64 = base64::prelude::BASE64_STANDARD.encode(&state_transition_hash);

        let span = tracing::trace_span!("wait_for_state_transition_result", tx = %hash_hex);

        async move {
            trace!("waitForStateTransitionResult called for hash: {}", hash_hex);

            // Check if WebSocket is connected
            if !self.websocket_client.is_connected() {
                return Err(DapiError::Unavailable(
                    "Tenderdash is not available".to_string(),
                ));
            }

            // RACE-FREE IMPLEMENTATION: Subscribe via subscription manager BEFORE checking existing state
            trace!(
                "Subscribing (manager) to platform tx for hash: {}",
                hash_hex
            );
            let sub_handle = self
                .subscriber_manager
                .add_subscription(FilterType::PlatformTxId(hash_hex.clone()))
                .await;

            // Check if transaction already exists (after subscription is active)
            trace!("Checking existing transaction for hash: {}", hash_hex);
            match self.tenderdash_client.tx(hash_base64).await {
                Ok(tx) => {
                    debug!(tx = hash_hex, "Transaction already exists, returning it");
                    return self
                        .build_response_from_existing_tx(
                            tx,
                            options,
                        )
                        .await;
                }
                Err(error) => {
                    debug!(?error, "Transaction not found, will wait for future events");
                }
            };

            // Wait for transaction event with timeout. The deadline counts from here, so the
            // optional registration below spends this wait's time, not more.
            let timeout_duration =
                Duration::from_millis(self.config.dapi.state_transition_wait_timeout);
            let deadline = Instant::now() + timeout_duration;

            // The transition is not in a block yet, so Drive can still prove the state before
            // its block: that proof can only be made before the block is committed. The
            // registration lasts as long as this wait: dropping it, whichever way the wait
            // ends, a client hanging up included, releases it.
            let _execution_proof_registration = if options.prove_state_before_block {
                self.register_state_transition_execution_proof(&state_transition_hash, deadline)
                    .await
            } else {
                None
            };

            trace!(
                "Waiting for transaction event with timeout: {:?}",
                timeout_duration
            );

            // Filter events to find our specific transaction
            timeout_at(deadline, async {
                loop {
                    let result = sub_handle.recv().await;
                    match result {
                        Some(crate::services::streaming_service::StreamingEvent::PlatformTx { event }) => {
                            debug!(tx = hash_hex, "Received matching transaction event");
                            return self
                                .build_response_from_event(
                                    event,
                                    options,
                                    deadline,
                                )
                                .await;
                        }
                        Some(message) => {
                            // Ignore other message types
                            trace!(
                                ?message,
                                "Received non-matching message, ignoring; this should not happen due to filtering"
                            );
                            continue;
                        }
                        None => {
                            debug!("Platform tx subscription channel closed unexpectedly");
                            return Err(DapiError::Unavailable(
                                "Platform tx subscription channel closed unexpectedly".to_string(),
                            ));
                        }
                    }
                }
            })
            .await
            .map_err(|msg| DapiError::Timeout(msg.to_string()))
            .inspect_err(|e| {
                tracing::debug!(
                    error = %e,
                    tx = %hash_hex,
                    "wait_for_state_transition_result: timed out"
                );
            })?
        }
        .instrument(span)
        .await
    }

    /// Build a response for a transaction already known to Tenderdash, optionally generating proofs.
    async fn build_response_from_existing_tx(
        &self,
        tx_response: crate::clients::tenderdash_client::TxResponse,
        options: WaitOptions,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, DapiError> {
        let mut response_v0 =
            wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                result: None,
                metadata: None,
                state_before_block: None,
            };

        // Check if transaction had an error
        let tx_result = &tx_response.tx_result;

        if tx_result.code != 0 {
            // Transaction had an error
            let consensus_error_serialized = if tx_result.info.is_empty() {
                None
            } else {
                decode_consensus_error(tx_result.info.clone())
            };

            let error = TenderdashStatus::new(
                i64::from(tx_result.code),
                if tx_result.data.is_empty() {
                    None
                } else {
                    Some(tx_result.data.clone())
                },
                consensus_error_serialized,
            );
            return Ok(error.into());
        }

        // No error; generate proof if requested, else report what can be read without one
        if !tx_response.tx.is_empty()
            && let Ok(tx_data) =
                base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, &tx_response.tx)
        {
            self.fill_success_result(
                &mut response_v0,
                tx_data,
                options,
                u64::try_from(tx_response.height).unwrap_or(0),
                // Nothing waited for the transition, so the answer has no deadline of its own
                None,
            )
            .await;
        }

        let body = WaitForStateTransitionResultResponse {
            version: Some(wait_for_state_transition_result_response::Version::V0(
                response_v0,
            )),
        };

        Ok(body.into())
    }

    /// Build a response from a streamed transaction event, handling success and error cases.
    async fn build_response_from_event(
        &self,
        transaction_event: crate::clients::TransactionEvent,
        options: WaitOptions,
        deadline: Instant,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, DapiError> {
        // Check transaction result
        match transaction_event.result {
            crate::clients::TransactionResult::Success => {
                let mut response_v0 =
                    wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                        result: None,
                        metadata: None,
                        state_before_block: None,
                };
                // Success case - generate proof if requested, else report what
                // can be read without one
                if let Some(tx_bytes) = transaction_event.tx {
                    self.fill_success_result(
                        &mut response_v0,
                        tx_bytes,
                        options,
                        transaction_event.height,
                        Some(deadline),
                    )
                    .await;
                }

                let body = WaitForStateTransitionResultResponse {
                    version: Some(wait_for_state_transition_result_response::Version::V0(
                        response_v0,
                    )),
                };

                Ok(body.into())
            }
            crate::clients::TransactionResult::Error { code, info, data } => {
                // Error case - create error response
                tracing::debug!(
                    code,
                    info = ?info,
                    data = ?data,
                    "Transaction event indicates error"
                );
                let consensus_error = if info.is_empty() {
                    None
                } else {
                    decode_consensus_error(info.clone())
                };
                let error = TenderdashStatus::new(code as i64, data, consensus_error);
                let result: Response<WaitForStateTransitionResultResponse> = error.into();

                Ok(result)
            }
        }
    }

    /// Complete a successful wait: with `prove`, the proof of the transition's
    /// execution (which carries the owner's balance for an owned, fee-paying
    /// transition from protocol version 14); else with `request_user_balance`,
    /// the balance of the identity that owns the transition read from Drive
    /// unverified, and only from a state at or past `executed_at_height`, the
    /// block that executed the transition (a read behind it answers with its
    /// metadata and no balance). Either read failing leaves the response as it
    /// was, which still tells the caller the transition succeeded.
    async fn fill_success_result(
        &self,
        response_v0: &mut wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0,
        tx_bytes: Vec<u8>,
        options: WaitOptions,
        executed_at_height: u64,
        deadline: Option<Instant>,
    ) {
        if options.prove {
            // The proofs Drive made around the transition's block, when the client asked for
            // them and the transition was registered with Drive in time. Without them the proof
            // is of the state Drive holds now, as before.
            // The lookup may take at most half of what is left of the wait, so the proof of
            // current state below still has time when Drive does not answer it
            if options.prove_state_before_block
                && let Some(budget) = execution_proof_call_budget(deadline, 2)
                && let Some((before, after)) = self
                    .fetch_state_transition_execution_proof(&tx_bytes, executed_at_height, budget)
                    .await
            {
                response_v0.result = Some(
                    wait_for_state_transition_result_response_v0::Result::Proof(after.0),
                );
                response_v0.metadata = Some(after.1);
                response_v0.state_before_block = Some(StateBeforeBlock {
                    proof: Some(before.0),
                    metadata: Some(before.1),
                });
                return;
            }
            match self.fetch_proof_for_state_transition(tx_bytes).await {
                Ok((proof, metadata)) => {
                    response_v0.result = Some(
                        wait_for_state_transition_result_response_v0::Result::Proof(proof),
                    );
                    response_v0.metadata = Some(metadata);
                }
                Err(e) => {
                    debug!("Failed to fetch proof: {}", e);
                    // Continue without proof
                }
            }
        } else if options.request_user_balance
            && let Some(owner_id) = transition_owner_id(&tx_bytes)
            && let Some((owner_balance, metadata)) =
                self.fetch_identity_balance_unproved(owner_id).await
        {
            if metadata.height < executed_at_height {
                // The read predates the executing block: answer with that
                // state's metadata and no balance, so the caller can tell a
                // lagging node (metadata height below its transaction's
                // block) from a node that was not asked.
                tracing::warn!(
                    balance_height = metadata.height,
                    executed_at_height,
                    "Drive's state predates the block that executed the transition; not reporting the owner's balance"
                );
                response_v0.metadata = Some(metadata);
                return;
            }
            response_v0.result = Some(
                wait_for_state_transition_result_response_v0::Result::SuccessWithOwnerBalance(
                    SuccessWithOwnerBalance { owner_balance },
                ),
            );
            response_v0.metadata = Some(metadata);
        }
    }

    /// An identity's credit balance as Drive holds it now, without a proof,
    /// with the metadata of the state it was read from. `None` when Drive
    /// cannot answer, which is logged and not an error of the wait.
    async fn fetch_identity_balance_unproved(
        &self,
        identity_id: Identifier,
    ) -> Option<(u64, ResponseMetadata)> {
        let request = GetIdentityBalanceRequest {
            version: Some(get_identity_balance_request::Version::V0(
                GetIdentityBalanceRequestV0 {
                    id: identity_id.to_vec(),
                    prove: false,
                },
            )),
        };
        let mut client = self.drive_client.get_client();
        let response = match client.get_identity_balance(request).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                debug!(
                    "Failed to fetch the transition owner's balance from Drive: {}",
                    e
                );
                return None;
            }
        };
        let Some(get_identity_balance_response::Version::V0(v0)) = response.version else {
            debug!("Drive returned an identity balance response without a version");
            return None;
        };
        match (v0.result, v0.metadata) {
            (
                Some(get_identity_balance_response::get_identity_balance_response_v0::Result::Balance(
                    balance,
                )),
                Some(metadata),
            ) => Some((balance, metadata)),
            _ => {
                debug!("Drive returned no balance for the transition owner");
                None
            }
        }
    }

    /// Asks Drive to prove the state before and after the block that executes the state
    /// transition with this hash, until the returned registration is dropped.
    ///
    /// The registration's lease is picked and armed before the call, so a wait cancelled while
    /// the call is on its way still releases it. The call may take a quarter of the time left
    /// before `deadline`, and at most [`EXECUTION_PROOF_CALL_TIMEOUT`]; with too little time
    /// left there is no registration. Best effort: when Drive refuses or does not answer in
    /// time, the wait answers with a proof of the state Drive holds when the transition is found.
    async fn register_state_transition_execution_proof(
        &self,
        state_transition_hash: &[u8],
        deadline: Instant,
    ) -> Option<ExecutionProofRegistration> {
        let budget = execution_proof_call_budget(Some(deadline), 4)?;
        let mut registration = ExecutionProofRegistration {
            client: self.drive_client.get_internal_client(),
            state_transition_hash: state_transition_hash.to_vec(),
            lease: new_registration_lease(),
            armed: true,
        };
        let request = RegisterStateTransitionExecutionProofRequest {
            state_transition_hash: state_transition_hash.to_vec(),
            lease: registration.lease.to_vec(),
        };
        let mut client = registration.client.clone();

        match timeout(
            budget,
            client.register_state_transition_execution_proof(request),
        )
        .await
        {
            Ok(Ok(_)) => {}
            Ok(Err(status)) => {
                debug!(
                    %status,
                    "Drive did not register the state transition for an execution proof"
                );
                // Drive answered that it did not take it: there is nothing to release
                if matches!(
                    status.code(),
                    Code::ResourceExhausted | Code::InvalidArgument
                ) {
                    registration.armed = false;
                }
            }
            Err(_) => {
                debug!("Drive did not answer the execution proof registration in time");
            }
        }

        Some(registration)
    }

    /// The proofs Drive made of the state before and after the block that executed the state
    /// transition, each with its block's metadata, when Drive has them and they are the pair
    /// around the block at `executed_at_height`.
    async fn fetch_state_transition_execution_proof(
        &self,
        tx_bytes: &[u8],
        executed_at_height: u64,
        budget: Duration,
    ) -> Option<((Proof, ResponseMetadata), (Proof, ResponseMetadata))> {
        let request = GetStateTransitionExecutionProofRequest {
            state_transition_hash: hash_single(tx_bytes).to_vec(),
        };

        let mut client = self.drive_client.get_internal_client();
        let response =
            match timeout(budget, client.get_state_transition_execution_proof(request)).await {
                Ok(Ok(response)) => response.into_inner(),
                Ok(Err(status)) => {
                    debug!(%status, "Failed to fetch the execution proof from Drive");
                    return None;
                }
                Err(_) => {
                    debug!("Drive did not answer the execution proof lookup in time");
                    return None;
                }
            };

        execution_proof_around_block(response, executed_at_height)
    }

    /// Fetch Drive proofs for the provided state transition bytes, returning proof and metadata.
    async fn fetch_proof_for_state_transition(
        &self,
        tx_bytes: Vec<u8>,
    ) -> crate::DAPIResult<(Proof, ResponseMetadata)> {
        // Create a GetProofsRequest with the state transition
        let request = dapi_grpc::drive::v0::GetProofsRequest {
            state_transition: tx_bytes.clone(),
        };

        // Get the internal client and make the request
        let mut internal_client = self.drive_client.get_internal_client();

        match internal_client.get_proofs(request).await {
            Ok(response) => {
                let inner = response.into_inner();

                let proof = inner
                    .proof
                    .ok_or(crate::DapiError::no_valid_tx_proof(&tx_bytes))?;
                let metadata = inner
                    .metadata
                    .ok_or(crate::DapiError::no_valid_tx_proof(&tx_bytes))?;

                Ok((proof, metadata))
            }
            Err(e) => {
                debug!("Failed to fetch proof from Drive: {}", e);
                Err(crate::DapiError::Client(format!(
                    "Failed to fetch proof: {}",
                    e
                )))
            }
        }
    }
}

/// What a wait asked for.
#[derive(Clone, Copy, Debug)]
struct WaitOptions {
    /// A proof of the transition's keys.
    prove: bool,
    /// Without a proof, the owner's balance.
    request_user_balance: bool,
    /// With a proof, the state before the transition's block too.
    prove_state_before_block: bool,
}

/// How long each optional call to Drive for an execution proof may take: the registration
/// before the wait, and the lookup of the pair once the transition is found. Past it the wait
/// goes on as it does without them, with the rest of its own deadline.
const EXECUTION_PROOF_CALL_TIMEOUT: Duration = Duration::from_secs(1);

/// The least time worth giving an optional call to Drive; with less left it is not made.
const MIN_EXECUTION_PROOF_CALL_BUDGET: Duration = Duration::from_millis(10);

/// How long an optional call to Drive for an execution proof may take: `1 / share` of the time
/// left before `deadline`, so the rest of the wait keeps the remainder, and at most
/// [`EXECUTION_PROOF_CALL_TIMEOUT`]. `None` when too little time is left to make it.
fn execution_proof_call_budget(deadline: Option<Instant>, share: u32) -> Option<Duration> {
    let budget = match deadline {
        Some(deadline) => (deadline.saturating_duration_since(Instant::now()) / share)
            .min(EXECUTION_PROOF_CALL_TIMEOUT),
        None => EXECUTION_PROOF_CALL_TIMEOUT,
    };
    (budget >= MIN_EXECUTION_PROOF_CALL_BUDGET).then_some(budget)
}

/// A lease no other registration has: a random half drawn once per process, and a count.
fn new_registration_lease() -> [u8; 16] {
    static PROCESS: OnceLock<u64> = OnceLock::new();
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let process = *PROCESS.get_or_init(|| RandomState::new().build_hasher().finish());
    let mut lease = [0; 16];
    lease[..8].copy_from_slice(&process.to_be_bytes());
    lease[8..].copy_from_slice(&NEXT.fetch_add(1, Ordering::Relaxed).to_be_bytes());
    lease
}

/// A state transition this wait registered with Drive for an execution proof, under a lease of
/// its own. Dropping it releases the lease, so a registration never outlives the wait that made
/// it: a client that waits for transitions that never land frees Drive's slots as its waits
/// end. Releasing only that lease never touches another wait's registration of the same
/// transition.
struct ExecutionProofRegistration {
    client: DriveInternalClient<DriveChannel>,
    state_transition_hash: Vec<u8>,
    lease: [u8; 16],
    /// Whether Drive may hold the registration, so it has to be released.
    armed: bool,
}

impl Drop for ExecutionProofRegistration {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        // Drop cannot wait, so the call runs on its own. Outside a runtime there is nothing to
        // run it on; Drive then forgets the registration after its lifetime in blocks.
        let Ok(runtime) = Handle::try_current() else {
            return;
        };
        let mut client = self.client.clone();
        let request = UnregisterStateTransitionExecutionProofRequest {
            state_transition_hash: mem::take(&mut self.state_transition_hash),
            lease: self.lease.to_vec(),
        };
        runtime.spawn(async move {
            match timeout(
                EXECUTION_PROOF_CALL_TIMEOUT,
                client.unregister_state_transition_execution_proof(request),
            )
            .await
            {
                Ok(Ok(_)) => {}
                Ok(Err(status)) => debug!(
                    %status,
                    "Drive did not release the execution proof registration"
                ),
                Err(_) => debug!("Drive did not answer the execution proof release in time"),
            }
        });
    }
}

/// The two sides of an execution proof, each with its block's metadata, when both are whole and
/// they are the pair around the block at `executed_at_height`: the state the block before it
/// left, and the state it left. Anything else is no pair, and the caller falls back to a proof
/// of the state Drive holds now.
fn execution_proof_around_block(
    response: GetStateTransitionExecutionProofResponse,
    executed_at_height: u64,
) -> Option<((Proof, ResponseMetadata), (Proof, ResponseMetadata))> {
    let before = response.before?;
    let after = response.after?;
    let before = (before.proof?, before.metadata?);
    let after = (after.proof?, after.metadata?);

    if after.1.height != executed_at_height
        || before.1.height.checked_add(1) != Some(after.1.height)
    {
        tracing::warn!(
            before_height = before.1.height,
            after_height = after.1.height,
            executed_at_height,
            "Drive's execution proof is not of the block that executed the transition; not using it"
        );
        return None;
    }

    Some((before, after))
}

/// The identity that owns a state transition: the one whose credit balance a
/// wait that asks for the user's balance reports back. `None` for a transition
/// without an owner and for bytes that do not decode.
fn transition_owner_id(state_transition_bytes: &[u8]) -> Option<Identifier> {
    StateTransition::deserialize_from_bytes_untrusted(state_transition_bytes)
        .ok()?
        .owner_id()
}

pub(super) fn validate_state_transition_hash(hash: &[u8]) -> Result<(), DapiError> {
    if hash.len() != STATE_TRANSITION_HASH_SIZE {
        return Err(DapiError::InvalidArgument(format!(
            "state transition hash must be {STATE_TRANSITION_HASH_SIZE} bytes, got {}",
            hash.len()
        )));
    }

    Ok(())
}

/// Convert a `DapiError` into the gRPC error response expected by waitForStateTransitionResult callers.
pub(super) fn build_wait_for_state_transition_error_response(
    error: &DapiError,
) -> Response<WaitForStateTransitionResultResponse> {
    // TenderdashStatus has everything we need
    let tenderdash_status = if let DapiError::TenderdashClientError(e) = error {
        e.clone()
    } else {
        let status = error.to_status();
        let message = if status.message().is_empty() {
            None
        } else {
            Some(status.message().to_string())
        };
        TenderdashStatus::new(status.code() as i64, message, None)
    };

    tracing::debug!(
        error = %error,
        ?tenderdash_status,
        code = tenderdash_status.code,
        "Mapping DapiError to WaitForStateTransitionResultResponse"
    );
    tenderdash_status.into()
}

#[cfg(test)]
mod tests {
    mod drive_calls {
        use super::super::{
            EXECUTION_PROOF_CALL_TIMEOUT, ExecutionProofRegistration, StateBeforeBlock, WaitOptions,
        };
        use crate::clients::drive_client::{DriveChannel, DriveClient};
        use crate::services::platform_service::PlatformServiceImpl;
        use crate::services::platform_service::protobuf_request_errors::channel;
        use dapi_grpc::drive::v0::drive_internal_server::{DriveInternal, DriveInternalServer};
        use dapi_grpc::drive::v0::get_state_transition_execution_proof_response::StateAtBlock;
        use dapi_grpc::drive::v0::{
            GetProofsRequest, GetProofsResponse, GetStateTransitionExecutionProofRequest,
            GetStateTransitionExecutionProofResponse, RegisterStateTransitionExecutionProofRequest,
            RegisterStateTransitionExecutionProofResponse,
            UnregisterStateTransitionExecutionProofRequest,
            UnregisterStateTransitionExecutionProofResponse,
        };
        use dapi_grpc::platform::v0::wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0;
        use dapi_grpc::platform::v0::wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result as WaitResult;
        use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
        use dapi_grpc::tonic::transport::Server;
        use dapi_grpc::tonic::{Request, Response, Status};
        use dpp::util::hash::hash_single;
        use std::future::pending;
        use std::sync::{Arc, Mutex};
        use std::time::Duration;
        use tokio::task::JoinHandle;
        use tokio::time::Instant;
        use tokio_stream::StreamExt;
        use tower::ServiceBuilder;
        use tower_http::trace::TraceLayer;

        /// One call Drive received, with the hash or bytes and the lease it named.
        #[derive(Clone, Debug, PartialEq)]
        enum Call {
            Register(Vec<u8>, Vec<u8>),
            Unregister(Vec<u8>, Vec<u8>),
            GetExecutionProof(Vec<u8>),
            GetProofs(Vec<u8>),
        }

        /// How the Drive stand-in answers a registration.
        #[derive(Clone, Copy)]
        enum Registration {
            Take,
            Refuse,
            /// Take it, and answer only after this long
            TakeAfter(Duration),
            /// Take it, and never answer
            Stall,
        }

        /// A Drive stand-in that records the internal calls it receives and answers them as
        /// configured.
        #[derive(Clone)]
        struct RecordingDrive {
            calls: Arc<Mutex<Vec<Call>>>,
            registration: Registration,
            execution_proof: GetStateTransitionExecutionProofResponse,
            stall_execution_proof: bool,
        }

        impl RecordingDrive {
            fn record(&self, call: Call) {
                self.calls.lock().unwrap().push(call);
            }
        }

        fn state_at(height: u64) -> StateAtBlock {
            StateAtBlock {
                proof: Some(proof_of(height)),
                metadata: Some(metadata_at(height)),
            }
        }

        fn proof_of(height: u64) -> Proof {
            Proof {
                grovedb_proof: vec![height as u8],
                ..Default::default()
            }
        }

        fn metadata_at(height: u64) -> ResponseMetadata {
            ResponseMetadata {
                height,
                ..Default::default()
            }
        }

        /// What Drive answers for the current state: a proof at height 99.
        const CURRENT_HEIGHT: u64 = 99;

        #[dapi_grpc::tonic::async_trait]
        impl DriveInternal for RecordingDrive {
            async fn get_proofs(
                &self,
                request: Request<GetProofsRequest>,
            ) -> Result<Response<GetProofsResponse>, Status> {
                self.record(Call::GetProofs(request.into_inner().state_transition));
                Ok(Response::new(GetProofsResponse {
                    proof: Some(proof_of(CURRENT_HEIGHT)),
                    metadata: Some(metadata_at(CURRENT_HEIGHT)),
                }))
            }

            async fn register_state_transition_execution_proof(
                &self,
                request: Request<RegisterStateTransitionExecutionProofRequest>,
            ) -> Result<Response<RegisterStateTransitionExecutionProofResponse>, Status>
            {
                let request = request.into_inner();
                // Drive takes the registration before it answers
                self.record(Call::Register(request.state_transition_hash, request.lease));
                match self.registration {
                    Registration::Take => {}
                    Registration::Refuse => return Err(Status::resource_exhausted("full")),
                    Registration::TakeAfter(delay) => tokio::time::sleep(delay).await,
                    Registration::Stall => pending::<()>().await,
                }
                Ok(Response::new(
                    RegisterStateTransitionExecutionProofResponse {},
                ))
            }

            async fn unregister_state_transition_execution_proof(
                &self,
                request: Request<UnregisterStateTransitionExecutionProofRequest>,
            ) -> Result<Response<UnregisterStateTransitionExecutionProofResponse>, Status>
            {
                let request = request.into_inner();
                self.record(Call::Unregister(
                    request.state_transition_hash,
                    request.lease,
                ));
                Ok(Response::new(
                    UnregisterStateTransitionExecutionProofResponse {},
                ))
            }

            async fn get_state_transition_execution_proof(
                &self,
                request: Request<GetStateTransitionExecutionProofRequest>,
            ) -> Result<Response<GetStateTransitionExecutionProofResponse>, Status> {
                self.record(Call::GetExecutionProof(
                    request.into_inner().state_transition_hash,
                ));
                if self.stall_execution_proof {
                    pending::<()>().await;
                }
                Ok(Response::new(self.execution_proof.clone()))
            }
        }

        struct Fixture {
            service: PlatformServiceImpl,
            calls: Arc<Mutex<Vec<Call>>>,
            server: JoinHandle<()>,
        }

        impl Drop for Fixture {
            fn drop(&mut self) {
                self.server.abort();
            }
        }

        impl Fixture {
            fn new(registration: Registration) -> Self {
                Self::serving(RecordingDrive {
                    calls: Default::default(),
                    registration,
                    execution_proof: Default::default(),
                    stall_execution_proof: false,
                })
            }

            fn with_execution_proof(
                execution_proof: GetStateTransitionExecutionProofResponse,
            ) -> Self {
                Self::serving(RecordingDrive {
                    calls: Default::default(),
                    registration: Registration::Take,
                    execution_proof,
                    stall_execution_proof: false,
                })
            }

            fn serving(drive: RecordingDrive) -> Self {
                let calls = Arc::clone(&drive.calls);
                let (client_io, server_io) = tokio::io::duplex(64 * 1024);
                let server = tokio::spawn(async move {
                    let incoming = tokio_stream::iter([Ok::<_, std::io::Error>(server_io)])
                        .chain(tokio_stream::pending());
                    Server::builder()
                        .add_service(DriveInternalServer::new(drive))
                        .serve_with_incoming(incoming)
                        .await
                        .expect("serve the Drive stand-in");
                });
                let traced: DriveChannel = ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .service(channel(client_io, "http://drive.in-memory"));
                Self {
                    service: PlatformServiceImpl::with_test_drive(DriveClient::from_test_channel(
                        traced,
                    )),
                    calls,
                    server,
                }
            }

            fn calls(&self) -> Vec<Call> {
                self.calls.lock().unwrap().clone()
            }

            /// Wait until Drive has received `count` calls.
            async fn wait_for_calls(&self, count: usize) {
                tokio::time::timeout(Duration::from_secs(10), async {
                    while self.calls().len() < count {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                })
                .await
                .expect("the calls reach Drive");
            }

            async fn fill_success_result(
                &self,
                tx: &[u8],
                prove_state_before_block: bool,
                executed_at_height: u64,
            ) -> WaitForStateTransitionResultResponseV0 {
                self.fill_success_result_before(
                    tx,
                    prove_state_before_block,
                    executed_at_height,
                    Some(far_deadline()),
                )
                .await
            }

            async fn fill_success_result_before(
                &self,
                tx: &[u8],
                prove_state_before_block: bool,
                executed_at_height: u64,
                deadline: Option<Instant>,
            ) -> WaitForStateTransitionResultResponseV0 {
                let mut response_v0 = WaitForStateTransitionResultResponseV0 {
                    result: None,
                    metadata: None,
                    state_before_block: None,
                };
                self.service
                    .fill_success_result(
                        &mut response_v0,
                        tx.to_vec(),
                        WaitOptions {
                            prove: true,
                            request_user_balance: false,
                            prove_state_before_block,
                        },
                        executed_at_height,
                        deadline,
                    )
                    .await;
                response_v0
            }
        }

        fn pair(before: u64, after: u64) -> GetStateTransitionExecutionProofResponse {
            GetStateTransitionExecutionProofResponse {
                before: Some(state_at(before)),
                after: Some(state_at(after)),
            }
        }

        /// A deadline far enough that every optional call gets its whole budget.
        fn far_deadline() -> Instant {
            Instant::now() + Duration::from_secs(30)
        }

        /// The lease a recorded registration was taken with.
        fn registered_lease(call: &Call) -> Vec<u8> {
            match call {
                Call::Register(_, lease) => lease.clone(),
                other => panic!("expected a registration, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn should_release_the_lease_a_registration_was_taken_with_when_it_is_dropped() {
            let fixture = Fixture::new(Registration::Take);

            let registration: ExecutionProofRegistration = fixture
                .service
                .register_state_transition_execution_proof(&[7; 32], far_deadline())
                .await
                .expect("expected time to register");
            let lease = registered_lease(&fixture.calls()[0]);
            assert_eq!(lease.len(), 16);

            drop(registration);
            fixture.wait_for_calls(2).await;

            assert_eq!(
                fixture.calls(),
                vec![
                    Call::Register(vec![7; 32], lease.clone()),
                    Call::Unregister(vec![7; 32], lease),
                ]
            );
        }

        #[tokio::test]
        async fn should_take_a_lease_of_its_own_for_each_registration() {
            let fixture = Fixture::new(Registration::Take);

            let first = fixture
                .service
                .register_state_transition_execution_proof(&[7; 32], far_deadline())
                .await
                .expect("expected time to register");
            let second = fixture
                .service
                .register_state_transition_execution_proof(&[7; 32], far_deadline())
                .await
                .expect("expected time to register");

            let calls = fixture.calls();
            assert_ne!(registered_lease(&calls[0]), registered_lease(&calls[1]));
            drop((first, second));
        }

        #[tokio::test]
        async fn should_release_a_registration_cancelled_before_drive_answered_it() {
            let fixture = Fixture::new(Registration::TakeAfter(Duration::from_millis(500)));
            let service = &fixture.service;

            // Drive has taken the registration and not answered yet when the wait ends
            let wait = async {
                let _registration = service
                    .register_state_transition_execution_proof(&[8; 32], far_deadline())
                    .await
                    .expect("expected time to register");
                pending::<()>().await;
            };
            let _ = tokio::time::timeout(Duration::from_millis(100), wait).await;
            fixture.wait_for_calls(2).await;

            let calls = fixture.calls();
            let lease = registered_lease(&calls[0]);
            assert_eq!(calls[1], Call::Unregister(vec![8; 32], lease));
        }

        #[tokio::test]
        async fn should_have_no_registration_to_release_when_drive_refuses_it() {
            let fixture = Fixture::new(Registration::Refuse);

            let registration = fixture
                .service
                .register_state_transition_execution_proof(&[9; 32], far_deadline())
                .await
                .expect("expected time to register");
            drop(registration);
            tokio::time::sleep(Duration::from_millis(50)).await;

            assert_eq!(fixture.calls().len(), 1);
        }

        #[tokio::test]
        async fn should_stop_waiting_for_a_registration_drive_does_not_answer() {
            let fixture = Fixture::new(Registration::Stall);

            let started = Instant::now();
            let registration = fixture
                .service
                .register_state_transition_execution_proof(&[10; 32], far_deadline())
                .await
                .expect("expected time to register");

            assert!(started.elapsed() < EXECUTION_PROOF_CALL_TIMEOUT + Duration::from_millis(500));
            // Drive may hold it, so it is released all the same
            drop(registration);
            fixture.wait_for_calls(2).await;
            assert!(matches!(fixture.calls()[1], Call::Unregister(..)));
        }

        #[tokio::test]
        async fn should_answer_with_drives_pair_around_the_block_of_the_transition() {
            let fixture = Fixture::with_execution_proof(pair(9, 10));
            let tx = b"a transition";

            let response = fixture.fill_success_result(tx, true, 10).await;

            assert_eq!(response.result, Some(WaitResult::Proof(proof_of(10))));
            assert_eq!(response.metadata, Some(metadata_at(10)));
            assert_eq!(
                response.state_before_block,
                Some(StateBeforeBlock {
                    proof: Some(proof_of(9)),
                    metadata: Some(metadata_at(9)),
                })
            );
            // Drive is asked by the transition's hash, and not for a proof of current state
            assert_eq!(
                fixture.calls(),
                vec![Call::GetExecutionProof(hash_single(tx).to_vec())]
            );
        }

        #[tokio::test]
        async fn should_answer_with_a_proof_of_current_state_unless_asked_for_the_state_before() {
            // Drive has a pair, two blocks behind the tip, and the client did not ask for it
            let fixture = Fixture::with_execution_proof(pair(96, 97));
            let tx = b"a transition";

            let response = fixture.fill_success_result(tx, false, 97).await;

            assert_eq!(
                response.result,
                Some(WaitResult::Proof(proof_of(CURRENT_HEIGHT)))
            );
            assert_eq!(response.metadata, Some(metadata_at(CURRENT_HEIGHT)));
            assert_eq!(response.state_before_block, None);
            assert_eq!(fixture.calls(), vec![Call::GetProofs(tx.to_vec())]);
        }

        #[tokio::test]
        async fn should_answer_with_a_proof_of_current_state_when_drive_has_no_pair() {
            for execution_proof in [Default::default(), pair(8, 9)] {
                let fixture = Fixture::with_execution_proof(execution_proof);
                let tx = b"a transition";

                let response = fixture.fill_success_result(tx, true, 10).await;

                assert_eq!(
                    response.result,
                    Some(WaitResult::Proof(proof_of(CURRENT_HEIGHT)))
                );
                assert_eq!(response.metadata, Some(metadata_at(CURRENT_HEIGHT)));
                assert_eq!(response.state_before_block, None);
                assert_eq!(
                    fixture.calls(),
                    vec![
                        Call::GetExecutionProof(hash_single(tx).to_vec()),
                        Call::GetProofs(tx.to_vec()),
                    ]
                );
            }
        }

        #[tokio::test]
        async fn should_answer_with_a_proof_of_current_state_when_drive_does_not_answer_the_pair() {
            let fixture = Fixture::serving(RecordingDrive {
                calls: Default::default(),
                registration: Registration::Take,
                execution_proof: pair(9, 10),
                stall_execution_proof: true,
            });
            let tx = b"a transition";

            let started = Instant::now();
            let response = fixture.fill_success_result(tx, true, 10).await;

            assert!(started.elapsed() < EXECUTION_PROOF_CALL_TIMEOUT + Duration::from_millis(500));
            assert_eq!(
                response.result,
                Some(WaitResult::Proof(proof_of(CURRENT_HEIGHT)))
            );
            assert_eq!(response.state_before_block, None);
        }

        #[tokio::test]
        async fn should_keep_time_for_the_proof_of_current_state_when_the_pair_lookup_stalls() {
            let fixture = Fixture::serving(RecordingDrive {
                calls: Default::default(),
                registration: Registration::Take,
                execution_proof: pair(9, 10),
                stall_execution_proof: true,
            });
            let tx = b"a transition";

            // Less time is left of the wait than the lookup's own limit: the event arrived late
            let deadline = Instant::now() + Duration::from_millis(800);
            let response = tokio::time::timeout_at(
                deadline,
                fixture.fill_success_result_before(tx, true, 10, Some(deadline)),
            )
            .await
            .expect("expected an answer before the wait's deadline");

            assert_eq!(
                response.result,
                Some(WaitResult::Proof(proof_of(CURRENT_HEIGHT)))
            );
            assert_eq!(response.state_before_block, None);
        }

        #[tokio::test]
        async fn should_give_a_registration_only_part_of_what_is_left_of_the_wait() {
            let fixture = Fixture::new(Registration::Stall);

            let deadline = Instant::now() + Duration::from_millis(400);
            let started = Instant::now();
            let registration = fixture
                .service
                .register_state_transition_execution_proof(&[11; 32], deadline)
                .await;

            // A quarter of the 400 ms left, not the whole second
            assert!(started.elapsed() < Duration::from_millis(250));
            assert!(registration.is_some());

            // With no time left there is nothing to register
            assert!(
                fixture
                    .service
                    .register_state_transition_execution_proof(&[12; 32], Instant::now())
                    .await
                    .is_none()
            );
        }
    }

    mod execution_proof {
        use super::super::execution_proof_around_block;
        use dapi_grpc::drive::v0::GetStateTransitionExecutionProofResponse;
        use dapi_grpc::drive::v0::get_state_transition_execution_proof_response::StateAtBlock;
        use dapi_grpc::platform::v0::{Proof, ResponseMetadata};

        fn state_at(height: u64) -> StateAtBlock {
            StateAtBlock {
                proof: Some(Proof {
                    grovedb_proof: vec![height as u8],
                    ..Default::default()
                }),
                metadata: Some(ResponseMetadata {
                    height,
                    ..Default::default()
                }),
            }
        }

        fn pair(before: u64, after: u64) -> GetStateTransitionExecutionProofResponse {
            GetStateTransitionExecutionProofResponse {
                before: Some(state_at(before)),
                after: Some(state_at(after)),
            }
        }

        #[test]
        fn should_use_the_pair_around_the_block_that_executed_the_transition() {
            let (before, after) =
                execution_proof_around_block(pair(9, 10), 10).expect("expected the pair");

            assert_eq!(before.0.grovedb_proof, vec![9]);
            assert_eq!(before.1.height, 9);
            assert_eq!(after.0.grovedb_proof, vec![10]);
            assert_eq!(after.1.height, 10);
        }

        #[test]
        fn should_use_no_pair_when_drive_has_none() {
            assert!(
                execution_proof_around_block(
                    GetStateTransitionExecutionProofResponse::default(),
                    10
                )
                .is_none()
            );
        }

        #[test]
        fn should_use_no_pair_that_misses_a_side_or_a_part_of_one() {
            let mut no_before = pair(9, 10);
            no_before.before = None;
            let mut no_after = pair(9, 10);
            no_after.after = None;
            let mut no_proof = pair(9, 10);
            no_proof.before.as_mut().unwrap().proof = None;
            let mut no_metadata = pair(9, 10);
            no_metadata.after.as_mut().unwrap().metadata = None;

            for response in [no_before, no_after, no_proof, no_metadata] {
                assert!(execution_proof_around_block(response, 10).is_none());
            }
        }

        #[test]
        fn should_use_no_pair_of_another_block() {
            // The pair is of block 10, the transition was executed in block 11
            assert!(execution_proof_around_block(pair(9, 10), 11).is_none());
            // The two sides are not of consecutive blocks
            assert!(execution_proof_around_block(pair(8, 10), 10).is_none());
        }
    }

    use super::*;
    use dapi_grpc::platform::v0::StateTransitionBroadcastError;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransition;
    use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;

    fn document_batch_bytes(owner_id: Identifier) -> Vec<u8> {
        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::new([3u8; 32]),
            identity_contract_nonce: 1,
            document_type_name: "note".to_string(),
            data_contract_id: Identifier::new([4u8; 32]),
        });
        StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
            owner_id,
            transitions: vec![DocumentTransition::Delete(DocumentDeleteTransition::V0(
                DocumentDeleteTransitionV0 { base },
            ))],
            ..Default::default()
        }))
        .serialize_to_bytes()
        .expect("a document batch serializes")
    }

    #[test]
    fn transition_owner_is_the_identity_whose_balance_is_reported() {
        let owner_id = Identifier::new([7u8; 32]);
        assert_eq!(
            transition_owner_id(&document_batch_bytes(owner_id)),
            Some(owner_id)
        );
        let transfer = StateTransition::IdentityCreditTransfer(
            IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0::default()),
        )
        .serialize_to_bytes()
        .expect("a transfer serializes");
        assert_eq!(transition_owner_id(&transfer), Some(Identifier::default()));
    }

    #[test]
    fn undecodable_bytes_report_no_owner() {
        assert_eq!(transition_owner_id(&[0xff; 8]), None);
        assert_eq!(transition_owner_id(&[]), None);
    }

    #[test]
    fn state_transition_hash_must_be_exact_width() {
        assert!(validate_state_transition_hash(&[0; STATE_TRANSITION_HASH_SIZE]).is_ok());

        for length in [0, 1, 31, 33, 20 * 1024] {
            assert!(matches!(
                validate_state_transition_hash(&vec![0; length]),
                Err(DapiError::InvalidArgument(message)) if message.contains("must be 32 bytes")
            ));
        }
    }

    /// Extract the `StateTransitionBroadcastError` from a
    /// `WaitForStateTransitionResultResponse`, unwrapping the V0 / Error layers.
    fn extract_st_error(
        response: Response<WaitForStateTransitionResultResponse>,
    ) -> StateTransitionBroadcastError {
        let inner = response.into_inner();
        match inner.version {
            Some(
                dapi_grpc::platform::v0::wait_for_state_transition_result_response::Version::V0(
                    v0,
                ),
            ) => match v0.result {
                Some(
                    dapi_grpc::platform::v0::wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Error(
                        st_err,
                    ),
                ) => st_err,
                other => panic!("expected Error result, got {:?}", other),
            },
            other => panic!("expected V0, got {:?}", other),
        }
    }

    #[test]
    fn build_error_response_from_tenderdash_client_error() {
        let tenderdash_status = TenderdashStatus::new(42, Some("td error".to_string()), None);
        let error = DapiError::TenderdashClientError(tenderdash_status);
        let response = build_wait_for_state_transition_error_response(&error);
        let st_err = extract_st_error(response);
        assert_eq!(st_err.code, 42);
        assert_eq!(st_err.message, "td error");
    }

    #[test]
    fn build_error_response_from_generic_dapi_error() {
        let error = DapiError::Timeout("timed out".to_string());
        let response = build_wait_for_state_transition_error_response(&error);
        let st_err = extract_st_error(response);
        // DapiError::Timeout -> to_status() -> Code::Internal (13)
        assert_eq!(st_err.code, dapi_grpc::tonic::Code::Internal as u32);
        assert!(
            st_err.message.contains("timed out"),
            "expected message to contain 'timed out', got: {}",
            st_err.message
        );
    }

    #[test]
    fn build_error_response_from_error_with_empty_status_message() {
        // Internal error with a message should produce non-empty message
        let error = DapiError::Internal("internal failure".to_string());
        let response = build_wait_for_state_transition_error_response(&error);
        let st_err = extract_st_error(response);
        assert!(st_err.message.contains("internal failure"));
    }
}
