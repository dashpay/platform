//! Admission and the per-subscription scan loop.

use super::CHECKPOINT_INTERVAL;
use super::block_source::{BlockSource, CommittedBlock, HeightMeta};
use crate::error::DAPIResult;
use crate::services::platform_service::shielded_proof_failure_budget::SourceKey;
use dapi_grpc::platform::v0::SubscribeToStateTransitionsResponse;
use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::subscribe_to_state_transitions_response_v0::{
    Checkpoint, Responses, StateTransitionMatch,
};
use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::{
    SubscribeToStateTransitionsResponseV0, Version,
};
use dapi_grpc::tonic::Status;
use dash_platform_queries::subscriptions::{FilterMatch, ResolvedFilters};
use dpp::version::PlatformVersion;
use futures::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, broadcast, mpsc, watch};
use tokio::time::{Instant, sleep, timeout};
use tracing::{debug, warn};

type Item = Result<SubscribeToStateTransitionsResponse, Status>;

/// What a subscription's stream reads: the buffered messages, then the status the scan ended
/// with, if any. The scan frees the messages of a client that stopped reading.
#[derive(Debug)]
struct Buffer {
    messages: mpsc::Receiver<Item>,
    end: Option<Status>,
}

/// The response stream of one subscription: the buffered messages, then the status the scan
/// ended with, if any.
#[derive(Debug)]
pub struct SubscriptionStream {
    buffer: Arc<Mutex<Buffer>>,
}

impl Stream for SubscriptionStream {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Item>> {
        let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        match buffer.messages.poll_recv(cx) {
            // The scan sets the status before dropping the sender, so it is there once the
            // channel has ended.
            Poll::Ready(None) => Poll::Ready(buffer.end.take().map(Err)),
            other => other,
        }
    }
}

/// Messages buffered per subscription before the client must read.
const STREAM_BUFFER: usize = 32;
/// How long a full buffer may stay full before the subscription ends.
const SEND_DEADLINE: Duration = Duration::from_secs(60);
/// How often the tip is polled when no new-block event arrives.
const TIP_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Bounds on what subscriptions may cost one node.
#[derive(Debug, Clone, Copy)]
pub struct SubscriptionLimits {
    /// Concurrent subscriptions on the node.
    pub max_subscriptions: usize,
    /// Concurrent subscriptions from one source (IPv6: per /48).
    pub max_subscriptions_per_source: usize,
    /// How far behind the tip a subscription may start.
    pub max_replay_blocks: u64,
    /// How far ahead of the tip a subscription may start; it waits for those blocks.
    pub max_blocks_ahead: u64,
}

impl Default for SubscriptionLimits {
    fn default() -> Self {
        Self {
            max_subscriptions: 256,
            max_subscriptions_per_source: 16,
            max_replay_blocks: 50_000,
            max_blocks_ahead: 1_000,
        }
    }
}

/// A subscription's claim on the node's subscription capacity, released on drop.
pub struct Admission {
    _permit: OwnedSemaphorePermit,
    _per_source: Option<PerSourceGuard>,
}

struct PerSourceGuard {
    key: SourceKey,
    counts: Arc<Mutex<HashMap<SourceKey, usize>>>,
}

impl Drop for PerSourceGuard {
    fn drop(&mut self) {
        let mut counts = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(count) = counts.get_mut(&self.key) {
            *count -= 1;
            if *count == 0 {
                counts.remove(&self.key);
            }
        }
    }
}

/// Serves every subscription of the node from one block source.
pub struct SubscriptionService {
    blocks: Arc<BlockSource>,
    tip: watch::Receiver<u64>,
    limits: SubscriptionLimits,
    subscriptions: Arc<Semaphore>,
    per_source: Arc<Mutex<HashMap<SourceKey, usize>>>,
}

impl SubscriptionService {
    /// `tip` carries the highest readable height; see [`Self::track_tip`].
    pub fn new(
        blocks: Arc<BlockSource>,
        tip: watch::Receiver<u64>,
        limits: SubscriptionLimits,
    ) -> Self {
        Self {
            blocks,
            tip,
            subscriptions: Arc::new(Semaphore::new(limits.max_subscriptions)),
            per_source: Arc::new(Mutex::new(HashMap::new())),
            limits,
        }
    }

