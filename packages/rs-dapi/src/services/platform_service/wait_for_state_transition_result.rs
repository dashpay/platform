use crate::clients::drive_client::DriveChannel;
use crate::error::DapiError;
use crate::server::grpc::IngressDeadline;
use crate::services::platform_service::error_mapping::decode_consensus_error;
use crate::services::platform_service::get_recent_state_transition_status::is_missing_from_index;
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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::OwnedSemaphorePermit;
use tokio::time::{Instant, timeout, timeout_at};
use tracing::{Instrument, debug, trace};

const STATE_TRANSITION_HASH_SIZE: usize = 32;

/// How long a wait leaves before it repeats a transaction lookup that Tenderdash failed, the
/// first time. The delay doubles after each lookup that fails again, up to
/// [`TX_LOOKUP_MAX_RETRY_DELAY`].
const TX_LOOKUP_RETRY_DELAY: Duration = Duration::from_millis(500);

/// The longest a wait leaves between two transaction lookups that Tenderdash failed.
const TX_LOOKUP_MAX_RETRY_DELAY: Duration = Duration::from_secs(8);

impl PlatformServiceImpl {
    /// Wait for a state transition result by subscribing to platform events and returning proofs when requested.
    pub async fn wait_for_state_transition_result_impl(
        &self,
        request: Request<WaitForStateTransitionResultRequest>,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, DapiError> {
        // Optional calls to Drive end by this instant, so that the answer leaves before the
        // ingress layer cuts the call off. The wait itself keeps its own timeout: a call cut
        // off there ends with the transport's DEADLINE_EXCEEDED, which clients retry.
        let ingress_deadline = request
            .extensions()
            .get::<IngressDeadline>()
            .map(|deadline| deadline.0);
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

        let wait_timeout = Duration::from_millis(self.config.dapi.state_transition_wait_timeout);

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

            // Check if transaction already exists (after subscription is active). A wait that asks
            // for the state before the block registers with Drive at the same time: that state
            // can only be proved before the block is finalized, which can happen during the
            // lookup. The registration lasts as long as this wait: dropping it, whichever way
            // the wait ends, a client hanging up included, releases it, also while its call to
            // Drive is still on its way.
            trace!("Checking existing transaction for hash: {}", hash_hex);
            let registration_deadline =
                optional_work_deadline(Instant::now() + wait_timeout, ingress_deadline);
            let mut register = Box::pin(async {
                if options.prove_state_before_block {
                    self.register_state_transition_execution_proof(
                        &state_transition_hash,
                        registration_deadline,
                    )
                    .await
                } else {
                    None
                }
            });
            let mut lookup = Box::pin(self.tenderdash_client.tx(hash_base64.clone()));
            let mut registered = None;
            let lookup = loop {
                tokio::select! {
                    found = &mut lookup => break found,
                    registration = &mut register, if registered.is_none() => {
                        registered = Some(registration);
                    }
                }
            };
            let lookup_answered = match lookup {
                Ok(tx) => {
                    // Its block is finalized already, so the registration has nothing left to
                    // do: it ends here, whether it is held or its call is still on its way
                    drop(registered);
                    drop(register);
                    debug!(tx = hash_hex, "Transaction already exists, returning it");
                    let work_deadline =
                        optional_work_deadline(Instant::now() + wait_timeout, ingress_deadline);
                    return self
                        .build_response_from_existing_tx(tx, options, work_deadline)
                        .await;
                }
                Err(error) if is_missing_from_index(&error) => {
                    debug!(?error, "Transaction not found, will wait for future events");
                    true
                }
                Err(error) => {
                    debug!(
                        ?error,
                        "Transaction lookup failed, will repeat it while waiting for events"
                    );
                    false
                }
            };

            // The wait's own timeout starts once the lookup is done, as it always has, and the
            // optional pair lookup also ends before the ingress layer cuts the call off. A
            // registration still on its way finishes within that time, not on top of it.
            let deadline = Instant::now() + wait_timeout;
            let work_deadline = optional_work_deadline(deadline, ingress_deadline);
            let timeout_duration = wait_timeout;
            let _execution_proof_registration = match registered {
                Some(registration) => registration,
                None => register.await,
            };

            trace!(
                "Waiting for transaction event with timeout: {:?}",
                timeout_duration
            );

            // Only an answer that the index does not hold the hash says the transaction is in
            // no block yet. A lookup that failed any other way says nothing: the transaction
            // can be in a block already, and then its event came before this wait subscribed
            // and does not come again. Such a lookup is repeated while the wait goes on, until
            // Tenderdash answers one.
            let lookup_again = async {
                let mut delay = TX_LOOKUP_RETRY_DELAY;
                loop {
                    tokio::time::sleep(delay).await;
                    match self.tenderdash_client.tx(hash_base64.clone()).await {
                        Ok(tx) => break Some(tx),
                        Err(error) if is_missing_from_index(&error) => break None,
                        Err(error) => {
                            debug!(?error, "Transaction lookup failed again");
                            delay = (delay * 2).min(TX_LOOKUP_MAX_RETRY_DELAY);
                        }
                    }
                }
            };
            tokio::pin!(lookup_again);
            let mut lookup_open = !lookup_answered;

            // Filter events to find our specific transaction
            timeout_at(deadline, async {
                loop {
                    let result = tokio::select! {
                        result = sub_handle.recv() => result,
                        found = &mut lookup_again, if lookup_open => {
                            lookup_open = false;
                            match found {
                                Some(tx) => {
                                    debug!(
                                        tx = hash_hex,
                                        "Transaction found by a repeated lookup, returning it"
                                    );
                                    return self
                                        .build_response_from_existing_tx(tx, options, work_deadline)
                                        .await;
                                }
                                // In no block yet, and this wait has been subscribed since
                                // before the lookup: its event is still to come
                                None => continue,
                            }
                        }
                    };
                    match result {
                        Some(crate::services::streaming_service::StreamingEvent::PlatformTx { event }) => {
                            debug!(tx = hash_hex, "Received matching transaction event");
                            return self
                                .build_response_from_event(
                                    event,
                                    options,
                                    work_deadline,
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
        deadline: Instant,
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
    /// left there is no registration. Each registration holds one of the process's
    /// registration permits until its release with Drive ends, after the wait itself; with none
    /// free there is no registration, nor when Drive refuses it outright, which frees the
    /// permit at once. Best effort: when Drive refuses or does not answer in time, the wait
    /// answers with a proof of the state Drive holds when the transition is found.
    async fn register_state_transition_execution_proof(
        &self,
        state_transition_hash: &[u8],
        deadline: Instant,
    ) -> Option<ExecutionProofRegistration> {
        let budget = execution_proof_call_budget(Some(deadline), 4)?;
        let Ok(permit) = Arc::clone(&self.execution_proof_registration_permits).try_acquire_owned()
        else {
            debug!("too many execution proof registrations are held; this wait registers none");
            return None;
        };
        let mut registration = ExecutionProofRegistration {
            client: self.drive_client.get_internal_client(),
            state_transition_hash: state_transition_hash.to_vec(),
            lease: new_registration_lease(),
            armed: true,
            permit: Some(permit),
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
                // Drive answered that it did not take it: it is full, the request is malformed,
                // the lease counts as released, or this Drive is older and has no such call.
                // There is nothing to release, and releasing anyway would only make Drive
                // remember the lease, so the registration ends here and its permit is free for
                // another wait at once. Any other status can be of a registration Drive took
                // and whose answer was lost, so that one is kept to be released.
                if matches!(
                    status.code(),
                    Code::ResourceExhausted
                        | Code::InvalidArgument
                        | Code::FailedPrecondition
                        | Code::Unimplemented
                ) {
                    registration.armed = false;
                    return None;
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

/// When a wait's optional calls to Drive must be done: by the wait's own deadline, or by the
/// ingress layer's [`IngressDeadline`] when that comes first.
fn optional_work_deadline(wait_deadline: Instant, ingress_deadline: Option<Instant>) -> Instant {
    ingress_deadline.map_or(wait_deadline, |ingress| ingress.min(wait_deadline))
}

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

/// A lease no other registration has: an issuer half drawn at random once per process, and a
/// sequence number that increases for every lease, as Drive's low-water marks expect.
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
    /// The process's registration permit, held until the release with Drive ends.
    permit: Option<OwnedSemaphorePermit>,
}

impl Drop for ExecutionProofRegistration {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        // Drop cannot wait, so the call runs on its own. Outside a runtime there is nothing to
        // run it on; Drive then forgets the registration once it has outlived its lifetime.
        let Ok(runtime) = Handle::try_current() else {
            return;
        };
        let mut client = self.client.clone();
        let request = UnregisterStateTransitionExecutionProofRequest {
            state_transition_hash: mem::take(&mut self.state_transition_hash),
            lease: self.lease.to_vec(),
        };
        let permit = self.permit.take();
        runtime.spawn(async move {
            // Releases still on their way count against the registrations held
            let _permit = permit;
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
        use dapi_grpc::tonic::{Code, Request, Response, Status};
        use super::event_wait::LookupTenderdash;
        use crate::error::DapiError;
        use serde_json::json;
        use dapi_grpc::platform::v0::WaitForStateTransitionResultRequest;
        use dapi_grpc::platform::v0::wait_for_state_transition_result_request::{
            Version as WaitRequestVersion, WaitForStateTransitionResultRequestV0,
        };
        use dpp::util::hash::hash_single;
        use std::future::pending;
        use std::sync::{Arc, Mutex};
        use tokio::sync::{Notify, Semaphore};
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
            /// Refuse it with this code
            Refuse(Code),
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
            /// When set, an unregister is recorded and answered only once this is notified
            unregister_gate: Option<Arc<Notify>>,
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
                    Registration::Refuse(code) => return Err(Status::new(code, "refused")),
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
                if let Some(gate) = &self.unregister_gate {
                    gate.notified().await;
                }
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
                    unregister_gate: None,
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
                    unregister_gate: None,
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
            let fixture = Fixture::new(Registration::Stall);
            let mut registration = Box::pin(
                fixture
                    .service
                    .register_state_transition_execution_proof(&[8; 32], far_deadline()),
            );

            // Cancel the wait once Drive has taken the registration and not answered it
            tokio::select! {
                _ = &mut registration => panic!("Drive never answers this registration"),
                _ = fixture.wait_for_calls(1) => {}
            }
            drop(registration);
            fixture.wait_for_calls(2).await;

            let calls = fixture.calls();
            let lease = registered_lease(&calls[0]);
            assert_eq!(calls[1], Call::Unregister(vec![8; 32], lease));
        }

        #[tokio::test]
        async fn should_have_no_registration_to_release_when_drive_refuses_it() {
            // Drive is full, counts the lease as released already, finds the request malformed,
            // or is an older Drive without the call
            for code in [
                Code::ResourceExhausted,
                Code::FailedPrecondition,
                Code::InvalidArgument,
                Code::Unimplemented,
            ] {
                let mut fixture = Fixture::new(Registration::Refuse(code));
                let permits = Arc::new(Semaphore::new(1));
                fixture.service.execution_proof_registration_permits = Arc::clone(&permits);

                let registration = fixture
                    .service
                    .register_state_transition_execution_proof(&[9; 32], far_deadline())
                    .await;

                // No registration, its permit free at once, and nothing sent to release
                assert!(registration.is_none(), "{code:?}");
                assert_eq!(permits.available_permits(), 1, "{code:?}");
                tokio::time::sleep(Duration::from_millis(50)).await;
                assert_eq!(fixture.calls().len(), 1, "{code:?}");
            }
        }

        #[tokio::test]
        async fn should_release_a_registration_whose_error_does_not_say_drive_refused_it() {
            // A status that can follow a registration Drive took, when its answer is lost
            let fixture = Fixture::new(Registration::Refuse(Code::Unavailable));

            let registration = fixture
                .service
                .register_state_transition_execution_proof(&[17; 32], far_deadline())
                .await
                .expect("expected a registration to release");
            let lease = registered_lease(&fixture.calls()[0]);

            drop(registration);
            fixture.wait_for_calls(2).await;
            assert_eq!(fixture.calls()[1], Call::Unregister(vec![17; 32], lease));
        }

        // Only the in-memory Drive stand-in is in play, so the clock is tokio's own: the limit
        // runs out when nothing else is left to do, however busy the machine is
        #[tokio::test(start_paused = true)]
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

        #[tokio::test(start_paused = true)]
        async fn should_answer_with_a_proof_of_current_state_when_drive_does_not_answer_the_pair() {
            let fixture = Fixture::serving(RecordingDrive {
                calls: Default::default(),
                registration: Registration::Take,
                execution_proof: pair(9, 10),
                stall_execution_proof: true,
                unregister_gate: None,
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

        #[tokio::test(start_paused = true)]
        async fn should_keep_time_for_the_proof_of_current_state_when_the_pair_lookup_stalls() {
            let fixture = Fixture::serving(RecordingDrive {
                calls: Default::default(),
                registration: Registration::Take,
                execution_proof: pair(9, 10),
                stall_execution_proof: true,
                unregister_gate: None,
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
        async fn should_hold_a_registration_permit_until_its_release_with_drive_ends() {
            let gate = Arc::new(Notify::new());
            let mut fixture = Fixture::serving(RecordingDrive {
                calls: Default::default(),
                registration: Registration::Take,
                execution_proof: Default::default(),
                stall_execution_proof: false,
                unregister_gate: Some(Arc::clone(&gate)),
            });
            let permits = Arc::new(Semaphore::new(1));
            fixture.service.execution_proof_registration_permits = Arc::clone(&permits);

            let first = fixture
                .service
                .register_state_transition_execution_proof(&[13; 32], far_deadline())
                .await
                .expect("expected a registration");

            // While it is held no other wait registers, and Drive is not asked
            assert!(
                fixture
                    .service
                    .register_state_transition_execution_proof(&[14; 32], far_deadline())
                    .await
                    .is_none()
            );
            assert_eq!(fixture.calls().len(), 1);

            // Its release, which runs on after the wait, keeps the permit while Drive has not
            // answered it
            drop(first);
            fixture.wait_for_calls(2).await;
            assert!(
                fixture
                    .service
                    .register_state_transition_execution_proof(&[14; 32], far_deadline())
                    .await
                    .is_none()
            );
            assert_eq!(fixture.calls().len(), 2);

            // And gives it back once Drive answers
            gate.notify_one();
            tokio::time::timeout(Duration::from_secs(10), async {
                while permits.available_permits() == 0 {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .expect("expected the permit back once the release ended");

            assert!(
                fixture
                    .service
                    .register_state_transition_execution_proof(&[14; 32], far_deadline())
                    .await
                    .is_some()
            );
        }

        #[tokio::test]
        async fn should_answer_an_indexed_transaction_without_waiting_for_its_registration() {
            // Tenderdash holds the transaction, executed with an error, so the answer needs no
            // call to Drive. It answers only once Drive has the registration, which Drive never
            // acknowledges.
            let lookup_gate = Arc::new(Notify::new());
            let tenderdash = LookupTenderdash::holding_once(
                Arc::clone(&lookup_gate),
                json!({
                    "hash": "0C",
                    "height": "7",
                    "index": 0,
                    "tx_result": { "code": 1, "data": "", "info": "", "log": "" },
                    "tx": "",
                }),
            )
            .await;
            let mut fixture = Fixture::new(Registration::Stall);
            tenderdash.serve(&mut fixture.service, Duration::from_secs(30));
            let request = Request::new(WaitForStateTransitionResultRequest {
                version: Some(WaitRequestVersion::V0(
                    WaitForStateTransitionResultRequestV0 {
                        state_transition_hash: vec![15; 32],
                        prove: true,
                        request_user_balance: false,
                        prove_state_before_block: true,
                    },
                )),
            });

            let answer_lookup = async {
                fixture.wait_for_calls(1).await;
                lookup_gate.notify_one();
            };
            let started = Instant::now();
            let (answer, ()) = tokio::join!(
                fixture
                    .service
                    .wait_for_state_transition_result_impl(request),
                answer_lookup
            );
            let elapsed = started.elapsed();
            answer.expect("expected an answer");

            // A wait that waited for its registration would answer no sooner than the
            // registration's own limit, so anything under it is an answer that did not wait
            assert!(elapsed < EXECUTION_PROOF_CALL_TIMEOUT, "{elapsed:?}");

            // The registration Drive received is released
            fixture.wait_for_calls(2).await;
            let calls = fixture.calls();
            let lease = registered_lease(&calls[0]);
            assert_eq!(calls[1], Call::Unregister(vec![15; 32], lease));
        }

        #[tokio::test]
        async fn should_count_a_pending_registration_against_the_event_wait() {
            // Tenderdash misses at once, and Drive never acknowledges the registration
            let tenderdash = LookupTenderdash::start(Duration::ZERO).await;
            let mut fixture = Fixture::new(Registration::Stall);
            // Long enough that a quarter of it is the registration's whole limit, the widest
            // gap there can be between a wait that counts the registration and one that does not
            let wait_timeout = 4 * EXECUTION_PROOF_CALL_TIMEOUT;
            tenderdash.serve(&mut fixture.service, wait_timeout);
            let request = Request::new(WaitForStateTransitionResultRequest {
                version: Some(WaitRequestVersion::V0(
                    WaitForStateTransitionResultRequestV0 {
                        state_transition_hash: vec![16; 32],
                        prove: true,
                        request_user_balance: false,
                        prove_state_before_block: true,
                    },
                )),
            });

            let started = Instant::now();
            let answer = fixture
                .service
                .wait_for_state_transition_result_impl(request)
                .await;

            // The wait ends at its own timeout after the miss, not that plus the registration's
            // limit, a quarter of the wait, nor earlier when that limit runs out
            let elapsed = started.elapsed();
            assert!(
                matches!(answer, Err(DapiError::Timeout(_))),
                "expected the event timeout, got {answer:?}"
            );
            assert!(
                elapsed >= wait_timeout
                    && elapsed < wait_timeout + EXECUTION_PROOF_CALL_TIMEOUT * 3 / 4,
                "{elapsed:?}"
            );
            assert!(matches!(
                fixture.calls().first(),
                Some(Call::Register(hash, _)) if hash.as_slice() == [16; 32].as_slice()
            ));
        }

        #[tokio::test]
        async fn should_register_while_the_transaction_lookup_is_still_on_its_way() {
            // Tenderdash answers no lookup for as long as this test runs
            let tenderdash = LookupTenderdash::holding_back(Arc::new(Notify::new())).await;
            let mut fixture = Fixture::new(Registration::Take);
            tenderdash.serve(&mut fixture.service, Duration::from_secs(30));
            let request = Request::new(WaitForStateTransitionResultRequest {
                version: Some(WaitRequestVersion::V0(
                    WaitForStateTransitionResultRequestV0 {
                        state_transition_hash: vec![12; 32],
                        prove: true,
                        request_user_balance: false,
                        prove_state_before_block: true,
                    },
                )),
            });

            // The block can be finalized during the lookup, so Drive is asked before it answers
            tokio::select! {
                _ = fixture.service.wait_for_state_transition_result_impl(request) => {
                    panic!("the lookup is not answered, and no event comes")
                }
                _ = fixture.wait_for_calls(1) => {}
            }

            assert!(matches!(
                &fixture.calls()[0],
                Call::Register(hash, _) if hash == &vec![12; 32]
            ));
        }

        #[tokio::test]
        async fn should_make_no_pair_lookup_when_no_time_is_left_for_it() {
            let fixture = Fixture::serving(RecordingDrive {
                calls: Default::default(),
                registration: Registration::Take,
                execution_proof: pair(9, 10),
                stall_execution_proof: true,
                unregister_gate: None,
            });
            let tx = b"a transition";

            // The client's deadline leaves no time for optional work, as with a grpc-timeout
            // shorter than the ingress layer's margin
            let response = fixture
                .fill_success_result_before(tx, true, 10, Some(Instant::now()))
                .await;

            assert_eq!(
                response.result,
                Some(WaitResult::Proof(proof_of(CURRENT_HEIGHT)))
            );
            assert_eq!(fixture.calls(), vec![Call::GetProofs(tx.to_vec())]);
        }

        #[tokio::test(start_paused = true)]
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

    mod event_wait {
        use super::super::TX_LOOKUP_RETRY_DELAY;
        use crate::clients::{TenderdashClient, TransactionEvent, TransactionResult};
        use crate::error::DapiError;
        use crate::server::grpc::TimeoutLayer;
        use crate::services::platform_service::PlatformServiceImpl;
        use crate::services::platform_service::protobuf_request_errors::channel;
        use crate::services::streaming_service::StreamingEvent;
        use axum::routing::post;
        use axum::{Json, Router};
        use dapi_grpc::platform::v0::WaitForStateTransitionResultRequest;
        use dapi_grpc::platform::v0::platform_client::PlatformClient;
        use dapi_grpc::platform::v0::platform_server::PlatformServer;
        use dapi_grpc::platform::v0::wait_for_state_transition_result_request::{
            Version, WaitForStateTransitionResultRequestV0,
        };
        use dapi_grpc::platform::v0::wait_for_state_transition_result_response::Version as ResponseVersion;
        use dapi_grpc::platform::v0::wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result as WaitResult;
        use dapi_grpc::tonic::transport::Server;
        use dapi_grpc::tonic::{Code, Request};
        use serde_json::{Value, json};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;
        use tokio::net::TcpListener;
        use tokio::sync::Notify;
        use tokio::task::JoinHandle;
        use tokio_stream::StreamExt;

        /// The wait's own timeout.
        const WAIT_TIMEOUT: Duration = Duration::from_millis(1_000);

        /// What the Tenderdash stand-in answers one transaction lookup with.
        #[derive(Clone)]
        pub(super) enum Lookup {
            /// Its index does not hold the hash, in Tenderdash's own words
            Missing,
            /// It could not read its index, so the hash may be there
            Failed,
            /// This `tx` result
            Found(Value),
        }

        /// A Tenderdash stand-in that answers every transaction lookup after a while: that it
        /// holds no such transaction, that the lookup failed, or with a transaction it holds.
        pub(super) struct LookupTenderdash {
            url: String,
            server: JoinHandle<()>,
            lookups: Arc<AtomicUsize>,
        }

        impl Drop for LookupTenderdash {
            fn drop(&mut self) {
                self.server.abort();
            }
        }

        impl LookupTenderdash {
            pub(super) async fn start(lookup_time: Duration) -> Self {
                Self::answering(lookup_time, vec![Lookup::Missing], None).await
            }

            /// Answers no lookup until `gate` is notified, then that it holds no such
            /// transaction.
            pub(super) async fn holding_back(gate: Arc<Notify>) -> Self {
                Self::answering(Duration::ZERO, vec![Lookup::Missing], Some(gate)).await
            }

            /// Answers every lookup with this `tx` result, once `gate` is notified.
            pub(super) async fn holding_once(gate: Arc<Notify>, tx: Value) -> Self {
                Self::answering(Duration::ZERO, vec![Lookup::Found(tx)], Some(gate)).await
            }

            /// Answers the lookups with `answers` in turn, and every later one with the last.
            pub(super) async fn answering_in_turn(answers: Vec<Lookup>) -> Self {
                Self::answering(Duration::ZERO, answers, None).await
            }

            /// How many lookups it has received.
            pub(super) fn lookups(&self) -> usize {
                self.lookups.load(Ordering::SeqCst)
            }

            async fn answering(
                lookup_time: Duration,
                answers: Vec<Lookup>,
                gate: Option<Arc<Notify>>,
            ) -> Self {
                let lookups = Arc::new(AtomicUsize::new(0));
                let received = Arc::clone(&lookups);
                let listener = TcpListener::bind("127.0.0.1:0")
                    .await
                    .expect("slow lookup Tenderdash listener");
                let address = listener
                    .local_addr()
                    .expect("slow lookup Tenderdash address");
                let router = Router::new().route(
                    "/",
                    post(move |Json(call): Json<Value>| async move {
                        let turn = received.fetch_add(1, Ordering::SeqCst);
                        if let Some(gate) = gate {
                            gate.notified().await;
                        }
                        tokio::time::sleep(lookup_time).await;
                        // As Tenderdash words them: every error of its `tx` method starts the
                        // same way, and only a hash its index does not hold has no cause
                        let error = |cause: &str| {
                            json!({
                                "jsonrpc": "2.0",
                                "id": call["id"],
                                "error": {
                                    "code": -32603,
                                    "message": "Internal error",
                                    "data": format!("tx (0C) not found, err: {cause}"),
                                },
                            })
                        };
                        let answer = answers
                            .get(turn)
                            .or(answers.last())
                            .cloned()
                            .unwrap_or(Lookup::Missing);
                        Json(match answer {
                            Lookup::Found(tx) => {
                                json!({ "jsonrpc": "2.0", "id": call["id"], "result": tx })
                            }
                            Lookup::Missing => error("%!w(<nil>)"),
                            Lookup::Failed => error("database unavailable"),
                        })
                    }),
                );
                let server = tokio::spawn(async move {
                    axum::serve(listener, router)
                        .await
                        .expect("slow lookup Tenderdash serves");
                });
                Self {
                    url: format!("http://{address}"),
                    server,
                    lookups,
                }
            }

            /// A service whose Tenderdash lookups come here, whose events the test delivers, and
            /// whose waits last `wait_timeout`.
            fn service(&self, wait_timeout: Duration) -> PlatformServiceImpl {
                let mut service = PlatformServiceImpl::without_backends();
                self.serve(&mut service, wait_timeout);
                service
            }

            /// Points `service`'s Tenderdash lookups here, lets the test deliver its events, and
            /// makes its waits last `wait_timeout`.
            pub(super) fn serve(&self, service: &mut PlatformServiceImpl, wait_timeout: Duration) {
                let tenderdash = TenderdashClient::with_test_base_url(self.url.clone());
                service.websocket_client = tenderdash.websocket_client();
                service.websocket_client.set_connected_for_test();
                service.tenderdash_client = Arc::new(tenderdash);
                let mut config = (*service.config).clone();
                config.dapi.state_transition_wait_timeout = wait_timeout.as_millis() as u64;
                service.config = Arc::new(config);
            }
        }

        /// A wait for the transition with this hash that does not ask for the state before the
        /// block.
        fn unflagged_wait(hash: [u8; 32]) -> WaitForStateTransitionResultRequest {
            WaitForStateTransitionResultRequest {
                version: Some(Version::V0(WaitForStateTransitionResultRequestV0 {
                    state_transition_hash: hash.to_vec(),
                    prove: false,
                    request_user_balance: false,
                    prove_state_before_block: false,
                })),
            }
        }

        #[tokio::test]
        async fn should_end_a_wait_cut_off_by_the_client_deadline_with_deadline_exceeded() {
            let tenderdash = LookupTenderdash::start(Duration::ZERO).await;
            // The wait's own timeout is far longer than the client's deadline
            let service = tenderdash.service(Duration::from_secs(30));
            let (client_io, server_io) = tokio::io::duplex(64 * 1024);
            let server = tokio::spawn(async move {
                let incoming = tokio_stream::iter([Ok::<_, std::io::Error>(server_io)])
                    .chain(tokio_stream::pending());
                Server::builder()
                    .layer(TimeoutLayer::new(
                        Duration::from_secs(15),
                        Duration::from_secs(600),
                    ))
                    .add_service(PlatformServer::new(service))
                    .serve_with_incoming(incoming)
                    .await
                    .expect("serve the platform service");
            });
            let mut client = PlatformClient::new(channel(client_io, "http://dapi.in-memory"));
            let mut request = Request::new(unflagged_wait([4; 32]));
            request.set_timeout(Duration::from_millis(300));

            let answer = client.wait_for_state_transition_result(request).await;
            server.abort();

            // The transport's status, which clients retry, not an answer of the wait's own
            let status = answer.expect_err("expected the call to be cut off");
            assert_eq!(status.code(), Code::DeadlineExceeded, "{status:?}");
        }

        #[tokio::test]
        async fn should_give_the_event_the_whole_wait_timeout_after_a_slow_lookup() {
            let lookup_gate = Arc::new(Notify::new());
            let tenderdash = LookupTenderdash::holding_back(Arc::clone(&lookup_gate)).await;
            let service = tenderdash.service(WAIT_TIMEOUT);
            let hash = [3u8; 32];
            let request = Request::new(unflagged_wait(hash));

            // The lookup is answered once the wait's own timeout has passed counted from the
            // start of the call, and the event comes a little after that: early in the timeout
            // counted from the lookup's answer
            let deliver_event = async {
                tokio::time::sleep(WAIT_TIMEOUT).await;
                lookup_gate.notify_one();
                tokio::time::sleep(WAIT_TIMEOUT / 10).await;
                service
                    .subscriber_manager
                    .notify(StreamingEvent::PlatformTx {
                        event: TransactionEvent {
                            hash: hex::encode_upper(hash),
                            height: 5,
                            result: TransactionResult::Success,
                            tx: None,
                        },
                    })
                    .await;
            };
            let (answer, ()) = tokio::join!(
                service.wait_for_state_transition_result_impl(request),
                deliver_event
            );

            answer.expect("expected the event to end the wait");
        }

        /// A transaction Tenderdash holds, executed with an error, so that answering with it
        /// needs no call to Drive.
        fn failed_transaction() -> Value {
            json!({
                "hash": "0C",
                "height": "7",
                "index": 0,
                "tx_result": { "code": 1, "data": "", "info": "", "log": "" },
                "tx": "",
            })
        }

        #[tokio::test]
        async fn should_answer_with_a_transaction_a_repeated_lookup_finds_after_a_failed_one() {
            // The transaction is in a block already, so no event comes for it, and Tenderdash
            // fails the first lookup of it
            let tenderdash = LookupTenderdash::answering_in_turn(vec![
                Lookup::Failed,
                Lookup::Found(failed_transaction()),
            ])
            .await;
            let service = tenderdash.service(Duration::from_secs(30));

            let answer = service
                .wait_for_state_transition_result_impl(Request::new(unflagged_wait([5; 32])))
                .await;

            // Without the second lookup the wait would end with its timeout
            let response = answer.expect("expected the repeated lookup to end the wait");
            let Some(ResponseVersion::V0(v0)) = response.into_inner().version else {
                panic!("expected a v0 response");
            };
            assert!(
                matches!(v0.result, Some(WaitResult::Error(ref error)) if error.code == 1),
                "{:?}",
                v0.result
            );
            assert_eq!(tenderdash.lookups(), 2);
        }

        #[tokio::test]
        async fn should_stop_repeating_the_lookup_once_tenderdash_says_it_holds_no_such_hash() {
            let tenderdash =
                LookupTenderdash::answering_in_turn(vec![Lookup::Failed, Lookup::Missing]).await;
            // The third lookup would come after three of the first delay, half of this wait
            let wait_timeout = 6 * TX_LOOKUP_RETRY_DELAY;
            let service = tenderdash.service(wait_timeout);

            let answer = service
                .wait_for_state_transition_result_impl(Request::new(unflagged_wait([6; 32])))
                .await;

            // The second lookup said the transaction is in no block yet, so only its event can
            // end the wait from then on: a third lookup would have come within the wait
            assert!(
                matches!(answer, Err(DapiError::Timeout(_))),
                "expected the event timeout, got {answer:?}"
            );
            assert_eq!(tenderdash.lookups(), 2);
        }

        #[tokio::test]
        async fn should_still_answer_with_the_event_while_the_lookup_keeps_failing() {
            let tenderdash = LookupTenderdash::answering_in_turn(vec![Lookup::Failed]).await;
            let service = tenderdash.service(Duration::from_secs(30));
            let hash = [7u8; 32];

            // The event comes once the lookup has failed twice
            let deliver_event = async {
                while tenderdash.lookups() < 2 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                service
                    .subscriber_manager
                    .notify(StreamingEvent::PlatformTx {
                        event: TransactionEvent {
                            hash: hex::encode_upper(hash),
                            height: 5,
                            result: TransactionResult::Success,
                            tx: None,
                        },
                    })
                    .await;
            };
            let (answer, ()) = tokio::join!(
                service.wait_for_state_transition_result_impl(Request::new(unflagged_wait(hash))),
                deliver_event
            );

            answer.expect("expected the event to end the wait");
        }
    }

    mod deadline {
        use super::super::optional_work_deadline;
        use std::time::Duration;
        use tokio::time::Instant;

        #[test]
        fn should_end_optional_work_by_the_wait_deadline_without_an_earlier_ingress_deadline() {
            let wait_deadline = Instant::now() + Duration::from_secs(30);

            assert_eq!(optional_work_deadline(wait_deadline, None), wait_deadline);
            assert_eq!(
                optional_work_deadline(wait_deadline, Some(wait_deadline + Duration::from_secs(1))),
                wait_deadline
            );
        }

        #[test]
        fn should_end_optional_work_by_the_ingress_deadline_when_it_comes_first() {
            let wait_deadline = Instant::now() + Duration::from_secs(30);
            let ingress = wait_deadline - Duration::from_secs(25);

            assert_eq!(
                optional_work_deadline(wait_deadline, Some(ingress)),
                ingress
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
