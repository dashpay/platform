//! Subscriptions to committed state transitions.
//!
//! [`Sdk::subscribe_to_state_transitions`] opens a `subscribeToStateTransitions` stream and
//! returns a [`StateTransitionSubscription`] yielding the transitions that match any of the
//! filters, as the node evaluated them, block by block, plus checkpoints.
//!
//! What a subscription is worth:
//! - delivery is at least once, and may be incomplete: a node can leave a match out;
//! - act on a transition only after reading the state it changed with a proved query;
//! - to catch up after being offline, start from a proved read's `metadata.height + 1`.
//!
//! A broken stream (including the transport's periodic streaming timeout) is resumed from the
//! height after the last checkpoint, possibly on another node, so transitions after it may be
//! delivered again. A node that sends something invalid (a wrong hash, indexes or positions out
//! of range, a checkpoint going backwards) is banned (only moved out of the rotation when
//! `ban_failed_address` is off) and the stream resumed elsewhere. A transition that matches its
//! hash but that this SDK cannot decode ends the subscription: the SDK is likely older than the
//! network. A node whose tip stalls keeps sending the same checkpoint; the SDK does not detect
//! that.
//!
//! ```no_run
//! use dash_sdk::dash_platform_queries::subscriptions::{DocumentFilter, StateTransitionFilter};
//! use dash_sdk::dpp::prelude::Identifier;
//! use dash_sdk::platform::subscriptions::SubscriptionEvent;
//! use dash_sdk::{Error, Sdk};
//!
//! async fn watch(sdk: &Sdk, contract_id: Identifier) -> Result<(), Error> {
//!     let filter = StateTransitionFilter::Documents(
//!         DocumentFilter::new(contract_id).with_document_type("note"),
//!     );
//!     let mut subscription = sdk
//!         .subscribe_to_state_transitions(vec![filter], None, None)
//!         .await?;
//!     loop {
//!         match subscription.next().await? {
//!             SubscriptionEvent::StateTransition(event) => {
//!                 println!("matched at height {}", event.block_height)
//!             }
//!             SubscriptionEvent::Checkpoint { block_height } => {
//!                 let _resume_after_restart_from = block_height + 1; // persist this
//!             }
//!         }
//!     }
//! }
//! ```

use crate::{Error, Sdk};
use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::subscribe_to_state_transitions_response_v0::{
    Responses, StateTransitionMatch,
};
use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::Version;
use dapi_grpc::platform::v0::{SubscribeToStateTransitionsRequest, SubscribeToStateTransitionsResponse};
use dapi_grpc::tonic::{Code, Status};
use dash_platform_queries::subscriptions::{subscribe_request, StateTransitionFilter};
use dpp::dashcore::hashes::{sha256, Hash};
use dpp::platform_value::DEFAULT_MAX_VALUE_DECODE_DEPTH;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use futures::{Stream, StreamExt};
use rs_dapi_client::transport::{sleep, TransportError, TransportRequest};
use rs_dapi_client::{
    Address, CanRetry, DapiClientError, DapiRequestExecutor, ExecutionError, RequestSettings,
};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Wait before the first reconnect; doubles per failure in a row, up to the configured cap.
const FIRST_RETRY_DELAY: Duration = Duration::from_secs(1);

/// Settings of a state transition subscription.
#[derive(Debug, Clone)]
pub struct SubscriptionConfig {
    /// Settings for opening each stream.
    pub request_settings: RequestSettings,
    /// A stream that sends nothing for a whole window of this length is resumed; the node
    /// checkpoints every 10 seconds while idle. Default: 60 seconds.
    pub idle_timeout: Duration,
    /// Longest wait between reconnects. Default: 30 seconds.
    pub max_backoff: Duration,
}

impl Default for SubscriptionConfig {
    fn default() -> Self {
        Self {
            request_settings: RequestSettings::default(),
            idle_timeout: Duration::from_secs(60),
            max_backoff: Duration::from_secs(30),
        }
    }
}

/// A committed state transition that matched the subscription's filters.
#[derive(Debug, Clone)]
pub struct StateTransitionEvent {
    /// Height of the block the transition executed in, as the node reports it.
    pub block_height: u64,
    /// Block time in milliseconds since the Unix epoch, as the node reports it.
    pub block_time_ms: u64,
    /// The protocol version the block executed under, as the node reports it.
    pub protocol_version: u32,
    /// SHA-256 of the serialized transition (its Tenderdash hash), computed by the SDK.
    pub state_transition_hash: [u8; 32],
    /// The transition.
    pub state_transition: StateTransition,
    /// Indexes, in the subscription's filters, of the filters the node matched.
    pub matched_filters: Vec<u32>,
    /// For a batch: positions of the document transitions the node matched.
    pub matched_batch_positions: Vec<u32>,
}

/// One item of a state transition subscription.
#[derive(Debug, Clone)]
pub enum SubscriptionEvent {
    /// A matching transition.
    StateTransition(Box<StateTransitionEvent>),
    /// Every match through `block_height` was delivered; resume from `block_height + 1`.
    Checkpoint {
        /// The highest fully scanned height.
        block_height: u64,
    },
}

/// The messages of one stream.
type Messages =
    Pin<Box<dyn Stream<Item = Result<SubscribeToStateTransitionsResponse, Status>> + Send>>;

/// The current stream, the node serving it and when it opened.
struct OpenStream {
    messages: Messages,
    address: Address,
    opened: Clock,
}