    /// Keep `tip` at the highest height whose execution results are saved, checking on every
    /// `new_blocks` event and every few seconds. Runs until `tip` has no receivers.
    pub async fn track_tip<T: Clone>(
        blocks: Arc<BlockSource>,
        tip: watch::Sender<u64>,
        mut new_blocks: broadcast::Receiver<T>,
    ) {
        let mut events_open = true;
        loop {
            if let Err(error) = publish_tip(&blocks, &tip).await {
                debug!(%error, "cannot advance the state transition subscription tip");
            }
            if tip.is_closed() {
                return;
            }
            tokio::select! {
                _ = sleep(TIP_POLL_INTERVAL) => {}
                // A lagged receiver still means new blocks; a closed one leaves only the poll.
                event = new_blocks.recv(), if events_open => {
                    events_open = !matches!(event, Err(RecvError::Closed));
                }
            }
        }
    }

    /// Claim capacity for a new subscription from `source`, or refuse it.
    pub(super) fn admit(&self, source: Option<SourceKey>) -> Result<Admission, Status> {
        let permit = self
            .subscriptions
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                Status::resource_exhausted("too many state transition subscriptions on this node")
            })?;
        let per_source = match source {
            Some(key) => {
                let mut counts = self.per_source.lock().unwrap_or_else(|e| e.into_inner());
                let count = counts.entry(key).or_insert(0);
                if *count >= self.limits.max_subscriptions_per_source {
                    return Err(Status::resource_exhausted(format!(
                        "at most {} state transition subscriptions per client address",
                        self.limits.max_subscriptions_per_source
                    )));
                }
                *count += 1;
                Some(PerSourceGuard {
                    key,
                    counts: self.per_source.clone(),
                })
            }
            None => None,
        };
        Ok(Admission {
            _permit: permit,
            _per_source: per_source,
        })
    }

    /// Start scanning from `from_block_height`, or after the current tip.
    pub async fn start(
        &self,
        admission: Admission,
        filters: ResolvedFilters,
        from_block_height: Option<u64>,
    ) -> Result<SubscriptionStream, Status> {
        let tip = self.current_tip().await?;
        let start = match from_block_height {
            None => tip + 1,
            Some(height) => {
                if height.saturating_add(self.limits.max_replay_blocks) < tip {
                    return Err(Status::out_of_range(format!(
                        "from_block_height {height} is more than {} blocks behind the tip {tip}",
                        self.limits.max_replay_blocks
                    )));
                }
                // A node this far behind may still serve the height elsewhere.
                if height > tip.saturating_add(self.limits.max_blocks_ahead) {
                    return Err(Status::unavailable(format!(
                        "from_block_height {height} is more than {} blocks ahead of the tip {tip}",
                        self.limits.max_blocks_ahead
                    )));
                }
                height
            }
        };

        let (sender, messages) = mpsc::channel(STREAM_BUFFER);
        let buffer = Arc::new(Mutex::new(Buffer {
            messages,
            end: None,
        }));
        let scan = Scan {
            blocks: self.blocks.clone(),
            tip: self.tip.clone(),
            filters,
            sender,
            buffer: Arc::downgrade(&buffer),
            next_height: start.max(1),
        };
        // The task ends once the client drops the stream, which closes `sender`.
        tokio::spawn(scan.run(admission));
        Ok(SubscriptionStream { buffer })
    }

    async fn current_tip(&self) -> Result<u64, Status> {
        let tip = *self.tip.borrow();
        if tip > 0 {
            return Ok(tip);
        }
        // Not published yet: the height the tracker would publish.
        readable_tip(&self.blocks, 0)
            .await
            .map(|height| height.unwrap_or(0))
            .map_err(|e| Status::unavailable(format!("cannot read the Tenderdash tip: {e}")))
    }
}

/// Publish the readable tip; see [`readable_tip`].
async fn publish_tip(blocks: &BlockSource, tip: &watch::Sender<u64>) -> DAPIResult<()> {
    let published = *tip.borrow();
    if let Some(height) = readable_tip(blocks, published).await? {
        tip.send_replace(height);
    }
    Ok(())
}

