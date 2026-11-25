//! Subscriptions to committed state transitions.
//!
//! [`Sdk::subscribe_to_state_transitions`] opens a `subscribeToStateTransitions` stream and
//! returns a [`StateTransitionSubscription`] yielding the transitions that match any of the
//! filters, as the node evaluated them, block by block, plus checkpoints. Nodes serve the stream
//! from protocol version 16; on a network not proven to run it, subscribing is refused.
//!
//! What a subscription is worth:
//! - delivery is at least once, and may be incomplete: a node can leave a match out;
//! - act on a transition only after reading the state it changed with a proved query;
//! - to catch up after being offline, start from a proved read's `metadata.height + 1`.
//!
//! A broken stream (including the transport's periodic streaming timeout) is resumed from the
//! height after the last checkpoint, possibly on another node, so transitions after it may be
//! delivered again. A node that sends something invalid (a wrong hash, indexes or positions out
//! of range, a stream that opens above the height it was asked to start at or with anything
//! but a checkpoint, a later checkpoint going backwards) is banned (only moved out of the
//! rotation when `ban_failed_address` is off) and the stream resumed elsewhere. A node that
//! lacks the heights asked for, or has no room for another stream, still serves other requests:
//! the stream moves off it and it is not banned. A transition this SDK cannot decode is asked
//! of another node first; when a second node delivers the same bytes the subscription ends:
//! the SDK is likely older than the network.
//!
//! A checkpoint is the node's word like the rest of the stream, and one that names a height
//! the chain has not reached is taken as it is. It hides the blocks committed between its
//! arrival and the next stream's start, up to that height (at most 1,000 blocks): the next
//! node starts no higher than the block after its own tip, opens with the checkpoint there,
//! and the place to resume from follows it down, so only blocks committed after it opens are
//! scanned. No honest node sends such a checkpoint; this is one way a node leaves matches
//! out, and a proved read is the remedy. A node more than 1,000 blocks short of the height
//! asked for refuses the stream, as a node that pruned a height does: once 8 in a row lack
//! the heights asked for, the subscription ends, and the app catches up from a proved read.
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

use crate::platform::ProtocolRequirement;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::subscribe_to_state_transitions_response_v0::{
    Responses, StateTransitionMatch,
};
use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::Version;
use dapi_grpc::platform::v0::{SubscribeToStateTransitionsRequest, SubscribeToStateTransitionsResponse};
use dapi_grpc::tonic::{Code, Status};
use dash_platform_queries::subscriptions::{
    decode_committed_state_transition, subscribe_request, StateTransitionFilter,
    MAX_START_ABOVE_TIP,
};
use dpp::dashcore::hashes::sha256;
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

/// Nodes in a row that may lack the heights asked for, without the place to resume from
/// moving, before the subscription ends.
const MAX_NODES_LACKING_HEIGHTS_IN_A_ROW: u32 = 8;

/// Times one node may send a transition this SDK cannot decode, with no other node sending
/// it, before the subscription ends.
const MAX_UNDECODABLE_FROM_ONE_NODE: u32 = 3;

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
    first_message: FirstMessage,
}

/// What a stream must send before anything else: the checkpoint that says where it starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FirstMessage {
    /// A checkpoint no higher than this height, the one before the height the stream was
    /// opened at, and less than [`MAX_START_ABOVE_TIP`] below it. A node starts there, or
    /// lower when its chain has not reached it, which only delivers again. One that starts
    /// higher would move the place to resume from past blocks nobody scanned. No node
    /// starts lower than that: it refuses a start so far above its tip, and following one
    /// that did could put the place to resume from behind what any node replays.
    CheckpointUpTo(u64),
    /// A checkpoint at the height the node starts after: a live start names none.
    AnyCheckpoint,
    /// It was sent.
    Sent,
}

