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
use dpp::data_contract::DataContract;
use dpp::prelude::Identifier;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use rs_dapi_client::transport::{sleep, TransportError};
use rs_dapi_client::{Address, DapiClientError, DapiRequestExecutor, RequestSettings};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Consecutive failed attempts after which the subscription gives up.
const MAX_CONSECUTIVE_FAILURES: u32 = 5;
/// Wait before the first retry; doubles per consecutive failure.
const FIRST_RETRY_DELAY: Duration = Duration::from_secs(1);
/// How long a stream may stay silent before it counts as stalled. The node checkpoints every
/// 10 seconds while idle; this leaves room for slow block reads on its side.
const MESSAGE_IDLE_DEADLINE: Duration = Duration::from_secs(90);

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
    /// Bound data contracts a delivered update changed; re-read with proofs before the next
    /// message is checked.
    contracts_to_refresh: BTreeSet<Identifier>,
    /// Why the subscription ended, once it has: later calls fail rather than read on.
    terminated: Option<String>,
    /// Fires when the current stream has been silent for `MESSAGE_IDLE_DEADLINE`; armed by
    /// the first read after a message or a new stream.
    idle: Option<IdleTimer>,
}

/// A deadline kept across reads.
type IdleTimer = Pin<Box<dyn Future<Output = ()> + Send>>;

/// `read`'s output, or `None` if `idle` fires first. `idle` is kept by the caller, so a read
/// that is cancelled and started again does not restart the deadline.
async fn read_or_idle<F: Future>(idle: &mut IdleTimer, read: F) -> Option<F::Output> {
    futures::pin_mut!(read);
    match futures::future::select(read, idle.as_mut()).await {
        futures::future::Either::Left((output, _)) => Some(output),
        futures::future::Either::Right(_) => None,
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
        for data_contract_id in &data_contract_ids {
            if let Some(contract) = DataContract::fetch(self, *data_contract_id).await? {
                contracts.insert(*data_contract_id, Arc::new(contract));
            }
        }
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
            contracts_to_refresh: BTreeSet::new(),
            terminated: None,
            idle: None,
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
                self.idle = None;
                self.stream.insert(stream)
            }
        };
        // The node checkpoints at least every few seconds; a stream quiet for much longer is
        // stalled, even if its connection still answers, and is resumed elsewhere. The timer
        // lives in the subscription, so a caller cancelling `next()` does not restart it.
        let idle = self
            .idle
            .get_or_insert_with(|| Box::pin(sleep(MESSAGE_IDLE_DEADLINE)));
        let message = read_or_idle(idle, stream.message())
            .await
            .unwrap_or_else(|| {
                Err(Status::unavailable(format!(
                    "no message for {}s",
                    MESSAGE_IDLE_DEADLINE.as_secs()
                )))
            });
        let failure = match message {
            Ok(Some(response)) => {
                self.idle = None;
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

    /// Open a new stream after a break, backing off after consecutive failures.
    async fn reopen(&mut self) -> Result<Streaming<SubscribeToStateTransitionsResponse>, Error> {
        loop {
            if self.consecutive_failures > 0 {
                sleep(FIRST_RETRY_DELAY * 2u32.pow(self.consecutive_failures - 1)).await;
            }
            match self.open(true).await {
                Ok(stream) => return Ok(stream),
                Err(error) => {
                    self.consecutive_failures += 1;
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

    /// Rebind the filters on contracts a delivered update changed to their current version,
    /// read with a proof. A contract that can no longer be read ends the subscription.
    async fn refresh_contracts(&mut self) -> Result<(), Error> {
        // An id leaves the queue only once rebound, so a cancelled or failed refresh is retried.
        while let Some(data_contract_id) = self.contracts_to_refresh.first().copied() {
            let contract = DataContract::fetch(&self.sdk, data_contract_id)
                .await?
                .ok_or_else(|| {
                    Error::MissingDependency(
                        "DataContract".to_string(),
                        format!("data contract {data_contract_id} not found after its update"),
                    )
                })?;
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
                // Decoded under the rules of the block's version, as consensus decoded it.
                let state_transition =
                    StateTransition::deserialize_from_bytes_untrusted_in_version(
                        &matched.state_transition,
                        platform_version,
                    )?;
                let local_match = self.resolved.matches(&state_transition, platform_version);
                if let Some(data_contract_id) =
                    self.resolved.followed_data_contract(&state_transition)
                {
                    // Never install the contract the node sent: re-read it with a proof.
                    self.contracts_to_refresh.insert(data_contract_id);
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
            contracts_to_refresh: BTreeSet::new(),
            terminated: None,
            idle: None,
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
        assert!(subscription.contracts_to_refresh.contains(&proved.id()));
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
        subscription.contracts_to_refresh.insert(data_contract_id);
        // The mock SDK has no expectation for this fetch, so the refresh fails part-way, as it
        // would if the future were dropped while fetching.
        assert!(subscription.refresh_contracts().await.is_err());
        assert!(subscription
            .contracts_to_refresh
            .contains(&data_contract_id));
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
        subscription.terminated = Some("a response could not be interpreted".to_string());
        // No stream is read or reopened, and the cursor stays where it was.
        assert!(subscription.next().await.is_err());
        assert!(subscription.next().await.is_err());
        assert_eq!(subscription.resume_height(), Some(42));
        assert!(subscription.stream.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn should_keep_the_idle_deadline_across_cancelled_reads() {
        let mut idle: IdleTimer = Box::pin(sleep(MESSAGE_IDLE_DEADLINE));
        // A read that never completes, cancelled twice before the deadline, as a caller
        // multiplexing `next()` with other work would.
        for _ in 0..2 {
            let cancelled = tokio::time::timeout(
                Duration::from_secs(40),
                read_or_idle(&mut idle, std::future::pending::<()>()),
            )
            .await;
            assert!(cancelled.is_err(), "still within the deadline");
        }
        // 80 seconds in: the original deadline fires 10 seconds into the next read, rather
        // than a fresh 90 seconds.
        let expired = tokio::time::timeout(
            Duration::from_secs(40),
            read_or_idle(&mut idle, std::future::pending::<()>()),
        )
        .await;
        assert_eq!(expired.ok(), Some(None), "the silence deadline expires");
    }
}