impl OpenStream {
    fn new(messages: Messages, address: Address) -> Self {
        Self {
            messages,
            address,
            opened: Clock::now(),
        }
    }
}

/// A point in time, to tell how long a stream lived: tokio's clock (which tests can pause), or
/// on wasm, which has no monotonic `Instant`, the wall clock.
struct Clock(
    #[cfg(not(target_arch = "wasm32"))] tokio::time::Instant,
    #[cfg(target_arch = "wasm32")] f64,
);

impl Clock {
    #[cfg(not(target_arch = "wasm32"))]
    fn now() -> Self {
        Self(tokio::time::Instant::now())
    }

    #[cfg(target_arch = "wasm32")]
    fn now() -> Self {
        Self(js_sys::Date::now())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    #[cfg(target_arch = "wasm32")]
    fn elapsed(&self) -> Duration {
        Duration::from_secs_f64((js_sys::Date::now() - self.0).max(0.0) / 1000.0)
    }
}

/// Why a message was not accepted.
enum Refusal {
    /// The node sent something invalid: ban it and resume elsewhere.
    NodeFault(Error),
    /// The node's bytes match their hash, but this SDK cannot decode them: every node would send
    /// the same, so the subscription ends.
    Undecodable(Error),
}

impl From<Error> for Refusal {
    fn from(error: Error) -> Self {
        Refusal::NodeFault(error)
    }
}

/// A pending open of a stream, reporting the node serving it.
type Opening = Pin<
    Box<dyn Future<Output = Result<(Messages, Address), ExecutionError<DapiClientError>>> + Send>,
>;
/// Opens a stream for a request.
type Opener = Box<dyn FnMut(SubscribeToStateTransitionsRequest) -> Opening + Send>;
/// A pending wait, kept across calls so cancelling `next()` does not restart it.
type Timer = Pin<Box<dyn Future<Output = ()> + Send>>;

/// A running subscription; see the [module documentation](self).
pub struct StateTransitionSubscription {
    sdk: Sdk,
    filters: Vec<StateTransitionFilter>,
    config: SubscriptionConfig,
    open: Opener,
    /// The open in progress, kept across calls so cancelling `next()` does not restart it.
    opening: Option<Opening>,
    /// Whether a failing node is banned, or only moved out of the rotation.
    ban_failed_address: bool,
    /// The current stream.
    stream: Option<OpenStream>,
    /// Where the next stream starts; `None` until the first checkpoint of a live start.
    resume_from: Option<u64>,
    /// The wait before the next reconnect, and the one after it.
    retry: Option<Timer>,
    retry_delay: Duration,
    idle: IdleWatch,
    /// Why the subscription ended, once it has.
    terminated: Option<String>,
}

/// Notices a stream that sent nothing for a whole window, with one timer however many messages
/// arrive (on wasm a dropped timer runs until it fires), so silence is noticed after one to two
/// windows.
struct IdleWatch {
    window: Option<Timer>,
    /// Whether a message arrived, or a stream opened, during the current window.
    heard: bool,
}

impl IdleWatch {
    fn heard(&mut self) {
        self.heard = true;
    }

    /// Completes once a whole `window` passes in which nothing was heard.
    async fn silent(&mut self, window: Duration) {
        loop {
            self.window
                .get_or_insert_with(|| Box::pin(sleep(window)))
                .as_mut()
                .await;
            self.window = None;
            if !std::mem::take(&mut self.heard) {
                return;
            }
        }
    }
}

impl Sdk {
    /// Subscribe to committed state transitions matching any of `filters` (1 to
    /// [`MAX_FILTERS`](dash_platform_queries::subscriptions::MAX_FILTERS)), scanning from
    /// `from_block_height` (inclusive), or from after the current tip when `None`. The first
    /// stream is opened as a reopen would be: a refused request fails here, a transient failure
    /// is retried by [`StateTransitionSubscription::next`].
    pub async fn subscribe_to_state_transitions(
        &self,
        filters: Vec<StateTransitionFilter>,
        from_block_height: Option<u64>,
        config: Option<SubscriptionConfig>,
    ) -> Result<StateTransitionSubscription, Error> {
        let config = config.unwrap_or_default();
        let sdk = self.clone();
        let settings = config.request_settings;
        let open: Opener = Box::new(move |request| {
            let sdk = sdk.clone();
            Box::pin(async move {
                let response = sdk.execute(request, settings).await?;
                Ok((Box::pin(response.inner) as Messages, response.address))
            })
        });
        let mut subscription = StateTransitionSubscription::new(
            self.clone(),
            filters,
            config,
            open,
            from_block_height,
        );
        subscription.reopen().await?;
        Ok(subscription)
    }
}

impl StateTransitionSubscription {
    fn new(
        sdk: Sdk,
        filters: Vec<StateTransitionFilter>,
        config: SubscriptionConfig,
        open: Opener,
        resume_from: Option<u64>,
    ) -> Self {
        // The settings `DapiClient::execute` applies to each open.
        let ban_failed_address = sdk
            .dapi_client_settings
            .override_by(SubscribeToStateTransitionsRequest::SETTINGS_OVERRIDES)
            .override_by(config.request_settings)
            .finalize()
            .ban_failed_address;
        Self {
            sdk,
            filters,
            config,
            open,
            opening: None,
            ban_failed_address,
            stream: None,
            resume_from,
            retry: None,
            retry_delay: FIRST_RETRY_DELAY,
            idle: IdleWatch {
                window: None,
                heard: true,
            },
            terminated: None,
        }
    }