impl OpenStream {
    /// A stream opened at `from_block_height`, or live.
    fn new(messages: Messages, address: Address, from_block_height: Option<u64>) -> Self {
        Self {
            messages,
            address,
            opened: Clock::now(),
            first_message: match from_block_height {
                Some(height) => FirstMessage::CheckpointUpTo(height.saturating_sub(1)),
                None => FirstMessage::AnyCheckpoint,
            },
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
    /// This SDK cannot decode the node's bytes. Their hash is the node's word too, so they
    /// may be this node's alone: resume on another node, without blaming this one.
    AskAnotherNode(Error),
    /// A second node delivered the bytes this SDK cannot decode, or the only node that
    /// answers keeps delivering them: the subscription ends.
    Undecodable(Error),
}

/// A transition this SDK could not decode, as the stream named it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UndecodedTransition {
    block_height: u64,
    state_transition_hash: [u8; 32],
}

/// A transition this SDK could not decode, the node that sent it first, and how many times
/// that node has sent it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Undecoded {
    transition: UndecodedTransition,
    node: Option<Address>,
    times: u32,
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
    /// The transition a stream sent that this SDK could not decode, until a checkpoint passes
    /// its block.
    undecoded: Option<Undecoded>,
    /// Nodes in a row that lacked the heights asked for, since the place to resume from moved.
    lacking_heights_in_a_row: u32,
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
    ///
    /// Nodes serve the stream from protocol version 16. Before the network is proven to run it,
    /// the call is refused with [`Error::ProtocolVersionNotActive`] and no stream is opened.
    pub async fn subscribe_to_state_transitions(
        &self,
        filters: Vec<StateTransitionFilter>,
        from_block_height: Option<u64>,
        config: Option<SubscriptionConfig>,
    ) -> Result<StateTransitionSubscription, Error> {
        let config = config.unwrap_or_default();
        // A node of an older release does not know the method. Checked once: a network's
        // protocol version never goes down, so every reopen is served too.
        let first_request = subscribe_request(&filters, from_block_height)?;
        self.ensure_network_serves(
            ProtocolRequirement::of_request(&first_request),
            Some(config.request_settings),
        )
        .await?;
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
            undecoded: None,
            lacking_heights_in_a_row: 0,
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
    /// a live start broke before its first checkpoint, two nodes delivered a transition this
    /// SDK cannot decode or the only node that answers kept delivering it, or no node holds the
    /// heights asked for), and every later call fails too.
    ///
    /// A checkpoint can be lower than one before it: a stream that resumes on a node whose
    /// chain has not reached the place to resume from starts at that node's tip.
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
                    Err(Refusal::AskAnotherNode(cause)) => (cause, false, true),
                    Err(Refusal::Undecodable(error)) => return Err(error),
                }
            }
            Some(Some(Err(status))) if is_transient(&status) => {
                if lacks_heights(&status) && self.gives_up_on_the_heights() {
                    return Err(status_error(status));
                }
                // A node lacking the heights serves everything else: it is passed over for
                // the next stream, as one that failed this stream is, and not banned.
                let evict = is_server_failure(&status) || lacks_heights(&status);
                (status_error(status), false, evict)
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
                self.stream = Some(OpenStream::new(messages, address, self.resume_from));
                self.idle.heard();
                Ok(())
            }
            Err(error) => {
                // `DapiClient::execute` already blamed the node for retryable failures. A
                // node lacking the heights is not at fault: it is only passed over.
                let transient = match &error.inner {
                    DapiClientError::Transport(TransportError::Grpc(status)) => {
                        let lacks_heights = lacks_heights(status);
                        if let (true, Some(address)) = (lacks_heights, &error.address) {
                            self.sdk.address_list().evict_from_rotation(address);
                        }
                        is_transient(status) && !(lacks_heights && self.gives_up_on_the_heights())
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

    /// Count a node that lacks the heights asked for. `true` once so many did in a row,
    /// without the place to resume from moving, that the subscription ends: the heights are
    /// pruned on the nodes this SDK reaches, or a checkpoint named a height the chain has not
    /// reached. Either way no stream will come, and the app must catch up from a proved read.
    fn gives_up_on_the_heights(&mut self) -> bool {
        self.lacking_heights_in_a_row += 1;
        self.lacking_heights_in_a_row >= MAX_NODES_LACKING_HEIGHTS_IN_A_ROW
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
        // Every stream opens with the checkpoint that says where it starts.
        let first_message = match self.stream.as_mut() {
            Some(stream) => std::mem::replace(&mut stream.first_message, FirstMessage::Sent),
            None => FirstMessage::Sent,
        };
        let opens = match (first_message, &response.responses) {
            (FirstMessage::Sent, _) => false,
            (FirstMessage::AnyCheckpoint, Some(Responses::Checkpoint(_))) => true,
            (FirstMessage::CheckpointUpTo(height), Some(Responses::Checkpoint(checkpoint)))
                if checkpoint.block_height <= height
                    && checkpoint.block_height.saturating_add(MAX_START_ABOVE_TIP) > height =>
            {
                true
            }
            _ => {
                return Err(node_fault(
                    "the stream did not open with a checkpoint at its start, or within what \
                     a node may start below it",
                )
                .into())
            }
        };
        match response.responses {
            Some(Responses::Checkpoint(checkpoint)) => {
                let resume_from = checkpoint
                    .block_height
                    .checked_add(1)
                    .ok_or_else(|| node_fault("checkpoint at the maximum height"))?;
                let behind = self
                    .resume_from
                    .is_some_and(|current| resume_from < current);
                // Only the checkpoint a stream opens with may be behind: its node starts at
                // its own tip when the chain it sees has not reached the height asked for,
                // and the place to resume from follows it, so nothing in between is skipped.
                if behind && !opens {
                    return Err(node_fault("checkpoint before the previous one").into());
                }
                if self.resume_from.is_none_or(|current| resume_from > current) {
                    self.retry_delay = FIRST_RETRY_DELAY;
                    self.lacking_heights_in_a_row = 0;
                }
                self.resume_from = Some(resume_from);
                // A stream got past the block: what an earlier one sent of it is settled.
                if self.undecoded.as_ref().is_some_and(|undecoded| {
                    undecoded.transition.block_height <= checkpoint.block_height
                }) {
                    self.undecoded = None;
                }
                Ok(SubscriptionEvent::Checkpoint {
                    block_height: checkpoint.block_height,
                })
            }
            Some(Responses::StateTransition(matched)) => self.accept_transition(matched),
            None => Err(node_fault("unknown response kind").into()),
        }
    }

    fn accept_transition(
        &mut self,
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
                let transition = UndecodedTransition {
                    block_height: matched.block_height,
                    state_transition_hash,
                };
                let cannot_decode = format!(
                    "cannot decode the state transition at height {} (protocol version {}): \
                     {cause}",
                    matched.block_height, matched.protocol_version
                );
                let undecodable =
                    |why| Refusal::Undecodable(Error::Protocol(ProtocolError::DecodingError(why)));
                let node = self.stream.as_ref().map(|stream| stream.address.clone());
                let times = match self.undecoded.take() {
                    // Two nodes agree on the bytes: they are the chain's, and it is this SDK
                    // that cannot read them.
                    Some(seen) if seen.transition == transition && seen.node != node => {
                        return undecodable(format!(
                            "{cannot_decode}; a second node delivered these bytes, so this SDK \
                             is likely older than the network: upgrade it"
                        ));
                    }
                    Some(seen) if seen.transition == transition => seen.times + 1,
                    _ => 1,
                };
                // One node's word does not tell a faulty node from an SDK too old, however
                // often it repeats it, so the error names both.
                if times >= MAX_UNDECODABLE_FROM_ONE_NODE {
                    return undecodable(format!(
                        "{cannot_decode}; one node delivered these bytes {times} times and no \
                         other node could be asked: that node is faulty, or this SDK is older \
                         than the network"
                    ));
                }
                self.undecoded = Some(Undecoded {
                    transition,
                    node,
                    times,
                });
                Refusal::AskAnotherNode(Error::Protocol(ProtocolError::DecodingError(
                    cannot_decode,
                )))
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

/// Decode `bytes` as the block's protocol version decoded them for execution, which from the
/// version that takes a raw transition to be exactly one encoded transition refuses bytes left
/// over after it. Values are never nested deeper than the default bound, even under a version
/// that set none: the version is the node's word, and a deeper tree could exhaust the stack.
fn decode(bytes: &[u8], protocol_version: u32) -> Result<StateTransition, String> {
    let platform_version = PlatformVersion::get(protocol_version)
        .map_err(|_| format!("protocol version {protocol_version} is unknown to this SDK"))?;
    let decoded = if platform_version
        .system_limits
        .max_document_value_depth
        .is_some()
    {
        decode_committed_state_transition(bytes, platform_version)
    } else {
        let mut bounded = platform_version.clone();
        bounded.system_limits.max_document_value_depth =
            u16::try_from(DEFAULT_MAX_VALUE_DECODE_DEPTH).ok();
        decode_committed_state_transition(bytes, &bounded)
    };
    decoded.map_err(|error| error.to_string())
}

/// Whether another stream may get past a stream or open failing with `status`.
fn is_transient(status: &Status) -> bool {
    status.can_retry() || lacks_heights(status)
}

/// Whether this node cannot serve the requested heights: it no longer keeps a block, has not
/// reached it yet, or cannot decode it. `OutOfRange`, a start behind every node's replay
/// window, ends the subscription instead.
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

    /// A node other than the one every reopened stream comes from.
    fn first_node() -> Address {
        "https://127.0.0.1:7".parse().expect("address")
    }

    /// Have the open stream of `subscription` come from `address`.
    fn served_by(subscription: &mut StateTransitionSubscription, address: Address) {
        subscription.stream.as_mut().expect("a stream").address = address;
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
                    possibly_accepted: false,
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
        subscription.stream = Some(OpenStream::new(first, node(), resume_from));
        (subscription, opens)
    }

    /// A subscription with two filters on a stream that has sent its opening checkpoint, and
    /// resumes from `resume_from`.
    fn opened(resume_from: u64) -> StateTransitionSubscription {
        let (mut subscription, _) = subscription(Some(resume_from), ending(vec![]), vec![]);
        subscription
            .stream
            .as_mut()
            .expect("a stream")
            .first_message = FirstMessage::Sent;
        subscription
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
            Err(Refusal::AskAnotherNode(error)) => Err(format!("another node: {error}")),
            Err(Refusal::Undecodable(error)) => Err(format!("undecodable: {error}")),
        }
    }

    #[test]
    fn should_refuse_values_nested_deeper_than_the_default_bound_without_blaming_the_node() {
        // Building and dropping a value 300 levels deep needs more stack than libtest's
        // default test thread has.
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(refuse_values_nested_deeper_than_the_default_bound)
            .expect("the test thread spawns")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
    }

    fn refuse_values_nested_deeper_than_the_default_bound() {
        let no_limit = (1..PlatformVersion::latest().protocol_version)
            .rev()
            .filter_map(|version| PlatformVersion::get(version).ok())
            .find(|version| version.system_limits.max_document_value_depth.is_none())
            .expect("an early protocol version sets no value depth limit")
            .protocol_version;
        for protocol_version in [PlatformVersion::latest().protocol_version, no_limit] {
            let mut subscription = opened(42);
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
            // The SDK's own bound refuses them, and no node is blamed for it: another is
            // asked until the same one has sent them three times.
            for refusal in ["another node", "another node", "undecodable"] {
                let error = accept(300).expect_err("too deep");
                assert!(
                    error.starts_with(refusal) && error.contains("exceeds maximum 256"),
                    "{error}"
                );
            }
        }
    }

    #[test]
    fn should_not_blame_the_node_for_a_protocol_version_this_sdk_does_not_know() {
        let mut subscription = opened(42);
        let unknown = PlatformVersion::latest().protocol_version + 1;
        let accept = |subscription: &mut StateTransitionSubscription| {
            outcome(subscription.accept(response(Responses::StateTransition(
                StateTransitionMatch {
                    protocol_version: unknown,
                    ..transition(transfer())
                },
            ))))
            .expect_err("unknown version")
        };
        // The version is the node's word: another node is asked before it is believed.
        served_by(&mut subscription, first_node());
        let first = accept(&mut subscription);
        assert!(first.starts_with("another node"), "{first}");
        served_by(&mut subscription, node());
        let second = accept(&mut subscription);
        assert!(
            second.starts_with("undecodable") && second.contains("upgrade"),
            "{second}"
        );
    }

    /// From the protocol version that takes a raw transition to be exactly one encoded
    /// transition, no block executes bytes with something left over after the transition.
    #[test]
    fn should_refuse_bytes_left_over_after_a_transition_where_the_protocol_does() {
        let mut padded = transfer();
        padded.push(0);
        let exact = PlatformVersion::latest().protocol_version;
        let ignores_leftover_bytes = (1..exact)
            .rev()
            .filter_map(|version| PlatformVersion::get(version).ok())
            .find(|version| {
                version
                    .drive_abci
                    .methods
                    .state_transition_processing
                    .decode_raw_state_transitions
                    == 0
            })
            .expect("an early protocol version ignores bytes left over")
            .protocol_version;
        let mut subscription = opened(42);
        let mut accept = |protocol_version| {
            outcome(subscription.accept(response(Responses::StateTransition(
                StateTransitionMatch {
                    protocol_version,
                    ..transition(padded.clone())
                },
            ))))
        };
        assert_eq!(accept(ignores_leftover_bytes), Ok(()));
        let error = accept(exact).expect_err("bytes left over");
        assert!(error.starts_with("another node"), "{error}");
    }

    /// The hash of bytes this SDK cannot decode is the node's word like the bytes, so one
    /// node sending them proves nothing about the others: the stream moves to another node,
    /// and the first is not blamed. When a second node delivers the same bytes, they are the
    /// chain's, and the subscription ends: this SDK cannot read them.
    #[tokio::test(start_paused = true)]
    async fn should_ask_another_node_before_ending_on_a_transition_it_cannot_decode() {
        let undecodable = || {
            let mut undecodable = transition(vec![0xff, 0x00, 0x13]);
            undecodable.block_height = 77;
            Ok(Responses::StateTransition(undecodable))
        };
        let watched = |subscription: &StateTransitionSubscription| {
            let mut addresses = subscription.sdk.address_list().clone();
            addresses.add(node());
            addresses.add(first_node());
            addresses
        };

        // The next node decodes the block: the subscription goes on.
        let first = silent(vec![checkpoint(41), undecodable()]);
        let second = silent(vec![
            checkpoint(41),
            Ok(Responses::StateTransition(transition(transfer()))),
            checkpoint(77),
        ]);
        let (mut recovered, opened) = subscription(Some(42), first, vec![second]);
        served_by(&mut recovered, first_node());
        let addresses = watched(&recovered);
        for _ in 0..2 {
            assert!(matches!(
                recovered.next().await,
                Ok(SubscriptionEvent::Checkpoint { block_height: 41 })
            ));
        }
        assert!(matches!(
            recovered.next().await,
            Ok(SubscriptionEvent::StateTransition(_))
        ));
        assert!(recovered.undecoded.is_some(), "until its block is passed");
        assert!(matches!(
            recovered.next().await,
            Ok(SubscriptionEvent::Checkpoint { block_height: 77 })
        ));
        assert!(recovered.undecoded.is_none());
        assert_eq!(opens(&opened), vec![Some(42)]);
        assert!(!addresses.is_banned(&first_node()) && !addresses.is_banned(&node()));

        // A second node sends the same bytes: this SDK cannot read the transition.
        let first = silent(vec![checkpoint(41), undecodable()]);
        let second = silent(vec![checkpoint(41), undecodable()]);
        let third = silent(vec![checkpoint(41)]);
        let (mut repeated, reopened) = subscription(Some(42), first, vec![second, third]);
        served_by(&mut repeated, first_node());
        let addresses = watched(&repeated);
        for _ in 0..2 {
            assert!(repeated.next().await.is_ok());
        }
        let error = repeated.next().await.expect_err("undecodable");
        assert!(
            error.to_string().contains("height 77") && error.to_string().contains("upgrade"),
            "{error}"
        );
        assert!(repeated.next().await.is_err());
        assert_eq!(opens(&reopened), vec![Some(42)], "no third node is tried");
        assert!(!addresses.is_banned(&first_node()) && !addresses.is_banned(&node()));
    }

    /// The same node sending the bytes again adds nothing to its word: the reopened stream
    /// can land on it when no other node is live. It is asked three times, and the error then
    /// says what one node's word leaves open, not that the SDK must be upgraded.
    #[tokio::test(start_paused = true)]
    async fn should_not_take_one_nodes_repeated_bytes_as_a_reason_to_upgrade() {
        let undecodable = || {
            let mut undecodable = transition(vec![0xff, 0x00, 0x13]);
            undecodable.block_height = 77;
            silent(vec![
                checkpoint(41),
                Ok(Responses::StateTransition(undecodable)),
            ])
        };
        let (mut subscription, opened) = subscription(
            Some(42),
            undecodable(),
            vec![undecodable(), undecodable(), silent(vec![checkpoint(41)])],
        );
        // Every stream comes from the one node, and each opens with its checkpoint.
        for _ in 0..3 {
            assert!(matches!(
                subscription.next().await,
                Ok(SubscriptionEvent::Checkpoint { block_height: 41 })
            ));
        }
        let error = subscription.next().await.expect_err("undecodable");
        let error = error.to_string();
        assert!(
            error.contains("3 times") && error.contains("faulty") && !error.contains("upgrade it"),
            "{error}"
        );
        assert_eq!(opens(&opened), vec![Some(42), Some(42)]);
    }

    /// A stream opens with a checkpoint, no higher than the height before the one it was
    /// opened at. A node that opens higher would move the place to resume from past blocks
    /// nobody scanned: it is at fault, and the place to resume from stays.
    #[tokio::test(start_paused = true)]
    async fn should_refuse_a_stream_that_opens_above_its_start_or_without_a_checkpoint() {
        for opening in [
            checkpoint(5_000_000),
            checkpoint(42),
            Ok(Responses::StateTransition(transition(transfer()))),
        ] {
            let first = silent(vec![opening, checkpoint(6_000_000)]);
            let second = silent(vec![checkpoint(41)]);
            let (mut subscription, opened) = subscription(Some(42), first, vec![second]);
            assert!(matches!(
                subscription.next().await,
                Ok(SubscriptionEvent::Checkpoint { block_height: 41 })
            ));
            assert_eq!(opens(&opened), vec![Some(42)]);
            assert_eq!(subscription.resume_height(), Some(42));
        }

        // A live start names no height: its first checkpoint says where it starts.
        let (mut live, _) = subscription(None, silent(vec![checkpoint(900)]), vec![]);
        assert!(matches!(
            live.next().await,
            Ok(SubscriptionEvent::Checkpoint { block_height: 900 })
        ));
        assert_eq!(live.resume_height(), Some(901));
        let unannounced = silent(vec![Ok(Responses::StateTransition(transition(transfer())))]);
        let (mut live, _) = subscription(None, unannounced, vec![]);
        assert!(live.next().await.is_err());
        assert_eq!(live.resume_height(), None);
    }

    /// A node whose chain has not reached the height it is asked to start at starts at its
    /// own tip and says so with its opening checkpoint. The place to resume from follows it
    /// down, which only delivers again. After the opening, a checkpoint never goes back.
    #[tokio::test(start_paused = true)]
    async fn should_follow_a_stream_that_opens_below_its_start_and_no_other_checkpoint_back() {
        let first = silent(vec![checkpoint(30), checkpoint(35), checkpoint(34)]);
        let second = silent(vec![checkpoint(35)]);
        let (mut subscription, opened) = subscription(Some(42), first, vec![second]);
        for expected in [30, 35] {
            assert!(matches!(
                subscription.next().await,
                Ok(SubscriptionEvent::Checkpoint { block_height }) if block_height == expected
            ));
            assert_eq!(subscription.resume_height(), Some(expected + 1));
        }
        // 34 after 35 is a node fault: the stream resumes from 36 on another node.
        assert!(matches!(
            subscription.next().await,
            Ok(SubscriptionEvent::Checkpoint { block_height: 35 })
        ));
        assert_eq!(opens(&opened), vec![Some(36)]);
    }

    /// A node refuses a start more than 1,000 blocks above its tip, so no node opens a stream
    /// lower than that below the height asked for. One that does is at fault: following it
    /// could put the place to resume from behind what any node replays, and end a
    /// subscription whose place was good. Nothing of it is published.
    #[tokio::test(start_paused = true)]
    async fn should_follow_an_opening_checkpoint_only_as_far_down_as_a_node_may_start() {
        let asked = 100_001;
        let lowest = asked - MAX_START_ABOVE_TIP;

        let (mut followed, _) = subscription(Some(asked), silent(vec![checkpoint(lowest)]), vec![]);
        assert!(matches!(
            followed.next().await,
            Ok(SubscriptionEvent::Checkpoint { block_height }) if block_height == lowest
        ));
        assert_eq!(followed.resume_height(), Some(lowest + 1));

        for below in [lowest - 1, 0] {
            let first = silent(vec![checkpoint(below), checkpoint(below + 1)]);
            let second = silent(vec![checkpoint(asked - 1)]);
            let (mut subscription, opened) = subscription(Some(asked), first, vec![second]);
            assert!(matches!(
                subscription.next().await,
                Ok(SubscriptionEvent::Checkpoint { block_height }) if block_height == asked - 1
            ));
            assert_eq!(opens(&opened), vec![Some(asked)], "opening at {below}");
            assert_eq!(subscription.resume_height(), Some(asked));
        }
    }

    /// A node names a height the chain has not reached in a checkpoint, and the stream then
    /// breaks. The next node is honest and 5 blocks behind that height: it starts at its own
    /// tip, so the block committed since, and its match, is not skipped.
    #[tokio::test(start_paused = true)]
    async fn should_not_skip_blocks_committed_after_a_checkpoint_ahead_of_the_chain() {
        let lying = ending(vec![
            checkpoint(41),
            checkpoint(1_000),
            Err(Status::unavailable("gone")),
        ]);
        let mut committed_since = transition(transfer());
        committed_since.block_height = 996;
        let honest = silent(vec![
            checkpoint(995),
            Ok(Responses::StateTransition(committed_since)),
            checkpoint(996),
        ]);
        let (mut subscription, opened) = subscription(Some(42), lying, vec![honest]);
        for expected in [41, 1_000, 995] {
            assert!(matches!(
                subscription.next().await,
                Ok(SubscriptionEvent::Checkpoint { block_height }) if block_height == expected
            ));
        }
        assert_eq!(opens(&opened), vec![Some(1_001)]);
        let Ok(SubscriptionEvent::StateTransition(event)) = subscription.next().await else {
            panic!("expected the transition committed since");
        };
        assert_eq!(event.block_height, 996);
        assert!(matches!(
            subscription.next().await,
            Ok(SubscriptionEvent::Checkpoint { block_height: 996 })
        ));
        assert_eq!(subscription.resume_height(), Some(997));
    }

    #[test]
    fn should_treat_batch_positions_out_of_range_as_node_faults() {
        let mut subscription = opened(42);
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
        let mut subscription = opened(42);
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

    /// A node that no longer keeps the heights asked for, or has not reached them, serves
    /// every other request. The next stream goes elsewhere, and the node is not banned,
    /// whatever the ban setting.
    #[tokio::test(start_paused = true)]
    async fn should_move_off_a_node_lacking_heights_without_banning_it() {
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
                    subscription.stream = Some(OpenStream::new(first, node(), Some(42)));
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
                assert!(!addresses.is_banned(&node()), "{case}");
                assert_ne!(addresses.get_live_address(), Some(node()), "{case}");
            }
        }
    }

    /// Heights pruned on every node, or a checkpoint that named a height the chain has not
    /// reached, would otherwise be retried without end, with no event and no error.
    #[tokio::test(start_paused = true)]
    async fn should_end_once_node_after_node_lacks_the_heights_asked_for() {
        let lacking = || Err(Status::failed_precondition("block 42 is not available"));
        let refusals = MAX_NODES_LACKING_HEIGHTS_IN_A_ROW as usize;

        let (mut subscription, opened) =
            unopened(Some(42), (0..refusals).map(|_| lacking()).collect());
        let error = subscription
            .next()
            .await
            .expect_err("no node holds the height");
        assert!(
            error.to_string().contains("block 42 is not available"),
            "{error}"
        );
        assert_eq!(opens(&opened), vec![Some(42); refusals]);

        // A stream that moves the place to resume from starts the count again.
        let mut streams: Vec<_> = (1..refusals).map(|_| lacking()).collect();
        streams.push(Ok(ending(vec![
            checkpoint(41),
            checkpoint(50),
            Err(Status::failed_precondition("block 51 is not available")),
        ])));
        streams.extend((2..refusals).map(|_| lacking()));
        streams.push(Ok(silent(vec![checkpoint(50)])));
        let (mut subscription, opened) = unopened(Some(42), streams);
        for expected in [41, 50, 50] {
            assert!(matches!(
                subscription.next().await,
                Ok(SubscriptionEvent::Checkpoint { block_height }) if block_height == expected
            ));
        }
        assert_eq!(opens(&opened).len(), 2 * refusals - 1);
    }

    /// A node admits a bounded number of streams. One that has no room for another answers
    /// `RESOURCE_EXHAUSTED` and still serves every other request, so opening a stream asks
    /// the client not to ban a busy node.
    #[test]
    fn should_not_ban_a_node_that_has_no_room_for_another_stream() {
        assert_eq!(
            SubscribeToStateTransitionsRequest::SETTINGS_OVERRIDES.ban_busy_address,
            Some(false)
        );
    }
}
