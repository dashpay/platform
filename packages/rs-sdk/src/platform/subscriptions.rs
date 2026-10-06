//! Subscriptions to committed state transitions.
//!
//! [`Sdk::subscribe_to_state_transitions`] opens a `subscribeToStateTransitions` stream and
//! returns a [`StateTransitionSubscription`] that yields every committed, successfully
//! executed transition matching any of the filters, block by block, plus checkpoints.
//!
//! The subscription resumes by itself: when a stream ends or fails in a way another attempt
//! can fix (the node restarted, timed the stream out, lags behind or cannot read a block), it
//! reopens the stream from the height after the last checkpoint, possibly on another node.
//! Delivery is therefore at least once: transitions between the last checkpoint and the break
//! are sent again. Persist [`SubscriptionEvent::Checkpoint`] heights to resume across restarts
//! with [`Sdk::subscribe_to_state_transitions`]'s `from_block_height`.
//!
//! # What a subscription proves
//!
//! Every transition a node sends is checked here: its hash, and that it matches the filters,
//! evaluated against data contracts fetched with proofs. A data contract update in the stream
//! is only a signal: the contract is read again with a proof before the filters on it rebind. A node that sends a transition the
//! filters do not match is skipped. What a node cannot be held to is completeness: it may
//! leave a matching transition out, and block heights and times are its word. To act on a
//! transition, read the state it changed with a proved query; to catch up on what happened
//! while offline, start from a proved read's height (`metadata.height + 1`).

use crate::platform::Fetch;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::subscribe_to_state_transitions_response_v0::Responses;
use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::Version;
use dapi_grpc::platform::v0::{SubscribeToStateTransitionsRequest, SubscribeToStateTransitionsResponse};
use dapi_grpc::tonic::{Code, Status, Streaming};
use dash_platform_queries::subscriptions::{
    canonical_operands, subscribe_request, ResolvedFilters, StateTransitionFilter, MAX_FILTERS,
};
use dpp::dashcore::hashes::{sha256, Hash};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::prelude::Identifier;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use rs_dapi_client::transport::{sleep, TransportError};
use rs_dapi_client::{Address, DapiClientError, DapiRequestExecutor, RequestSettings};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Consecutive failed attempts after which the subscription gives up.
const MAX_CONSECUTIVE_FAILURES: u32 = 5;
/// Wait before the first retry; doubles per consecutive failure.
const FIRST_RETRY_DELAY: Duration = Duration::from_secs(1);
/// A stream that sends nothing for a whole window of this length counts as stalled, so after
/// 60 to 120 seconds of silence. The node checkpoints every 10 seconds while idle; this leaves
/// room for slow block reads on its side.
const IDLE_WINDOW: Duration = Duration::from_secs(60);

/// A committed state transition that matched the subscription's filters.
#[derive(Debug, Clone)]
pub struct StateTransitionEvent {
    /// Height of the block the transition executed in.
    pub block_height: u64,
    /// Block time, in milliseconds since the Unix epoch.
    pub block_time_ms: u64,
    /// The protocol version the block executed under.
    pub protocol_version: u32,
    /// Position of the transition among the block's transactions.
    pub index_in_block: u32,
    /// SHA-256 of the serialized transition, its Tenderdash hash.
    pub hash: [u8; 32],
    /// The transition.
    pub state_transition: StateTransition,
    /// Indexes, in the subscription's filters, of the filters it matched.
    pub matched_filters: Vec<u32>,
    /// For a batch: positions, in its transitions, of the document and token transitions that
    /// matched.
    pub matched_batch_positions: Vec<u32>,
}

/// One item of a state transition subscription.
#[derive(Debug, Clone)]
pub enum SubscriptionEvent {
    /// A matching transition.
    StateTransition(Box<StateTransitionEvent>),
    /// Every match from the first scanned block through `block_height` has been delivered;
    /// resume from `block_height + 1`.
    Checkpoint {
        /// The highest fully scanned height.
        block_height: u64,
    },
}

/// A running subscription; see the [module documentation](self).
pub struct StateTransitionSubscription {
    sdk: Sdk,
    /// The caller's filters, followed by the internal contract-update filter, if any.
    filters: Vec<StateTransitionFilter>,
    /// How many of `filters` are the caller's.
    caller_filters: usize,
    resolved: ResolvedFilters,
    stream: Option<Streaming<SubscribeToStateTransitionsResponse>>,
    /// Messages received on the current stream.
    stream_messages: u32,
    /// The node serving the current stream.
    stream_address: Option<Address>,
    /// Where the next stream starts; `None` until the first checkpoint of a live-only start.
    resume_from: Option<u64>,
    consecutive_failures: u32,
    /// The wait before the next stream is opened, after a failure. Kept here, like `idle`, so
    /// a caller cancelling `next()` does not restart it.
    retry: Option<Timer>,
    /// Bound data contracts a delivered update changed, with the version it changed each to;
    /// re-read with proofs before the next message is checked.
    contracts_to_refresh: BTreeMap<Identifier, u32>,
    /// Proved reads of a refreshed contract that came back older than its update, in a row,
    /// and the wait before the next; kept here so a caller cancelling `next()` restarts neither.
    stale_reads: u32,
    refresh_wait: Option<Timer>,
    /// Why the subscription ended, once it has: later calls fail rather than read on.
    terminated: Option<String>,
    /// Notices the current stream going silent.
    idle: IdleWatch,
}

/// The deepest value nesting the SDK decodes, whatever the protocol version a node reports.
const MAX_VALUE_DEPTH: usize = dpp::platform_value::DEFAULT_MAX_VALUE_DECODE_DEPTH;