    /// The next matching transition or checkpoint. Cancel-safe. Broken streams are resumed
    /// without limit; an error means the subscription cannot continue (the request was refused,
    /// a live start broke before its first checkpoint, or this SDK cannot decode a transition),
    /// and every later call fails too.
    pub async fn next(&mut self) -> Result<SubscriptionEvent, Error> {
        if let Some(reason) = &self.terminated {
            return Err(Error::Cancelled(format!(
                "the subscription ended: {reason}"
            )));
        }
        loop {
            match self.step().await {
                Ok(Some(event)) => return Ok(event),
                Ok(None) => {}
                Err(error) => {
                    self.terminated = Some(error.to_string());
                    self.stream = None;
                    return Err(error);
                }
            }
        }
    }

    /// The height a new stream would start from: the last checkpoint + 1.
    pub fn resume_height(&self) -> Option<u64> {
        self.resume_from
    }

    /// Consume the subscription as a stream of events, which ends after the first error.
    pub fn into_stream(self) -> impl Stream<Item = Result<SubscriptionEvent, Error>> {
        futures::stream::unfold(Some(self), |subscription| async move {
            let mut subscription = subscription?;
            let item = subscription.next().await;
            let next = item.is_ok().then_some(subscription);
            Some((item, next))
        })
    }

    /// Read one message, or reopen the stream if it broke; `None` when there is no event yet.
    async fn step(&mut self) -> Result<Option<SubscriptionEvent>, Error> {
        let Some(stream) = self.stream.as_mut() else {
            self.reopen().await?;
            return Ok(None);
        };
        let message = {
            let message = stream.messages.next();
            let silent = self.idle.silent(self.config.idle_timeout);
            futures::pin_mut!(silent);
            match futures::future::select(message, silent).await {
                futures::future::Either::Left((message, _)) => Some(message),
                futures::future::Either::Right(_) => None,
            }
        };
        let (cause, ban, evict) = match message {
            Some(Some(Ok(response))) => {
                self.idle.heard();
                match self.accept(response) {
                    Ok(event) => return Ok(Some(event)),
                    Err(Refusal::NodeFault(fault)) => (fault, true, false),
                    Err(Refusal::Undecodable(error)) => return Err(error),
                }
            }
            Some(Some(Err(status))) if is_transient(&status) => {
                let ban = lacks_heights(&status);
                let evict = is_server_failure(&status);
                (status_error(status), ban, evict)
            }
            Some(Some(Err(status))) => return Err(status_error(status)),
            Some(None) => (
                Error::Cancelled("the node ended the stream".to_string()),
                false,
                false,
            ),
            // Never a ban nor an eviction: a client that is offline would blame every node.
            None => (
                Error::TimeoutReached(
                    self.config.idle_timeout,
                    "no message on the state transition stream".to_string(),
                ),
                false,
                false,
            ),
        };
        let Some(stream) = self.stream.take() else {
            return Ok(None);
        };
        if ban {
            self.blame(Some(&stream.address), &cause);
        } else if evict {
            // `execute` returned before the failure, so it moved nothing off this node.
            self.sdk.address_list().evict_from_rotation(&stream.address);
        }
        // A stream that lived a whole idle window was not a failure in a row: without this, the
        // transport's streaming timeout on a quiet chain (no new checkpoint height) would grow
        // the wait before every reconnect up to the cap.
        if stream.opened.elapsed() >= self.config.idle_timeout {
            self.retry_delay = FIRST_RETRY_DELAY;
        }
        // A live start has no height to resume from before its first checkpoint: a new live
        // start would skip the blocks in between.
        if self.resume_from.is_none() {
            return Err(Error::Cancelled(format!(
                "the stream broke before its first checkpoint ({cause}); subscribe again"
            )));
        }
        tracing::debug!(%cause, "state transition stream broke; resuming");
        self.schedule_retry();
        Ok(None)
    }

    /// Open a new stream once the wait after the last break has passed. A transient failure
    /// schedules another attempt.
    async fn reopen(&mut self) -> Result<(), Error> {
        if let Some(retry) = self.retry.as_mut() {
            retry.await;
            self.retry = None;
        }
        let opening = match self.opening.take() {
            Some(opening) => opening,
            None => (self.open)(subscribe_request(&self.filters, self.resume_from)?),
        };
        let opened = self.opening.insert(opening).await;
        self.opening = None;
        match opened {
            Ok((messages, address)) => {
                self.stream = Some(OpenStream::new(messages, address));
                self.idle.heard();
                Ok(())
            }
            Err(error) => {
                // `DapiClient::execute` already blamed the node for retryable failures.
                let transient = match &error.inner {
                    DapiClientError::Transport(TransportError::Grpc(status)) => {
                        if lacks_heights(status) {
                            let cause = status_error(status.clone());
                            self.blame(error.address.as_ref(), &cause);
                        }
                        is_transient(status)
                    }
                    other => other.can_retry() || other.is_no_available_addresses(),
                };
                if !transient {
                    return Err(Error::from(error.inner));
                }
                tracing::debug!(error = %error.inner, "cannot reopen the state transition stream; retrying");
                self.schedule_retry();
                Ok(())
            }
        }
    }

    fn schedule_retry(&mut self) {
        let delay = self.retry_delay.min(self.config.max_backoff);
        self.retry = Some(Box::pin(sleep(delay)));
        self.retry_delay = delay.saturating_mul(2);
    }