/// The latest stored height once its results are saved, or the one before it while they are
/// not, so fast blocks cannot starve publication: every height at or below it is readable.
/// `None` if it is not above `published`.
async fn readable_tip(blocks: &BlockSource, published: u64) -> DAPIResult<Option<u64>> {
    let height = blocks.latest_height().await?;
    if height <= published {
        return Ok(None);
    }
    match blocks.results_saved(height).await {
        Ok(()) => Ok(Some(height)),
        Err(error) if height - 1 > published => {
            blocks.results_saved(height - 1).await?;
            debug!(%error, height, "tip results not saved yet; using the height before it");
            Ok(Some(height - 1))
        }
        Err(error) => Err(error),
    }
}

/// Why a scan stopped.
enum Stop {
    /// The client went away.
    Closed,
    /// End the stream with this status.
    Fail(Status),
    /// The client did not read for `SEND_DEADLINE`.
    SlowClient,
}

struct Scan {
    blocks: Arc<BlockSource>,
    tip: watch::Receiver<u64>,
    filters: ResolvedFilters,
    sender: mpsc::Sender<Item>,
    /// The stream's buffer, given the status the stream ends with before `sender` is dropped.
    /// Weak, so a client dropping the stream still closes the channel.
    buffer: Weak<Mutex<Buffer>>,
    /// The next height to scan; every height below it has been fully scanned.
    next_height: u64,
}

impl Scan {
    /// Scan until the subscription ends, then leave the status for the stream to end with, after
    /// what is buffered if the client reads it within `SEND_DEADLINE`. Capacity is given back
    /// only once the buffer is read or discarded, so a stalled client holds both or neither.
    async fn run(mut self, admission: Admission) {
        let (status, drained) = match self.scan().await {
            Ok(()) | Err(Stop::Closed) => return,
            // Its buffer already stayed full for the whole deadline.
            Err(Stop::SlowClient) => (
                Status::resource_exhausted(format!(
                    "the client did not read for {}s; resume from the last checkpoint",
                    SEND_DEADLINE.as_secs()
                )),
                false,
            ),
            Err(Stop::Fail(status)) => {
                debug!(code = ?status.code(), message = status.message(), "ending state transition subscription");
                // Every slot is free again once the client has read everything buffered.
                let drained = timeout(SEND_DEADLINE, self.sender.reserve_many(STREAM_BUFFER))
                    .await
                    .is_ok_and(|reserved| reserved.is_ok());
                (status, drained)
            }
        };
        if let Some(buffer) = self.buffer.upgrade() {
            let mut buffer = buffer.lock().unwrap_or_else(|e| e.into_inner());
            if !drained {
                while buffer.messages.try_recv().is_ok() {}
            }
            buffer.end = Some(status);
        }
        // Dropping the sender ends the channel; the capacity goes after the buffer.
        drop(self);
        drop(admission);
    }

    async fn scan(&mut self) -> Result<(), Stop> {
        self.checkpoint().await?;
        let mut last_checkpoint = Instant::now();
        loop {
            let tip = *self.tip.borrow();
            if self.next_height > tip {
                tokio::select! {
                    _ = self.sender.closed() => return Err(Stop::Closed),
                    changed = self.tip.changed() => {
                        if changed.is_err() {
                            return Err(Stop::Fail(Status::unavailable("the node is shutting down")));
                        }
                    }
                    _ = sleep(CHECKPOINT_INTERVAL.saturating_sub(last_checkpoint.elapsed())) => {
                        self.checkpoint().await?;
                        last_checkpoint = Instant::now();
                    }
                }
                continue;
            }
            if self.sender.is_closed() {
                return Err(Stop::Closed);
            }

            let (page_start, page) =
                unless_closed(&self.sender, self.blocks.meta_page(self.next_height, tip))
                    .await?
                    .map_err(|e| {
                        Stop::Fail(Status::unavailable(format!("cannot read block metas: {e}")))
                    })?;
            let page_end = page_start + page.len() as u64 - 1;
            while self.next_height <= page_end {
                let height = self.next_height;
                match page[(height - page_start) as usize] {
                    HeightMeta::Empty => {}
                    HeightMeta::Unavailable => {
                        return Err(Stop::Fail(Status::failed_precondition(format!(
                            "block {height} is not available on this node"
                        ))));
                    }
                    HeightMeta::HasTxs => {
                        if self.scan_block(height).await? {
                            last_checkpoint = Instant::now();
                        }
                    }
                }
                self.next_height += 1;
                if last_checkpoint.elapsed() >= CHECKPOINT_INTERVAL {
                    self.checkpoint().await?;
                    last_checkpoint = Instant::now();
                }
            }
        }
    }