/// A pending wait, kept across reads.
type Timer = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Notices a stream that sent nothing for a whole `IDLE_WINDOW`, with one timer per
/// subscription however many messages arrive: a message is noted rather than restarting the
/// timer, since on wasm a timer that is dropped keeps running until it fires.
#[derive(Default)]
struct IdleWatch {
    /// The current window; kept across reads, so cancelling a read does not restart it.
    window: Option<Timer>,
    /// Whether a message arrived (or a stream opened) during the current window.
    heard: bool,
}

impl IdleWatch {
    /// Note a message, or a new stream, which starts out not silent.
    fn heard(&mut self) {
        self.heard = true;
    }

    /// Completes once a whole window passes in which nothing was heard.
    async fn silent(&mut self) {
        loop {
            self.window
                .get_or_insert_with(|| Box::pin(sleep(IDLE_WINDOW)))
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
    /// Subscribe to committed state transitions matching any of `filters`, scanning from
    /// `from_block_height` (inclusive), or from after the current tip when `None`.
    ///
    /// The data contracts the document filters name are fetched with proofs first, to check
    /// what the node sends; a contract that does not exist fails the call. The subscription
    /// also follows updates of those contracts, which the node applies to its matching, so both
    /// sides match against the same contract version; those updates are not reported unless
    /// `filters` asks for them. Following takes one of the request's filters, so with document
    /// filters at most `MAX_FILTERS - 1` filters may be given.
    ///
    /// Without a `from_block_height`, a subscription with document filters starts just after
    /// the height its contracts were proved at, usually a block or two before the tip, so that
    /// an update of one committed while subscribing is still followed.
    pub async fn subscribe_to_state_transitions(
        &self,
        mut filters: Vec<StateTransitionFilter>,
        from_block_height: Option<u64>,
    ) -> Result<StateTransitionSubscription, Error> {
        let caller_filters = filters.len();
        let data_contract_ids = ResolvedFilters::data_contract_ids(&filters);
        // Bound the request before fetching anything for it.
        let max_filters = if data_contract_ids.is_empty() {
            MAX_FILTERS
        } else {
            // One filter is needed to follow updates of the data contracts.
            MAX_FILTERS - 1
        };
        if filters.is_empty() || filters.len() > max_filters {
            return Err(Error::Config(format!(
                "a subscription takes 1 to {max_filters} filters here, got {}",
                filters.len()
            )));
        }
        let mut contracts = BTreeMap::new();
        // The lowest height the contracts were proved at: the bindings cover every update up
        // to it, and the stream must carry those after it.
        let mut bound_at: Option<u64> = None;
        for data_contract_id in &data_contract_ids {
            let (contract, metadata) =
                DataContract::fetch_with_metadata(self, *data_contract_id, None).await?;
            if let Some(contract) = contract {
                contracts.insert(*data_contract_id, Arc::new(contract));
            }
            bound_at = Some(bound_at.map_or(metadata.height, |height| height.min(metadata.height)));
        }
        let from_block_height =
            from_block_height.or_else(|| bound_at.map(|height| height.saturating_add(1)));
        if !data_contract_ids.is_empty() {
            filters.push(StateTransitionFilter::DataContracts {
                data_contract_ids: data_contract_ids.into_iter().collect(),
            });
        }
        // Send operands in the form the schema stores them, so the node reads what was meant.
        let filters = filters
            .iter()
            .map(|filter| {
                canonical_operands(filter, |id| contracts.get(id).cloned(), self.version())
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::Config)?;
        let resolved = ResolvedFilters::resolve(
            filters.clone(),
            |id| contracts.get(id).cloned(),
            self.version(),
        )
        .map_err(|e| Error::Config(e.to_string()))?;

        let mut subscription = StateTransitionSubscription {
            sdk: self.clone(),
            filters,
            caller_filters,
            resolved,
            stream: None,
            stream_messages: 0,
            stream_address: None,
            resume_from: from_block_height,
            consecutive_failures: 0,
            retry: None,
            contracts_to_refresh: BTreeMap::new(),
            stale_reads: 0,
            refresh_wait: None,
            terminated: None,
            idle: IdleWatch::default(),
        };
        // Open the first stream now, so a refused request fails here rather than on `next`.
        subscription.stream = Some(subscription.open(false).await?);
        Ok(subscription)
    }
}

impl StateTransitionSubscription {
    /// The next matching transition or checkpoint. Resumes broken streams by itself; an error
    /// means the subscription cannot continue (the request was refused, a response could not
    /// be interpreted, or too many attempts in a row failed), and every later call returns an
    /// error too. Only a failed proved re-read of an updated contract may be retried.
    pub async fn next(&mut self) -> Result<SubscriptionEvent, Error> {
        if let Some(reason) = &self.terminated {
            return Err(Error::Generic(format!(
                "the subscription has ended: {reason}"
            )));
        }
        loop {
            self.refresh_contracts().await?;
            match self.step().await {
                Ok(Some(event)) => return Ok(event),
                Ok(None) => continue,
                Err(error) => {
                    self.terminated = Some(error.to_string());
                    self.stream = None;
                    return Err(error);
                }
            }
        }
    }

    /// Read one message, reopening the stream first if it broke. `None` when the message is
    /// skipped or the stream broke and will be reopened.
    async fn step(&mut self) -> Result<Option<SubscriptionEvent>, Error> {
        let stream = match self.stream.as_mut() {
            Some(stream) => stream,
            None => {
                let stream = self.reopen().await?;
                self.stream_messages = 0;
                self.idle.heard();
                self.stream.insert(stream)
            }
        };
        // The node checkpoints at least every few seconds; a stream quiet for much longer is
        // stalled, even if its connection still answers, and is resumed elsewhere.
        let message = {
            let message = stream.message();
            futures::pin_mut!(message);
            let silent = self.idle.silent();
            futures::pin_mut!(silent);
            match futures::future::select(message, silent).await {
                futures::future::Either::Left((message, _)) => message,
                futures::future::Either::Right(_) => Err(Status::unavailable(format!(
                    "no message for over {}s",
                    IDLE_WINDOW.as_secs()
                ))),
            }
        };
        let failure = match message {
            Ok(Some(response)) => {
                self.idle.heard();
                self.stream_messages += 1;
                // A stream that got past its opening checkpoint was healthy; a node that
                // fails right after every start is not retried forever.
                if self.stream_messages > 1 {
                    self.consecutive_failures = 0;
                }
                return self.accept(response);
            }
            // The node or a proxy ended the stream (its lifetime elapsed): continue.
            Ok(None) => None,
            Err(status) if resumable(&status) => Some(status),
            Err(status) => return Err(status_error(status)),
        };
        if let Some(status) = &failure {
            tracing::debug!(code = ?status.code(), message = status.message(), "state transition stream broke; resuming");
            // Resume elsewhere when the node itself is at fault, rather than reconnecting
            // to it; a stream ended by its lifetime or the client says nothing of the node.
            if node_at_fault(status) {
                if let Some(address) = &self.stream_address {
                    self.sdk.address_list().ban_with_reason(
                        address,
                        Some(format!(
                            "state transition stream failed: {}",
                            status.message()
                        )),
                    );
                }
            }
        }
        self.stream = None;
        self.consecutive_failures += 1;
        self.back_off();
        if self.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
            return Err(match failure {
                Some(status) => status_error(status),
                None => Error::Generic(
                    "state transition streams keep ending right after they start".to_string(),
                ),
            });
        }
        Ok(None)
    }

    /// The height the subscription resumes from if its stream breaks now.
    pub fn resume_height(&self) -> Option<u64> {
        self.resume_from
    }

    /// Consume the subscription as a stream of events.
    pub fn into_stream(self) -> impl futures::Stream<Item = Result<SubscriptionEvent, Error>> {
        futures::stream::unfold(Some(self), |subscription| async move {
            let mut subscription = subscription?;
            match subscription.next().await {
                Ok(event) => Some((Ok(event), Some(subscription))),
                Err(error) => Some((Err(error), None)),
            }
        })
    }

    /// Open a stream from the resume height. `reopening` is false only for the subscription's
    /// first stream.
    async fn open(
        &mut self,
        reopening: bool,
    ) -> Result<Streaming<SubscribeToStateTransitionsResponse>, Error> {
        let request: SubscribeToStateTransitionsRequest =
            subscribe_request(&self.filters, self.resume_from)?;
        match self.sdk.execute(request, RequestSettings::default()).await {
            Ok(response) => {
                self.stream_address = Some(response.address);
                Ok(response.inner)
            }
            Err(error) => {
                // A node refusing to reopen this subscription for its own reasons (lagging
                // too far behind the resume height, missing history) is moved off of, like one
                // failing an open stream; the first open's refusal is the caller's to handle.
                if reopening {
                    if let (
                        DapiClientError::Transport(TransportError::Grpc(status)),
                        Some(address),
                    ) = (&error.inner, &error.address)
                    {
                        if node_at_fault(status) {
                            self.sdk.address_list().ban_with_reason(
                                address,
                                Some(format!(
                                    "refused to resume a state transition subscription: {}",
                                    status.message()
                                )),
                            );
                        }
                    }
                }
                Err(Error::from(error.inner))
            }
        }
    }

    /// Wait before the next attempt, doubling per consecutive failure.
    fn back_off(&mut self) {
        let delay = FIRST_RETRY_DELAY * 2u32.pow(self.consecutive_failures.saturating_sub(1));
        self.retry = Some(Box::pin(sleep(delay)));
    }

    /// Open a new stream after a break, once the wait after the last failure has passed.
    async fn reopen(&mut self) -> Result<Streaming<SubscribeToStateTransitionsResponse>, Error> {
        loop {
            if let Some(retry) = self.retry.as_mut() {
                retry.await;
                self.retry = None;
            }
            match self.open(true).await {
                Ok(stream) => return Ok(stream),
                Err(error) => {
                    self.consecutive_failures += 1;
                    self.back_off();
                    if self.consecutive_failures >= MAX_CONSECUTIVE_FAILURES
                        || !error_is_resumable(&error)
                    {
                        return Err(error);
                    }
                    tracing::debug!(%error, "cannot reopen the state transition stream; retrying");
                }
            }
        }
    }

    /// Rebind the filters on contracts a delivered update changed to the updated version or a
    /// later one, read with a proof. A node answering with an older version is behind the
    /// stream; the read is retried, and fails (to be retried by the caller) if it stays behind.
    /// A contract that can no longer be read ends the subscription.
    async fn refresh_contracts(&mut self) -> Result<(), Error> {
        // An id leaves the queue only once rebound, so a cancelled or failed refresh is retried.
        while let Some((&data_contract_id, &updated_version)) =
            self.contracts_to_refresh.first_key_value()
        {
            let bound_version = self
                .resolved
                .data_contract(data_contract_id)
                .map(|contract| contract.version());
            if bound_version >= Some(updated_version) {
                self.contracts_to_refresh.remove(&data_contract_id);
                continue;
            }
            if let Some(wait) = self.refresh_wait.as_mut() {
                wait.await;
                self.refresh_wait = None;
            }
            let contract = DataContract::fetch(&self.sdk, data_contract_id)
                .await?
                .ok_or_else(|| {
                    Error::MissingDependency(
                        "DataContract".to_string(),
                        format!("data contract {data_contract_id} not found after its update"),
                    )
                })?;
            if contract.version() < updated_version {
                // Never bind a version older than the update the stream has passed: matching
                // under the old schema could drop events the node matched under the new one.
                self.stale_reads += 1;
                self.refresh_wait = Some(Box::pin(sleep(FIRST_RETRY_DELAY)));
                if self.stale_reads >= MAX_CONSECUTIVE_FAILURES {
                    self.stale_reads = 0;
                    return Err(Error::Generic(format!(
                        "data contract {data_contract_id} read with a proof is still at version \
                         {}, before the update to version {updated_version} the stream passed; \
                         try again",
                        contract.version()
                    )));
                }
                continue;
            }
            self.stale_reads = 0;
            self.resolved
                .rebind_data_contract(Arc::new(contract), self.sdk.version());
            self.contracts_to_refresh.remove(&data_contract_id);
        }
        Ok(())
    }

    /// The event a response carries, after checking it; `None` for a response to skip. A
    /// transition the node should not have sent (wrong hash, not matching) is skipped; one this
    /// SDK cannot judge (a protocol version or transition it does not know) is an error, since
    /// guessing could report or drop it wrongly: upgrade the SDK.
    fn accept(
        &mut self,
        response: SubscribeToStateTransitionsResponse,
    ) -> Result<Option<SubscriptionEvent>, Error> {
        let unsupported = || {
            Error::Generic(
                "the node sent a subscription response this SDK does not understand; upgrade \
                 the SDK"
                    .to_string(),
            )
        };
        let Some(Version::V0(response)) = response.version else {
            return Err(unsupported());
        };
        let Some(responses) = response.responses else {
            return Err(unsupported());
        };
        Ok(match responses {
            Responses::Checkpoint(checkpoint) => {
                let resume_from = checkpoint.block_height.checked_add(1).ok_or_else(|| {
                    Error::Generic(format!(
                        "the node sent a checkpoint at height {}, after which no stream can resume",
                        checkpoint.block_height
                    ))
                })?;
                self.resume_from = Some(resume_from);
                Some(SubscriptionEvent::Checkpoint {
                    block_height: checkpoint.block_height,
                })
            }
            Responses::StateTransition(matched) => {
                let hash: [u8; 32] = sha256::Hash::hash(&matched.state_transition).to_byte_array();
                if hash.as_slice() != matched.state_transition_hash.as_slice() {
                    tracing::warn!(
                        height = matched.block_height,
                        "skipping a state transition whose hash does not match its bytes"
                    );
                    return Ok(None);
                }
                let platform_version = PlatformVersion::get(matched.protocol_version)
                    .map_err(|e| Error::Protocol(e.into()))?;
                // Decoded under the rules of the block's version, as consensus decoded it, except
                // that the version is the node's word: values are never nested deeper than the
                // current limit, even under an early version that set none, since such a tree
                // could exhaust the stack when it is dropped.
                let state_transition =
                    StateTransition::deserialize_from_bytes_untrusted_in_version_with_max_value_depth(
                        &matched.state_transition,
                        platform_version,
                        MAX_VALUE_DEPTH,
                    )?;
                let local_match = self.resolved.matches(&state_transition, platform_version);
                if let Some((data_contract_id, updated_version)) =
                    self.resolved.followed_data_contract(&state_transition)
                {
                    // Never install the contract the node sent: re-read it with a proof.
                    let pending = self
                        .contracts_to_refresh
                        .entry(data_contract_id)
                        .or_insert(updated_version);
                    *pending = (*pending).max(updated_version);
                }
                let Some(mut local_match) = local_match else {
                    tracing::warn!(
                        height = matched.block_height,
                        "skipping a state transition the node sent that does not match the filters"
                    );
                    return Ok(None);
                };
                // Contract updates the subscription follows only for itself are not reported.
                let caller_filters = self.caller_filters as u32;
                local_match
                    .matched_filters
                    .retain(|index| *index < caller_filters);
                if local_match.matched_filters.is_empty() {
                    return Ok(None);
                }
                Some(SubscriptionEvent::StateTransition(Box::new(
                    StateTransitionEvent {
                        block_height: matched.block_height,
                        block_time_ms: matched.block_time_ms,
                        protocol_version: matched.protocol_version,
                        index_in_block: matched.index_in_block,
                        hash,
                        state_transition,
                        matched_filters: local_match.matched_filters,
                        matched_batch_positions: local_match.matched_batch_positions,
                    },
                )))
            }
        })
    }
}

/// Whether a stream failure says the serving node cannot serve this subscription (it lacks
/// history, cannot read or decode a block, or Tenderdash is unreachable), so another node
/// should be tried; not an ended stream lifetime, a cancellation or a slow client.
fn node_at_fault(status: &Status) -> bool {
    matches!(
        status.code(),
        Code::OutOfRange
            | Code::FailedPrecondition
            | Code::Unavailable
            | Code::Internal
            | Code::Unknown
    )
}

/// Whether a stream that failed with `status` can continue on a new stream.
fn resumable(status: &Status) -> bool {
    matches!(
        status.code(),
        Code::Unavailable
            | Code::ResourceExhausted
            | Code::DeadlineExceeded
            | Code::Aborted
            | Code::Cancelled
            | Code::Internal
            | Code::Unknown
            | Code::FailedPrecondition
            | Code::OutOfRange
    )
}

fn error_is_resumable(error: &Error) -> bool {
    match error {
        Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(status))) => {
            resumable(status)
        }
        Error::DapiClientError(DapiClientError::NoAvailableAddressesToRetry(_)) => true,
        _ => false,
    }
}