    /// Ban the node, or when banning is disabled, only move the next stream off it.
    fn blame(&self, address: Option<&Address>, cause: &Error) {
        let Some(address) = address else {
            return;
        };
        let addresses = self.sdk.address_list();
        if self.ban_failed_address {
            addresses.ban_with_reason(address, Some(format!("state transition stream: {cause}")));
        } else {
            addresses.evict_from_rotation(address);
        }
    }

    /// The event a response carries.
    fn accept(
        &mut self,
        response: SubscribeToStateTransitionsResponse,
    ) -> Result<SubscriptionEvent, Refusal> {
        let Some(Version::V0(response)) = response.version else {
            return Err(node_fault("unknown response version").into());
        };
        match response.responses {
            Some(Responses::Checkpoint(checkpoint)) => {
                let resume_from = checkpoint
                    .block_height
                    .checked_add(1)
                    .ok_or_else(|| node_fault("checkpoint at the maximum height"))?;
                if self
                    .resume_from
                    .is_some_and(|current| resume_from < current)
                {
                    return Err(node_fault("checkpoint before the previous one").into());
                }
                if self.resume_from != Some(resume_from) {
                    self.retry_delay = FIRST_RETRY_DELAY;
                }
                self.resume_from = Some(resume_from);
                Ok(SubscriptionEvent::Checkpoint {
                    block_height: checkpoint.block_height,
                })
            }
            Some(Responses::StateTransition(matched)) => self.accept_transition(matched),
            None => Err(node_fault("unknown response kind").into()),
        }
    }