    /// Send the matches of the block at `height`, then a checkpoint at it. Returns whether
    /// anything matched.
    async fn scan_block(&mut self, height: u64) -> Result<bool, Stop> {
        let block = unless_closed(&self.sender, self.blocks.read(height))
            .await?
            .map_err(|error| {
                Stop::Fail(Status::unavailable(format!(
                    "cannot read block {height}: {error}; resume from {height}"
                )))
            })?;
        let matches = self.match_block(&block)?;
        if matches.is_empty() {
            return Ok(false);
        }
        // Build the responses first, so a stalled client pins no decoded block.
        let responses: Vec<_> = matches
            .into_iter()
            .map(|(position, filter_match)| {
                let tx = &block.txs[position];
                Responses::StateTransition(StateTransitionMatch {
                    block_height: height,
                    block_time_ms: block.time_ms,
                    protocol_version: block.protocol_version,
                    state_transition_hash: tx.hash.to_vec(),
                    state_transition: tx.bytes.clone(),
                    matched_filters: filter_match.matched_filters,
                    matched_batch_positions: filter_match.matched_batch_positions,
                })
            })
            .collect();
        drop(block);
        for response in responses {
            self.send(response).await?;
        }
        self.checkpoint_at(height).await?;
        Ok(true)
    }

    /// The positions, in `block.txs`, of the block's transitions that match, in block order,
    /// with what they matched.
    fn match_block(&self, block: &CommittedBlock) -> Result<Vec<(usize, FilterMatch)>, Stop> {
        let height = block.height;
        let platform_version = PlatformVersion::get(block.protocol_version).map_err(|_| {
            Stop::Fail(Status::failed_precondition(format!(
                "block {height} executed under protocol version {}, which this node does not \
                 know; resume from {height} on an upgraded node",
                block.protocol_version
            )))
        })?;

        let mut matches = Vec::new();
        for (position, tx) in block.txs.iter().enumerate() {
            let state_transition = tx.state_transition.as_ref().map_err(|e| {
                warn!(height, index = tx.index, error = %e, "cannot decode an executed state transition");
                Stop::Fail(Status::failed_precondition(format!(
                    "block {height} transaction {} cannot be decoded by this node ({e}); resume \
                     from {height} on an upgraded node",
                    tx.index
                )))
            })?;
            if let Some(filter_match) = self.filters.matches(state_transition, platform_version) {
                matches.push((position, filter_match));
            }
        }
        Ok(matches)
    }

    /// A checkpoint at the highest fully scanned height.
    async fn checkpoint(&mut self) -> Result<(), Stop> {
        self.checkpoint_at(self.next_height - 1).await
    }

    async fn checkpoint_at(&mut self, height: u64) -> Result<(), Stop> {
        self.send(Responses::Checkpoint(Checkpoint {
            block_height: height,
        }))
        .await
    }

    async fn send(&mut self, message: Responses) -> Result<(), Stop> {
        let response = SubscribeToStateTransitionsResponse {
            version: Some(Version::V0(SubscribeToStateTransitionsResponseV0 {
                responses: Some(message),
            })),
        };
        match timeout(SEND_DEADLINE, self.sender.send(Ok(response))).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(Stop::Closed),
            Err(_) => Err(Stop::SlowClient),
        }
    }
}

/// `read`, or [`Stop::Closed`] once the client is gone: a read can queue behind other
/// subscriptions' fetches, and a gone client must not hold its admission meanwhile.
async fn unless_closed<T>(
    sender: &mpsc::Sender<Item>,
    read: impl Future<Output = T>,
) -> Result<T, Stop> {
    tokio::select! {
        biased;
        value = read => Ok(value),
        _ = sender.closed() => Err(Stop::Closed),
    }
}

#[cfg(test)]
mod tests;