fn status_error(status: Status) -> Error {
    Error::from(DapiClientError::Transport(TransportError::Grpc(status)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::subscribe_to_state_transitions_response_v0::{
        Checkpoint, StateTransitionMatch,
    };
    use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::SubscribeToStateTransitionsResponseV0;
    use dash_platform_queries::subscriptions::Role;
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use tokio::time::timeout;

    /// A subscription to transfers received by identity 7, plus an internal filter on identity
    /// 9 standing in for the contract-update filter the SDK adds for itself.
    fn subscription() -> StateTransitionSubscription {
        let identities = |id: u8| StateTransitionFilter::Identities {
            identity_ids: vec![Identifier::from([id; 32])],
            role: Role::Recipient,
        };
        let filters = vec![identities(7), identities(9)];
        let resolved =
            ResolvedFilters::resolve(filters.clone(), |_| None, PlatformVersion::latest())
                .expect("filters resolve");
        StateTransitionSubscription {
            sdk: Sdk::new_mock(),
            filters,
            caller_filters: 1,
            resolved,
            stream: None,
            stream_messages: 0,
            stream_address: None,
            resume_from: None,
            consecutive_failures: 0,
            retry: None,
            contracts_to_refresh: BTreeMap::new(),
            stale_reads: 0,
            refresh_wait: None,
            terminated: None,
            idle: IdleWatch::default(),
        }
    }

    fn transfer_to(recipient: u8) -> Vec<u8> {
        StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
            IdentityCreditTransferTransitionV0 {
                identity_id: Identifier::from([1u8; 32]),
                recipient_id: Identifier::from([recipient; 32]),
                amount: 10,
                ..Default::default()
            },
        ))
        .serialize_to_bytes()
        .expect("serializable")
    }

    fn response(message: Responses) -> SubscribeToStateTransitionsResponse {
        SubscribeToStateTransitionsResponse {
            version: Some(Version::V0(SubscribeToStateTransitionsResponseV0 {
                responses: Some(message),
            })),
        }
    }

    fn matched(bytes: Vec<u8>) -> StateTransitionMatch {
        StateTransitionMatch {
            block_height: 5,
            block_time_ms: 1,
            protocol_version: PlatformVersion::latest().protocol_version,
            index_in_block: 0,
            state_transition_hash: sha256::Hash::hash(&bytes).to_byte_array().to_vec(),
            state_transition: bytes,
            matched_filters: vec![0],
            matched_batch_positions: vec![],
        }
    }

    #[test]
    fn should_deliver_a_matching_transition_with_caller_filter_indexes() {
        let mut subscription = subscription();
        let event = subscription
            .accept(response(Responses::StateTransition(matched(transfer_to(
                7,
            )))))
            .expect("accepted");
        let Some(SubscriptionEvent::StateTransition(event)) = event else {
            panic!("expected a transition event");
        };
        assert_eq!(event.block_height, 5);
        assert_eq!(event.matched_filters, vec![0]);
    }

    #[test]
    fn should_skip_transitions_with_a_wrong_hash_or_not_matching_the_filters() {
        let mut subscription = subscription();
        let mut wrong_hash = matched(transfer_to(7));
        wrong_hash.state_transition_hash = vec![0; 32];
        assert!(subscription
            .accept(response(Responses::StateTransition(wrong_hash)))
            .expect("skipped, not failed")
            .is_none());
        assert!(subscription
            .accept(response(Responses::StateTransition(matched(transfer_to(
                8
            )))))
            .expect("skipped, not failed")
            .is_none());
    }

    #[test]
    fn should_not_report_matches_of_the_internal_contract_update_filter() {
        let mut subscription = subscription();
        assert!(subscription
            .accept(response(Responses::StateTransition(matched(transfer_to(
                9
            )))))
            .expect("skipped, not failed")
            .is_none());
    }

    #[test]
    fn should_fail_on_a_protocol_version_or_transition_this_sdk_does_not_know() {
        let mut subscription = subscription();
        let mut unknown_version = matched(transfer_to(7));
        unknown_version.protocol_version = u32::MAX;
        assert!(subscription
            .accept(response(Responses::StateTransition(unknown_version)))
            .is_err());
        assert!(subscription
            .accept(response(Responses::StateTransition(matched(vec![
                0xff, 0, 0x13
            ]))))
            .is_err());
    }

    /// A batch creating a `niceDocument` of `data_contract_id` whose data nests `depth` arrays,
    /// encoded without building the nested value: a fabricated response deeper than the stack
    /// could drop.
    fn nested_document_batch(data_contract_id: Identifier, depth: usize) -> Vec<u8> {
        use dpp::platform_value::Value;
        use dpp::state_transition::batch_transition::batched_transition::document_create_transition::v0::DocumentCreateTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransition;
        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};

        let encode = |depth: usize| {
            let nested = (0..depth).fold(Value::Null, |value, _| Value::Array(vec![value]));
            StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
                owner_id: Identifier::from([1u8; 32]),
                transitions: vec![BatchedTransition::Document(DocumentTransition::Create(
                    DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                        base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                            id: Identifier::from([2u8; 32]),
                            identity_contract_nonce: 1,
                            document_type_name: "niceDocument".to_string(),
                            data_contract_id,
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
        };
        // One more level of nesting inserts the same bytes where the innermost value starts.
        let (shallow, deeper) = (encode(2), encode(3));
        let at = shallow
            .iter()
            .zip(&deeper)
            .take_while(|(a, b)| a == b)
            .count();
        let level = &deeper[at..at + deeper.len() - shallow.len()];
        let mut bytes = shallow[..at].to_vec();
        for _ in 2..depth {
            bytes.extend_from_slice(level);
        }
        bytes.extend_from_slice(&shallow[at..]);
        if depth == 3 {
            assert_eq!(
                bytes, deeper,
                "the nesting level is spliced where it belongs"
            );
        }
        bytes
    }

    #[test]
    fn should_refuse_values_nested_too_deep_whatever_version_the_node_reports() {
        use dash_platform_queries::subscriptions::DocumentFilter;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::tests::fixtures::get_data_contract_fixture;

        // An early version the node may claim, under which consensus set no depth limit.
        let unlimited = (1..=PlatformVersion::latest().protocol_version)
            .rev()
            .filter_map(|version| PlatformVersion::get(version).ok())
            .find(|version| version.system_limits.max_document_value_depth.is_none())
            .expect("an early protocol version sets no value depth limit");
        let contract = Arc::new(
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
                .data_contract_owned(),
        );
        let documents = vec![StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id()).with_document_type("niceDocument"),
        )];
        let matching = || StateTransitionSubscription {
            resolved: ResolvedFilters::resolve(
                documents.clone(),
                |_| Some(contract.clone()),
                PlatformVersion::latest(),
            )
            .expect("filters resolve"),
            filters: documents.clone(),
            ..subscription()
        };
        let claimed = |bytes: Vec<u8>| {
            let mut matched = matched(bytes);
            matched.protocol_version = unlimited.protocol_version;
            response(Responses::StateTransition(matched))
        };

        // Shallow nesting decodes under that version: reported by a matching subscription,
        // skipped by one that does not match it.
        let shallow = nested_document_batch(contract.id(), 16);
        assert!(matches!(
            matching().accept(claimed(shallow.clone())),
            Ok(Some(SubscriptionEvent::StateTransition(_)))
        ));
        assert!(matches!(subscription().accept(claimed(shallow)), Ok(None)));

        // Nesting far past what the stack could drop is refused while decoding, by matching and
        // non-matching subscriptions alike, rather than built and then disposed of.
        let deep = nested_document_batch(contract.id(), 40_000);
        assert!(deep.len() <= 100_000, "within the transition size limit");
        for mut subscription in [matching(), subscription()] {
            let error = subscription
                .accept(claimed(deep.clone()))
                .expect_err("too deep to decode");
            assert!(error.to_string().contains("exceeds maximum 256"), "{error}");
        }
    }

    #[test]
    fn should_resume_after_the_last_checkpoint() {
        let mut subscription = subscription();
        subscription
            .accept(response(Responses::Checkpoint(Checkpoint {
                block_height: 41,
            })))
            .expect("accepted");
        assert_eq!(subscription.resume_height(), Some(42));
    }

    #[test]
    fn should_resume_after_transient_failures_but_not_after_refusals() {
        for code in [
            Code::Unavailable,
            Code::ResourceExhausted,
            Code::FailedPrecondition,
            Code::OutOfRange,
        ] {
            assert!(resumable(&Status::new(code, "")), "{code:?}");
        }
        for code in [Code::InvalidArgument, Code::NotFound, Code::Unimplemented] {
            assert!(!resumable(&Status::new(code, "")), "{code:?}");
        }
    }

    #[test]
    fn should_fail_on_a_response_envelope_this_sdk_does_not_know() {
        let mut subscription = subscription();
        assert!(subscription
            .accept(SubscribeToStateTransitionsResponse { version: None })
            .is_err());
        assert!(subscription
            .accept(SubscribeToStateTransitionsResponse {
                version: Some(Version::V0(SubscribeToStateTransitionsResponseV0 {
                    responses: None,
                })),
            })
            .is_err());
    }

    #[test]
    fn should_not_install_a_contract_a_node_sends_but_refresh_it_with_a_proof() {
        use dash_platform_queries::subscriptions::DocumentFilter;
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
        use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use dpp::version::TryFromPlatformVersioned;

        let proved = Arc::new(
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
                .data_contract_owned(),
        );
        let filters = vec![
            StateTransitionFilter::Documents(
                DocumentFilter::new(proved.id()).with_document_type("niceDocument"),
            ),
            StateTransitionFilter::DataContracts {
                data_contract_ids: vec![proved.id()],
            },
        ];
        let resolved = ResolvedFilters::resolve(
            filters.clone(),
            |_| Some(proved.clone()),
            PlatformVersion::latest(),
        )
        .expect("filters resolve");
        let mut subscription = StateTransitionSubscription {
            filters,
            caller_filters: 1,
            resolved,
            ..subscription()
        };

        // A node fabricates an update of the watched contract.
        let mut fabricated = (*proved).clone();
        fabricated.set_version(proved.version() + 7);
        let update = StateTransition::DataContractUpdate(
            DataContractUpdateTransition::try_from_platform_versioned(
                (fabricated, 1),
                PlatformVersion::latest(),
            )
            .expect("update transition"),
        )
        .serialize_to_bytes()
        .expect("serializable");

        let event = subscription
            .accept(response(Responses::StateTransition(matched(update))))
            .expect("accepted");
        // The internal follow filter's match is not reported...
        assert!(event.is_none());
        // ...the proved binding is untouched...
        assert_eq!(
            subscription
                .resolved
                .data_contract(proved.id())
                .expect("bound")
                .version(),
            proved.version()
        );
        // ...and the contract is queued to be read again with a proof.
        assert_eq!(
            subscription.contracts_to_refresh.get(&proved.id()),
            Some(&(proved.version() + 7))
        );
    }

    #[test]
    fn should_reject_a_checkpoint_no_stream_can_resume_after() {
        let mut subscription = subscription();
        subscription
            .accept(response(Responses::Checkpoint(Checkpoint {
                block_height: 41,
            })))
            .expect("accepted");
        assert!(subscription
            .accept(response(Responses::Checkpoint(Checkpoint {
                block_height: u64::MAX,
            })))
            .is_err());
        assert_eq!(subscription.resume_height(), Some(42));
    }

    #[tokio::test]
    async fn should_keep_a_contract_queued_until_its_refresh_completes() {
        let mut subscription = subscription();
        let data_contract_id = Identifier::from([5u8; 32]);
        subscription
            .contracts_to_refresh
            .insert(data_contract_id, 2);
        // The mock SDK has no expectation for this fetch, so the refresh fails part-way, as it
        // would if the future were dropped while fetching.
        assert!(subscription.refresh_contracts().await.is_err());
        assert!(subscription
            .contracts_to_refresh
            .contains_key(&data_contract_id));
    }

    /// A subscription with a document filter bound to version 1 of a contract, and the
    /// contract at versions 2 and 3.
    fn subscription_on_contract() -> (StateTransitionSubscription, [Arc<DataContract>; 2]) {
        use dash_platform_queries::subscriptions::DocumentFilter;
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
        use dpp::tests::fixtures::get_data_contract_fixture;

        let fixture =
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
                .data_contract_owned();
        let version = |version: u32| {
            let mut contract = fixture.clone();
            contract.set_version(version);
            Arc::new(contract)
        };
        let bound = version(1);
        let filters = vec![StateTransitionFilter::Documents(
            DocumentFilter::new(bound.id()).with_document_type("niceDocument"),
        )];
        let subscription = StateTransitionSubscription {
            resolved: ResolvedFilters::resolve(
                filters.clone(),
                |_| Some(bound.clone()),
                PlatformVersion::latest(),
            )
            .expect("filters resolve"),
            filters,
            ..subscription()
        };
        (subscription, [version(2), version(3)])
    }

    #[tokio::test(start_paused = true)]
    async fn should_keep_the_stale_refresh_pacing_and_budget_across_cancelled_calls() {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        let (mut subscription, [lagging, _]) = subscription_on_contract();
        let id = lagging.id();
        subscription.contracts_to_refresh.insert(id, 3);
        subscription
            .sdk
            .mock()
            .expect_fetch(id, Some((*lagging).clone()))
            .await
            .expect("expectation");
        // A caller cancelling every 400ms still waits a second between stale reads, and still
        // gets the error after five of them.
        let started = tokio::time::Instant::now();
        let mut calls = 0;
        let error = loop {
            calls += 1;
            assert!(calls <= 20, "the stale-read budget was never exhausted");
            if let Ok(outcome) =
                timeout(Duration::from_millis(400), subscription.refresh_contracts()).await
            {
                break outcome.expect_err("still stale");
            }
        };
        assert!(error.to_string().contains("try again"), "{error}");
        assert!(started.elapsed() >= FIRST_RETRY_DELAY * (MAX_CONSECUTIVE_FAILURES - 1));
        assert_eq!(subscription.contracts_to_refresh.get(&id), Some(&3));
    }

    fn bound_version(subscription: &StateTransitionSubscription, id: Identifier) -> u32 {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        subscription
            .resolved
            .data_contract(id)
            .expect("bound")
            .version()
    }

    #[tokio::test(start_paused = true)]
    async fn should_not_bind_a_proved_contract_older_than_the_update_the_stream_passed() {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        let (mut subscription, [lagging, updated]) = subscription_on_contract();
        let id = updated.id();
        // The stream passed the update to version 3, but the proved read comes from a node
        // still at version 2.
        subscription.contracts_to_refresh.insert(id, 3);
        subscription
            .sdk
            .mock()
            .expect_fetch(id, Some((*lagging).clone()))
            .await
            .expect("expectation");
        assert!(subscription.refresh_contracts().await.is_err());
        assert_eq!(
            bound_version(&subscription, id),
            1,
            "not rebound to the lagging read"
        );
        assert_eq!(subscription.contracts_to_refresh.get(&id), Some(&3));

        // Once a node has caught up, the refresh completes.
        subscription
            .sdk
            .mock()
            .remove_fetch_expectation::<DataContract, _>(id)
            .await;
        subscription
            .sdk
            .mock()
            .expect_fetch(id, Some((*updated).clone()))
            .await
            .expect("expectation");
        subscription.refresh_contracts().await.expect("refreshed");
        assert_eq!(bound_version(&subscription, id), 3);
        assert!(subscription.contracts_to_refresh.is_empty());

        // An update the binding already covers needs no read (the mock now expects none).
        subscription
            .sdk
            .mock()
            .remove_fetch_expectation::<DataContract, _>(id)
            .await;
        subscription.contracts_to_refresh.insert(id, 2);
        subscription
            .refresh_contracts()
            .await
            .expect("nothing to read");
        assert_eq!(bound_version(&subscription, id), 3, "not regressed");
        assert!(subscription.contracts_to_refresh.is_empty());
    }

    #[test]
    fn should_skip_rather_than_panic_on_a_fabricated_wrong_typed_unicode_value() {
        use dash_platform_queries::subscriptions::{DocumentAction, DocumentActionMatch, DocumentFilter};
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::DataContractFactory;
        use dpp::platform_value::{platform_value, Value};
        use dpp::state_transition::batch_transition::batched_transition::document_create_transition::v0::DocumentCreateTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransition;
        use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
        use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
        use dpp::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
        use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
        use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
        use drive::query::{WhereClause, WhereOperator};

        let documents = platform_value!({
            "rating": {
                "type": "object",
                "properties": {
                    "stars": { "type": "integer", "minimum": 0, "maximum": 255, "position": 0 }
                },
                "additionalProperties": false
            }
        });
        let contract = Arc::new(
            DataContractFactory::new(PlatformVersion::latest().protocol_version)
                .expect("factory")
                .create_with_value_config(Identifier::from([1u8; 32]), 1, documents, None, None)
                .expect("the contract parses")
                .data_contract_owned(),
        );
        let filters = vec![StateTransitionFilter::Documents(
            DocumentFilter::new(contract.id())
                .with_document_type("rating")
                .with_action(
                    DocumentActionMatch::new(DocumentAction::Create).with_new_document_where(
                        WhereClause {
                            field: "stars".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::U64(3),
                        },
                    ),
                ),
        )];
        let resolved = ResolvedFilters::resolve(
            filters.clone(),
            |_| Some(contract.clone()),
            PlatformVersion::latest(),
        )
        .expect("filters resolve");
        let mut subscription = StateTransitionSubscription {
            filters,
            caller_filters: 1,
            resolved,
            ..subscription()
        };

        // A node fabricates a create whose integer property is text with a multi-byte character
        // across byte 20, and supplies its correct hash.
        let fabricated = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from([2u8; 32]),
            transitions: vec![BatchedTransition::Document(DocumentTransition::Create(
                DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                    base: DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
                        id: Identifier::from([3u8; 32]),
                        document_type_name: "rating".to_string(),
                        data_contract_id: contract.id(),
                        identity_contract_nonce: 0,
                        token_payment_info: None,
                    }),
                    entropy: [0u8; 32],
                    data: std::collections::BTreeMap::from([(
                        "stars".to_string(),
                        Value::Text(format!("{}é", "a".repeat(19))),
                    )]),
                    prefunded_voting_balance: None,
                }),
            ))],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }))
        .serialize_to_bytes()
        .expect("serializable");

        assert!(subscription
            .accept(response(Responses::StateTransition(matched(fabricated))))
            .expect("skipped, not failed")
            .is_none());
    }

    #[test]
    fn should_move_off_a_node_only_when_it_is_at_fault() {
        for code in [
            Code::OutOfRange,
            Code::FailedPrecondition,
            Code::Unavailable,
            Code::Internal,
        ] {
            assert!(node_at_fault(&Status::new(code, "")), "{code:?}");
        }
        // An ended lifetime, a cancellation or a slow client is not the node's fault.
        for code in [
            Code::DeadlineExceeded,
            Code::Cancelled,
            Code::ResourceExhausted,
            Code::Aborted,
        ] {
            assert!(!node_at_fault(&Status::new(code, "")), "{code:?}");
        }
    }

    #[tokio::test]
    async fn should_keep_failing_after_the_subscription_ended() {
        let mut subscription = subscription();
        subscription
            .accept(response(Responses::Checkpoint(Checkpoint {
                block_height: 41,
            })))
            .expect("accepted");
        // Reopening the stream fails for good: the mock SDK refuses the request.
        let error = subscription.next().await.expect_err("refused");
        assert!(
            subscription.terminated.is_some(),
            "next() latches a terminal failure"
        );
        // Later calls do not try again, and the cursor stays where it was.
        let later = subscription.next().await.expect_err("ended");
        assert!(later.to_string().contains("has ended"), "{later}");
        assert!(later.to_string().contains(&error.to_string()), "{later}");
        assert_eq!(subscription.resume_height(), Some(42));
        assert!(subscription.stream.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn should_keep_the_idle_window_across_cancelled_reads() {
        let mut idle = IdleWatch::default();
        // A read cancelled before the window ends, as by a caller multiplexing `next()` with
        // other work, does not restart it: the next read sees it end 20 seconds in.
        assert!(timeout(Duration::from_secs(40), idle.silent())
            .await
            .is_err());
        assert!(timeout(Duration::from_secs(40), idle.silent())
            .await
            .is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn should_count_silence_from_a_window_without_messages() {
        let mut idle = IdleWatch::default();
        // A message during the first window carries the stream into a second one, on the same
        // timer: silence is reported once a whole window passes without a message.
        idle.heard();
        assert!(timeout(Duration::from_secs(110), idle.silent())
            .await
            .is_err());
        assert!(timeout(Duration::from_secs(10), idle.silent())
            .await
            .is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn should_keep_the_reconnect_wait_across_cancelled_calls() {
        let mut subscription = subscription();
        // A stream just failed: the next one opens after a second.
        subscription.consecutive_failures = 1;
        subscription.back_off();
        // Calls cancelled before then do not restart the wait; the third reaches the open
        // (which the mock SDK, expecting no request, refuses).
        for _ in 0..2 {
            assert!(timeout(Duration::from_millis(400), subscription.next())
                .await
                .is_err());
        }
        assert!(timeout(Duration::from_millis(400), subscription.next())
            .await
            .expect("the wait ended and the stream was opened")
            .is_err());
    }
}