    fn accept_transition(
        &self,
        matched: StateTransitionMatch,
    ) -> Result<SubscriptionEvent, Refusal> {
        let state_transition_hash = sha256::Hash::hash(&matched.state_transition).to_byte_array();
        if matched.state_transition_hash != state_transition_hash {
            return Err(node_fault("state transition hash does not match its bytes").into());
        }
        if matched.matched_filters.is_empty()
            || matched
                .matched_filters
                .iter()
                .any(|&index| index as usize >= self.filters.len())
        {
            return Err(node_fault("matched filter indexes out of range").into());
        }
        let state_transition = decode(&matched.state_transition, matched.protocol_version)
            .map_err(|cause| {
                Refusal::Undecodable(Error::Protocol(ProtocolError::DecodingError(format!(
                    "cannot decode the state transition at height {} (protocol version {}): \
                     {cause}; the node's bytes match their hash, so this SDK is likely older \
                     than the network: upgrade it",
                    matched.block_height, matched.protocol_version
                ))))
            })?;
        // Positions index the batch's transitions; any other transition has none.
        let transitions = match &state_transition {
            StateTransition::Batch(batch) => batch.transitions_len(),
            _ => 0,
        };
        if matched
            .matched_batch_positions
            .iter()
            .any(|&position| position as usize >= transitions)
        {
            return Err(node_fault("matched batch positions out of range").into());
        }
        Ok(SubscriptionEvent::StateTransition(Box::new(
            StateTransitionEvent {
                block_height: matched.block_height,
                block_time_ms: matched.block_time_ms,
                protocol_version: matched.protocol_version,
                state_transition_hash,
                state_transition,
                matched_filters: matched.matched_filters,
                matched_batch_positions: matched.matched_batch_positions,
            },
        )))
    }
}

/// Decode `bytes` as the node did, under the protocol version the block executed with, except
/// that values are never nested deeper than the default bound, even under a version that set
/// none: the version is the node's word, and a deeper tree could exhaust the stack.
fn decode(bytes: &[u8], protocol_version: u32) -> Result<StateTransition, String> {
    let platform_version = PlatformVersion::get(protocol_version)
        .map_err(|_| format!("protocol version {protocol_version} is unknown to this SDK"))?;
    let decoded = if platform_version
        .system_limits
        .max_document_value_depth
        .is_some()
    {
        StateTransition::deserialize_from_bytes_untrusted_in_version(bytes, platform_version)
    } else {
        let mut bounded = platform_version.clone();
        bounded.system_limits.max_document_value_depth =
            u16::try_from(DEFAULT_MAX_VALUE_DECODE_DEPTH).ok();
        StateTransition::deserialize_from_bytes_untrusted_in_version(bytes, &bounded)
    };
    decoded.map_err(|error| error.to_string())
}

/// Whether another stream may get past a stream or open failing with `status`.
fn is_transient(status: &Status) -> bool {
    status.can_retry() || lacks_heights(status)
}

/// Whether this node cannot serve the requested heights (it lacks or cannot decode a block).
/// `OutOfRange`, a start behind every node's replay window, ends the subscription instead.
fn lacks_heights(status: &Status) -> bool {
    status.code() == Code::FailedPrecondition
}

/// Whether the node failed an open stream itself: the next stream should go to another node,
/// though this one is not banned.
fn is_server_failure(status: &Status) -> bool {
    matches!(
        status.code(),
        Code::Unavailable | Code::Internal | Code::Unknown
    )
}

fn status_error(status: Status) -> Error {
    Error::from(DapiClientError::Transport(TransportError::Grpc(status)))
}

fn node_fault(message: &str) -> Error {
    Error::Protocol(ProtocolError::DecodingError(format!(
        "invalid state transition stream message: {message}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::subscribe_to_state_transitions_request::Version as RequestVersion;
    use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::subscribe_to_state_transitions_response_v0::Checkpoint;
    use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::SubscribeToStateTransitionsResponseV0;
    use dash_platform_queries::subscriptions::Role;
    use dpp::address_funds::PlatformAddress;
    use dpp::platform_value::Value;
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::batched_transition::document_create_transition::v0::DocumentCreateTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransition;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    type Opens = Arc<Mutex<Vec<Option<u64>>>>;

    fn node() -> Address {
        "https://127.0.0.1:1".parse().expect("address")
    }

    /// A subscription with two filters and no stream yet, whose opens yield `opens` in turn
    /// (then are refused), recording the height each open asks for.
    fn unopened(
        resume_from: Option<u64>,
        opens: Vec<Result<Messages, Status>>,
    ) -> (StateTransitionSubscription, Opens) {
        unopened_with(
            Duration::ZERO,
            SubscriptionConfig::default(),
            resume_from,
            opens,
        )
    }

    /// As [`unopened`], with each open taking `delay` and the subscription using `config`.
    fn unopened_with(
        delay: Duration,
        config: SubscriptionConfig,
        resume_from: Option<u64>,
        opens: Vec<Result<Messages, Status>>,
    ) -> (StateTransitionSubscription, Opens) {
        let recorded = Opens::default();
        let heights = recorded.clone();
        let mut opens = VecDeque::from(opens);
        let open: Opener = Box::new(move |request: SubscribeToStateTransitionsRequest| {
            let Some(RequestVersion::V0(request)) = request.version else {
                panic!("v0 request");
            };
            heights.lock().unwrap().push(request.from_block_height);
            let result = opens
                .pop_front()
                .unwrap_or_else(|| Err(Status::invalid_argument("no more streams")))
                .map(|messages| (messages, node()))
                .map_err(|status| ExecutionError {
                    inner: DapiClientError::Transport(TransportError::Grpc(status)),
                    retries: 0,
                    address: Some(node()),
                });
            Box::pin(async move {
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                result
            })
        });
        let address = |byte| StateTransitionFilter::Addresses {
            addresses: vec![PlatformAddress::P2pkh([byte; 20])],
            role: Role::Any,
        };
        let subscription = StateTransitionSubscription::new(
            Sdk::new_mock(),
            vec![address(1), address(2)],
            config,
            open,
            resume_from,
        );
        (subscription, recorded)
    }

    /// A subscription with two filters whose first stream yields `first` and whose reopens yield
    /// `next` in turn (then are refused), recording the height each reopen asks for.
    fn subscription(
        resume_from: Option<u64>,
        first: Messages,
        next: Vec<Messages>,
    ) -> (StateTransitionSubscription, Opens) {
        let (mut subscription, opens) = unopened(resume_from, next.into_iter().map(Ok).collect());
        subscription.stream = Some(OpenStream::new(first, node()));
        (subscription, opens)
    }

    fn response(message: Responses) -> SubscribeToStateTransitionsResponse {
        SubscribeToStateTransitionsResponse {
            version: Some(Version::V0(SubscribeToStateTransitionsResponseV0 {
                responses: Some(message),
            })),
        }
    }

    /// A stream sending `messages` and then ending.
    fn ending(messages: Vec<Result<Responses, Status>>) -> Messages {
        Box::pin(futures::stream::iter(
            messages.into_iter().map(|message| message.map(response)),
        ))
    }

    /// A stream sending a checkpoint at `block_height` `count` times, `every` apart, then ending.
    fn repeating(block_height: u64, every: Duration, count: usize) -> Messages {
        Box::pin(futures::stream::unfold(0, move |sent| async move {
            if sent == count {
                return None;
            }
            if sent > 0 {
                tokio::time::sleep(every).await;
            }
            let message = response(Responses::Checkpoint(Checkpoint { block_height }));
            Some((Ok(message), sent + 1))
        }))
    }

    /// A stream sending `messages` and then nothing.
    fn silent(messages: Vec<Result<Responses, Status>>) -> Messages {
        Box::pin(ending(messages).chain(futures::stream::pending()))
    }

    fn checkpoint(block_height: u64) -> Result<Responses, Status> {
        Ok(Responses::Checkpoint(Checkpoint { block_height }))
    }

    fn transition(bytes: Vec<u8>) -> StateTransitionMatch {
        StateTransitionMatch {
            block_height: 45,
            block_time_ms: 1,
            protocol_version: PlatformVersion::latest().protocol_version,
            state_transition_hash: sha256::Hash::hash(&bytes).to_byte_array().to_vec(),
            state_transition: bytes.into(),
            matched_filters: vec![1],
            matched_batch_positions: vec![],
        }
    }

    fn transfer() -> Vec<u8> {
        StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
            IdentityCreditTransferTransitionV0 {
                identity_id: Identifier::from([1u8; 32]),
                recipient_id: Identifier::from([7u8; 32]),
                amount: 10,
                ..Default::default()
            },
        ))
        .serialize_to_bytes()
        .expect("serializable")
    }

    /// A batch creating a document whose data nests `depth` arrays.
    fn nested_batch(depth: usize) -> Vec<u8> {
        let nested = (0..depth).fold(Value::Null, |value, _| Value::Array(vec![value]));
        StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from([1u8; 32]),
            transitions: vec![BatchedTransition::Document(DocumentTransition::Create(
                DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                    base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                        id: Identifier::from([2u8; 32]),
                        identity_contract_nonce: 1,
                        document_type_name: "note".to_string(),
                        data_contract_id: Identifier::from([3u8; 32]),
                    }),
                    entropy: [0; 32],
                    data: [("nested".to_string(), nested)].into(),
                    prefunded_voting_balance: None,
                }),
            ))],
            ..Default::default()
        }))
        .serialize_to_bytes()
        .expect("serializable")
    }

    fn opens(opens: &Opens) -> Vec<Option<u64>> {
        opens.lock().unwrap().clone()
    }

    #[tokio::test(start_paused = true)]
    async fn should_deliver_a_transition_with_the_nodes_matched_filters() {
        let bytes = transfer();
        let first = ending(vec![
            checkpoint(41),
            Ok(Responses::StateTransition(transition(bytes.clone()))),
        ]);
        let (mut subscription, _) = subscription(Some(42), first, vec![]);
        assert!(matches!(
            subscription.next().await,
            Ok(SubscriptionEvent::Checkpoint { block_height: 41 })
        ));
        let Ok(SubscriptionEvent::StateTransition(event)) = subscription.next().await else {
            panic!("expected a transition");
        };
        assert_eq!(event.block_height, 45);
        assert_eq!(event.matched_filters, vec![1]);
        assert_eq!(
            event.state_transition_hash,
            sha256::Hash::hash(&bytes).to_byte_array()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn should_treat_a_wrong_hash_as_a_node_fault_and_resume() {
        let mut wrong_hash = transition(transfer());
        wrong_hash.state_transition_hash = vec![0; 32];
        let first = silent(vec![
            checkpoint(41),
            Ok(Responses::StateTransition(wrong_hash)),
        ]);
        let second = silent(vec![
            checkpoint(41),
            Ok(Responses::StateTransition(transition(transfer()))),
        ]);
        let (mut subscription, opened) = subscription(Some(42), first, vec![second]);
        assert!(subscription.next().await.is_ok());
        assert!(matches!(
            subscription.next().await,
            Ok(SubscriptionEvent::Checkpoint { block_height: 41 })
        ));
        assert!(matches!(
            subscription.next().await,
            Ok(SubscriptionEvent::StateTransition(_))
        ));
        assert_eq!(opens(&opened), vec![Some(42)]);
    }

    /// What `accept` makes of a transition.
    fn outcome(result: Result<SubscriptionEvent, Refusal>) -> Result<(), String> {
        match result {
            Ok(_) => Ok(()),
            Err(Refusal::NodeFault(error)) => Err(format!("node fault: {error}")),
            Err(Refusal::Undecodable(error)) => Err(format!("undecodable: {error}")),
        }
    }

    #[test]
    fn should_refuse_values_nested_deeper_than_the_default_bound_without_blaming_the_node() {
        let (mut subscription, _) = subscription(Some(42), ending(vec![]), vec![]);
        let no_limit = (1..PlatformVersion::latest().protocol_version)
            .rev()
            .filter_map(|version| PlatformVersion::get(version).ok())
            .find(|version| version.system_limits.max_document_value_depth.is_none())
            .expect("an early protocol version sets no value depth limit")
            .protocol_version;
        for protocol_version in [PlatformVersion::latest().protocol_version, no_limit] {
            let mut accept = |depth| {
                outcome(subscription.accept(response(Responses::StateTransition(
                    StateTransitionMatch {
                        protocol_version,
                        matched_batch_positions: vec![0],
                        ..transition(nested_batch(depth))
                    },
                ))))
            };
            assert_eq!(accept(16), Ok(()), "protocol version {protocol_version}");
            // The node vouched for the bytes; the SDK's own bound refuses them.
            let error = accept(300).expect_err("too deep");
            assert!(
                error.starts_with("undecodable") && error.contains("exceeds maximum 256"),
                "{error}"
            );
        }
    }

    #[test]
    fn should_not_blame_the_node_for_a_protocol_version_this_sdk_does_not_know() {
        let (mut subscription, _) = subscription(Some(42), ending(vec![]), vec![]);
        let unknown = PlatformVersion::latest().protocol_version + 1;
        let error = outcome(subscription.accept(response(Responses::StateTransition(
            StateTransitionMatch {
                protocol_version: unknown,
                ..transition(transfer())
            },
        ))))
        .expect_err("unknown version");
        assert!(
            error.starts_with("undecodable") && error.contains("upgrade"),
            "{error}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn should_end_the_subscription_on_an_undecodable_transition_without_resuming() {
        let mut undecodable = transition(vec![0xff, 0x00, 0x13]);
        undecodable.block_height = 77;
        let first = silent(vec![
            checkpoint(41),
            Ok(Responses::StateTransition(undecodable)),
        ]);
        let (mut subscription, opened) =
            subscription(Some(42), first, vec![silent(vec![checkpoint(41)])]);
        assert!(subscription.next().await.is_ok());
        let error = subscription.next().await.expect_err("undecodable");
        assert!(
            error.to_string().contains("height 77") && error.to_string().contains("upgrade"),
            "{error}"
        );
        assert!(subscription.next().await.is_err());
        assert!(opens(&opened).is_empty(), "no other node is tried");
    }

    #[test]
    fn should_treat_batch_positions_out_of_range_as_node_faults() {
        let (mut subscription, _) = subscription(Some(42), ending(vec![]), vec![]);
        let mut accept = |bytes, matched_batch_positions| {
            outcome(subscription.accept(response(Responses::StateTransition(
                StateTransitionMatch {
                    matched_batch_positions,
                    ..transition(bytes)
                },
            ))))
        };
        assert_eq!(accept(nested_batch(1), vec![0]), Ok(()));
        assert_eq!(accept(transfer(), vec![]), Ok(()));
        for (bytes, positions) in [(nested_batch(1), vec![1]), (transfer(), vec![0])] {
            let error = accept(bytes, positions).expect_err("out of range");
            assert!(
                error.starts_with("node fault") && error.contains("batch positions"),
                "{error}"
            );
        }
    }

    #[test]
    fn should_treat_invalid_checkpoints_filter_indexes_and_envelopes_as_node_faults() {
        let (mut subscription, _) = subscription(Some(42), ending(vec![]), vec![]);
        let response = |responses| SubscribeToStateTransitionsResponse {
            version: Some(Version::V0(SubscribeToStateTransitionsResponseV0 {
                responses,
            })),
        };
        let with_filters = |matched_filters| {
            response(Some(Responses::StateTransition(StateTransitionMatch {
                matched_filters,
                ..transition(transfer())
            })))
        };
        for invalid in [
            response(Some(Responses::Checkpoint(Checkpoint { block_height: 40 }))),
            response(Some(Responses::StateTransition(StateTransitionMatch {
                state_transition_hash: vec![0; 32],
                ..transition(transfer())
            }))),
            response(Some(Responses::Checkpoint(Checkpoint {
                block_height: u64::MAX,
            }))),
            with_filters(vec![]),
            with_filters(vec![2]),
            response(None),
            SubscribeToStateTransitionsResponse { version: None },
        ] {
            assert!(matches!(
                subscription.accept(invalid),
                Err(Refusal::NodeFault(_))
            ));
        }
        assert_eq!(subscription.resume_height(), Some(42));
    }

    #[tokio::test(start_paused = true)]
    async fn should_resume_from_the_last_checkpoint_after_a_transient_error() {
        let first = ending(vec![
            checkpoint(41),
            checkpoint(43),
            Err(Status::unavailable("restarting")),
        ]);
        let second = silent(vec![checkpoint(43), checkpoint(50)]);
        let (mut subscription, opened) = subscription(Some(42), first, vec![second]);
        for expected in [41, 43, 43, 50] {
            assert!(matches!(
                subscription.next().await,
                Ok(SubscriptionEvent::Checkpoint { block_height }) if block_height == expected
            ));
        }
        assert_eq!(opens(&opened), vec![Some(44)]);
        assert_eq!(subscription.resume_height(), Some(51));
    }

    #[test]
    fn should_resume_after_transient_failures_but_not_after_refusals() {
        for code in [
            Code::Unavailable,
            Code::DeadlineExceeded,
            Code::ResourceExhausted,
            Code::FailedPrecondition,
        ] {
            assert!(is_transient(&Status::new(code, "")), "{code:?}");
        }
        for code in [Code::InvalidArgument, Code::NotFound, Code::OutOfRange] {
            assert!(!is_transient(&Status::new(code, "")), "{code:?}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn should_keep_failing_after_the_subscription_ended() {
        let first = ending(vec![
            checkpoint(41),
            Err(Status::invalid_argument("refused")),
        ]);
        let (mut subscription, opened) = subscription(Some(42), first, vec![]);
        assert!(subscription.next().await.is_ok());
        let error = subscription.next().await.expect_err("refused");
        let later = subscription.next().await.expect_err("ended");
        assert!(later.to_string().contains(&error.to_string()), "{later}");
        assert!(opens(&opened).is_empty());
        assert!(subscription.stream.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn should_fail_a_live_start_that_breaks_before_its_first_checkpoint() {
        let first = ending(vec![Err(Status::unavailable("restarting"))]);
        let (mut subscription, opened) = subscription(None, first, vec![]);
        let error = subscription.next().await.expect_err("cannot resume");
        assert!(error.to_string().contains("first checkpoint"), "{error}");
        assert!(opens(&opened).is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn should_open_the_first_stream_like_a_reopen() {
        // A node that lacks the start height is skipped (and banned), not fatal.
        let (mut subscription, opened) = unopened(
            Some(42),
            vec![
                Err(Status::failed_precondition("block 42 is pruned")),
                Ok(silent(vec![checkpoint(41)])),
            ],
        );
        subscription.reopen().await.expect("a transient failure");
        assert!(subscription.stream.is_none());
        assert!(matches!(
            subscription.next().await,
            Ok(SubscriptionEvent::Checkpoint { block_height: 41 })
        ));
        assert_eq!(opens(&opened), vec![Some(42), Some(42)]);

        // A refusal, or a start behind every node's replay window, fails the first open.
        for status in [
            Status::invalid_argument("bad filter"),
            Status::out_of_range("too far behind"),
        ] {
            let (mut subscription, opened) = unopened(Some(42), vec![Err(status)]);
            assert!(subscription.reopen().await.is_err());
            assert_eq!(opens(&opened), vec![Some(42)]);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn should_reset_the_backoff_after_a_stream_that_lived_an_idle_window() {
        // A quiet chain: the checkpoint never advances, and the stream ends after 70 seconds.
        let first = repeating(41, Duration::from_secs(10), 8);
        let second = silent(vec![checkpoint(41)]);
        let (mut subscription, opened) = subscription(Some(42), first, vec![second]);
        // As after several failures in a row.
        subscription.retry_delay = Duration::from_secs(16);
        for _ in 0..8 {
            assert!(matches!(
                subscription.next().await,
                Ok(SubscriptionEvent::Checkpoint { block_height: 41 })
            ));
        }
        let ended = tokio::time::Instant::now();
        assert!(matches!(
            subscription.next().await,
            Ok(SubscriptionEvent::Checkpoint { block_height: 41 })
        ));
        assert_eq!(ended.elapsed(), FIRST_RETRY_DELAY);
        assert_eq!(opens(&opened), vec![Some(42)]);
    }

    #[tokio::test(start_paused = true)]
    async fn should_move_the_next_stream_off_a_node_that_failed_an_open_stream() {
        for (code, ending_with, moved) in [
            (Some(Code::Unavailable), true, true),
            (Some(Code::Internal), true, true),
            (Some(Code::Unknown), true, true),
            // A clean end, and the idle watchdog, blame no node.
            (None, true, false),
            (None, false, false),
        ] {
            let failing = ending(
                std::iter::once(checkpoint(41))
                    .chain(code.map(|code| Err(Status::new(code, "cannot read the block store"))))
                    .collect(),
            );
            let first = if ending_with {
                failing
            } else {
                silent(vec![checkpoint(41)])
            };
            let (mut subscription, _) =
                subscription(Some(42), first, vec![silent(vec![checkpoint(41)])]);
            // One active node, failing, and a healthy standby.
            let mut addresses = subscription.sdk.address_list().clone();
            for port in [1, 2] {
                addresses.add(
                    format!("https://127.0.0.1:{port}")
                        .parse()
                        .expect("address"),
                );
            }
            let addresses = addresses.with_active_set_size(1);
            let active = addresses.get_live_address().expect("an active node");
            subscription.stream.as_mut().expect("a stream").address = active.clone();

            for _ in 0..2 {
                assert!(subscription.next().await.is_ok(), "{code:?}");
            }
            assert!(!addresses.is_banned(&active), "{code:?}");
            assert_eq!(
                addresses.get_live_address().expect("a node") != active,
                moved,
                "{code:?}"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn should_resume_a_silent_stream_after_the_idle_timeout() {
        let first = silent(vec![checkpoint(41)]);
        let second = silent(vec![checkpoint(41)]);
        let (mut subscription, opened) = subscription(Some(42), first, vec![second]);
        assert!(subscription.next().await.is_ok());
        let started = tokio::time::Instant::now();
        // A call cancelled before the stream goes silent does not restart the idle window.
        assert!(
            tokio::time::timeout(Duration::from_secs(30), subscription.next())
                .await
                .is_err()
        );
        assert!(matches!(
            subscription.next().await,
            Ok(SubscriptionEvent::Checkpoint { block_height: 41 })
        ));
        let waited = started.elapsed();
        assert!(
            waited >= Duration::from_secs(60) && waited <= Duration::from_secs(121),
            "{waited:?}"
        );
        assert_eq!(opens(&opened), vec![Some(42)]);
    }

    #[tokio::test(start_paused = true)]
    async fn should_keep_a_pending_open_across_cancelled_calls() {
        let (mut subscription, opened) = unopened_with(
            Duration::from_secs(10),
            SubscriptionConfig::default(),
            Some(42),
            vec![Ok(silent(vec![checkpoint(41)]))],
        );
        let started = tokio::time::Instant::now();
        for _ in 0..5 {
            assert!(
                tokio::time::timeout(Duration::from_secs(1), subscription.next())
                    .await
                    .is_err()
            );
        }
        assert!(matches!(
            subscription.next().await,
            Ok(SubscriptionEvent::Checkpoint { block_height: 41 })
        ));
        assert_eq!(started.elapsed(), Duration::from_secs(10));
        assert_eq!(opens(&opened), vec![Some(42)]);
    }

    #[tokio::test(start_paused = true)]
    async fn should_ban_a_node_lacking_heights_only_when_banning_is_enabled() {
        for ban_failed_address in [true, false] {
            let config = SubscriptionConfig {
                request_settings: RequestSettings {
                    ban_failed_address: Some(ban_failed_address),
                    ..RequestSettings::default()
                },
                ..SubscriptionConfig::default()
            };
            let lacks_heights = || Status::failed_precondition("block 42 is pruned");
            for streamed in [false, true] {
                let mut opens = vec![Ok(silent(vec![checkpoint(41)]))];
                if !streamed {
                    opens.insert(0, Err(lacks_heights()));
                }
                let (mut subscription, _) =
                    unopened_with(Duration::ZERO, config.clone(), Some(42), opens);
                if streamed {
                    let first = ending(vec![checkpoint(41), Err(lacks_heights())]);
                    subscription.stream = Some(OpenStream::new(first, node()));
                }
                // The failing node is active, with a healthy standby.
                let mut addresses = subscription.sdk.address_list().clone();
                addresses.add(node());
                let mut addresses = addresses.with_active_set_size(1);
                assert_eq!(addresses.get_live_address(), Some(node()));
                addresses.add("https://127.0.0.1:2".parse().expect("address"));

                let calls = if streamed { 2 } else { 1 };
                for _ in 0..calls {
                    assert!(subscription.next().await.is_ok());
                }
                let case = format!("ban {ban_failed_address}, streamed {streamed}");
                assert_eq!(addresses.is_banned(&node()), ban_failed_address, "{case}");
                assert_ne!(addresses.get_live_address(), Some(node()), "{case}");
            }
        }
    }
}
