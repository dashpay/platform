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
//! evaluated against data contracts fetched with proofs. A node that sends a transition the
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
    subscribe_request, ResolvedFilters, StateTransitionFilter, MAX_FILTERS,
};
use dpp::dashcore::hashes::{sha256, Hash};
use dpp::data_contract::DataContract;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use rs_dapi_client::transport::{sleep, TransportError};
use rs_dapi_client::{DapiClientError, DapiRequestExecutor, IntoInner, RequestSettings};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

/// Consecutive failed attempts after which the subscription gives up.
const MAX_CONSECUTIVE_FAILURES: u32 = 5;
/// Wait before the first retry; doubles per consecutive failure.
const FIRST_RETRY_DELAY: Duration = Duration::from_secs(1);

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
    /// Where the next stream starts; `None` until the first checkpoint of a live-only start.
    resume_from: Option<u64>,
    consecutive_failures: u32,
}

impl Sdk {
    /// Subscribe to committed state transitions matching any of `filters`, scanning from
    /// `from_block_height` (inclusive), or from after the current tip when `None`.
    ///
    /// The data contracts the document filters name are fetched with proofs first, to check
    /// what the node sends; a contract that does not exist fails the call. The subscription
    /// also follows updates of those contracts, which the node applies to its matching, so both
    /// sides match against the same contract version; those updates are not reported unless
    /// `filters` asks for them.
    pub async fn subscribe_to_state_transitions(
        &self,
        mut filters: Vec<StateTransitionFilter>,
        from_block_height: Option<u64>,
    ) -> Result<StateTransitionSubscription, Error> {
        let caller_filters = filters.len();
        let data_contract_ids = ResolvedFilters::data_contract_ids(&filters);
        let mut contracts = BTreeMap::new();
        for data_contract_id in &data_contract_ids {
            if let Some(contract) = DataContract::fetch(self, *data_contract_id).await? {
                contracts.insert(*data_contract_id, Arc::new(contract));
            }
        }
        // Within the request's filter limit; without room the caller's filters go alone.
        if !data_contract_ids.is_empty() && filters.len() < MAX_FILTERS {
            filters.push(StateTransitionFilter::DataContracts {
                data_contract_ids: data_contract_ids.into_iter().collect(),
            });
        }
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
            resume_from: from_block_height,
            consecutive_failures: 0,
        };
        // Open the first stream now, so a refused request fails here rather than on `next`.
        subscription.stream = Some(subscription.open().await?);
        Ok(subscription)
    }
}

impl StateTransitionSubscription {
    /// The next matching transition or checkpoint. Resumes broken streams by itself; an error
    /// means the subscription cannot continue (the request was refused, or too many attempts
    /// in a row failed).
    pub async fn next(&mut self) -> Result<SubscriptionEvent, Error> {
        loop {
            let stream = match self.stream.as_mut() {
                Some(stream) => stream,
                None => {
                    let stream = self.reopen().await?;
                    self.stream_messages = 0;
                    self.stream.insert(stream)
                }
            };
            let failure = match stream.message().await {
                Ok(Some(response)) => {
                    self.stream_messages += 1;
                    // A stream that got past its opening checkpoint was healthy; a node that
                    // fails right after every start is not retried forever.
                    if self.stream_messages > 1 {
                        self.consecutive_failures = 0;
                    }
                    if let Some(event) = self.accept(response) {
                        return Ok(event);
                    }
                    continue;
                }
                // The node or a proxy ended the stream (its lifetime elapsed): continue.
                Ok(None) => None,
                Err(status) if resumable(&status) => Some(status),
                Err(status) => return Err(status_error(status)),
            };
            if let Some(status) = &failure {
                tracing::debug!(code = ?status.code(), message = status.message(), "state transition stream broke; resuming");
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
        }
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

    async fn open(&self) -> Result<Streaming<SubscribeToStateTransitionsResponse>, Error> {
        let request: SubscribeToStateTransitionsRequest =
            subscribe_request(&self.filters, self.resume_from)?;
        self.sdk
            .execute(request, RequestSettings::default())
            .await
            .into_inner()
            .map_err(Error::from)
    }

    /// Open a new stream after a break, backing off after consecutive failures.
    async fn reopen(&mut self) -> Result<Streaming<SubscribeToStateTransitionsResponse>, Error> {
        loop {
            if self.consecutive_failures > 0 {
                sleep(FIRST_RETRY_DELAY * 2u32.pow(self.consecutive_failures - 1)).await;
            }
            match self.open().await {
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

    /// The event a response carries, after checking it; `None` for a response to skip.
    fn accept(
        &mut self,
        response: SubscribeToStateTransitionsResponse,
    ) -> Option<SubscriptionEvent> {
        let Some(Version::V0(response)) = response.version else {
            tracing::warn!("skipping a state transition subscription response of unknown version");
            return None;
        };
        match response.responses? {
            Responses::Checkpoint(checkpoint) => {
                self.resume_from = Some(checkpoint.block_height + 1);
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
                    return None;
                }
                let state_transition = match StateTransition::deserialize_from_bytes_untrusted(
                    &matched.state_transition,
                ) {
                    Ok(state_transition) => state_transition,
                    Err(error) => {
                        tracing::warn!(height = matched.block_height, %error, "skipping a state transition this SDK cannot decode");
                        return None;
                    }
                };
                let platform_version = PlatformVersion::get(matched.protocol_version)
                    .unwrap_or_else(|_| PlatformVersion::latest());
                let local_match = self.resolved.matches(&state_transition, platform_version);
                self.rebind_contract(&state_transition, platform_version);
                let Some(mut local_match) = local_match else {
                    tracing::warn!(
                        height = matched.block_height,
                        "skipping a state transition the node sent that does not match the filters"
                    );
                    return None;
                };
                // Contract updates the subscription follows only for itself are not reported.
                let caller_filters = self.caller_filters as u32;
                local_match
                    .matched_filters
                    .retain(|index| *index < caller_filters);
                if local_match.matched_filters.is_empty() {
                    return None;
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
        }
    }

    /// Later transitions on an updated contract match against the version the update carries,
    /// as the node does.
    fn rebind_contract(
        &mut self,
        state_transition: &StateTransition,
        platform_version: &PlatformVersion,
    ) {
        if let StateTransition::DataContractUpdate(update) = state_transition {
            if let Ok(contract) = DataContract::try_from_platform_versioned(
                update.data_contract().clone(),
                false,
                &mut vec![],
                platform_version,
            ) {
                self.resolved
                    .rebind_data_contract(Arc::new(contract), platform_version);
            }
        }
    }
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
}
