//! Admission and the per-subscription scan loop.

use super::CHECKPOINT_INTERVAL;
use super::block_source::{BlockRead, BlockSource, HeightMeta, META_PAGE, NOT_YET_RETRY};
use dapi_grpc::platform::v0::SubscribeToStateTransitionsResponse;
use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::subscribe_to_state_transitions_response_v0::{
    Checkpoint, Responses, StateTransitionMatch,
};
use dapi_grpc::platform::v0::subscribe_to_state_transitions_response::{
    SubscribeToStateTransitionsResponseV0, Version,
};
use dapi_grpc::tonic::Status;
use dash_platform_queries::subscriptions::ResolvedFilters;
use dpp::data_contract::DataContract;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::StateTransition;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::version::PlatformVersion;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, watch};
use tokio::time::{Instant, sleep, timeout};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, warn};

/// The response stream of one subscription.
pub type SubscriptionStream = ReceiverStream<Result<SubscribeToStateTransitionsResponse, Status>>;

/// Messages buffered per subscription before the client must read.
const STREAM_BUFFER: usize = 32;
/// How long a full buffer may stay full before the subscription ends.
const SEND_DEADLINE: Duration = Duration::from_secs(60);
/// How long a height may stay unreadable before the subscription ends.
const UNREADABLE_DEADLINE: Duration = Duration::from_secs(30);
/// How often the tip is polled when no new-block event arrives.
const TIP_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Bounds on what subscriptions may cost one node.
#[derive(Debug, Clone, Copy)]
pub struct SubscriptionLimits {
    /// Concurrent subscriptions on the node.
    pub max_subscriptions: usize,
    /// Concurrent subscriptions from one client address (IPv6: per /64).
    pub max_subscriptions_per_ip: usize,
    /// Subscriptions catching up on history at the same time.
    pub max_replaying: usize,
    /// How far behind the tip a subscription may start.
    pub max_replay_blocks: u64,
    /// How far ahead of the tip a subscription may start; it waits for those blocks.
    pub max_blocks_ahead: u64,
}

impl Default for SubscriptionLimits {
    fn default() -> Self {
        Self {
            max_subscriptions: 1024,
            max_subscriptions_per_ip: 16,
            max_replaying: 8,
            max_replay_blocks: 50_000,
            max_blocks_ahead: 1_000,
        }
    }
}

/// A subscription's claim on the node's subscription capacity, released on drop.
pub struct Admission {
    _permit: OwnedSemaphorePermit,
    _per_ip: Option<PerIpGuard>,
}

struct PerIpGuard {
    key: IpAddr,
    counts: Arc<Mutex<HashMap<IpAddr, usize>>>,
}

impl Drop for PerIpGuard {
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
    replaying: Arc<Semaphore>,
    per_ip: Arc<Mutex<HashMap<IpAddr, usize>>>,
}

impl SubscriptionService {
    /// `tip` carries the highest height Tenderdash reports stored; see [`Self::track_tip`].
    pub fn new(
        blocks: Arc<BlockSource>,
        tip: watch::Receiver<u64>,
        limits: SubscriptionLimits,
    ) -> Self {
        Self {
            blocks,
            tip,
            subscriptions: Arc::new(Semaphore::new(limits.max_subscriptions)),
            replaying: Arc::new(Semaphore::new(limits.max_replaying)),
            per_ip: Arc::new(Mutex::new(HashMap::new())),
            limits,
        }
    }

    /// Keep `tip` at the highest height Tenderdash reports, polling every few seconds and
    /// whenever `new_block` fires. Runs until `tip` has no receivers.
    pub async fn track_tip(
        blocks: Arc<BlockSource>,
        tip: watch::Sender<u64>,
        mut new_block: impl FnMut() -> futures::future::BoxFuture<'static, ()> + Send,
    ) {
        loop {
            match blocks.latest_height().await {
                Ok(height) => {
                    tip.send_if_modified(|current| {
                        let advanced = height > *current;
                        if advanced {
                            *current = height;
                        }
                        advanced
                    });
                }
                Err(error) => debug!(%error, "cannot read the Tenderdash tip"),
            }
            if tip.is_closed() {
                return;
            }
            tokio::select! {
                _ = sleep(TIP_POLL_INTERVAL) => {}
                _ = new_block() => {}
            }
        }
    }

    /// Claim capacity for a new subscription from `client_ip`, or refuse it.
    pub fn admit(&self, client_ip: Option<IpAddr>) -> Result<Admission, Status> {
        let permit = self
            .subscriptions
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                Status::resource_exhausted("too many state transition subscriptions on this node")
            })?;
        let per_ip = match client_ip.map(per_ip_key) {
            Some(key) => {
                let mut counts = self.per_ip.lock().unwrap_or_else(|e| e.into_inner());
                let count = counts.entry(key).or_insert(0);
                if *count >= self.limits.max_subscriptions_per_ip {
                    return Err(Status::resource_exhausted(format!(
                        "at most {} state transition subscriptions per client address",
                        self.limits.max_subscriptions_per_ip
                    )));
                }
                *count += 1;
                Some(PerIpGuard {
                    key,
                    counts: self.per_ip.clone(),
                })
            }
            None => None,
        };
        Ok(Admission {
            _permit: permit,
            _per_ip: per_ip,
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
                if height + self.limits.max_replay_blocks < tip {
                    return Err(Status::out_of_range(format!(
                        "from_block_height {height} is more than {} blocks behind the tip {tip}",
                        self.limits.max_replay_blocks
                    )));
                }
                if height > tip + self.limits.max_blocks_ahead {
                    return Err(Status::out_of_range(format!(
                        "from_block_height {height} is more than {} blocks ahead of the tip {tip}",
                        self.limits.max_blocks_ahead
                    )));
                }
                height
            }
        };

        let (sender, receiver) = mpsc::channel(STREAM_BUFFER);
        let scan = Scan {
            blocks: self.blocks.clone(),
            tip: self.tip.clone(),
            replaying: self.replaying.clone(),
            filters,
            sender,
            next_height: start,
        };
        tokio::spawn(async move {
            let _admission = admission;
            scan.run().await;
        });
        Ok(ReceiverStream::new(receiver))
    }

    async fn current_tip(&self) -> Result<u64, Status> {
        let tip = *self.tip.borrow();
        if tip > 0 {
            return Ok(tip);
        }
        self.blocks
            .latest_height()
            .await
            .map_err(|e| Status::unavailable(format!("cannot read the Tenderdash tip: {e}")))
    }
}

/// IPv4 addresses count individually; IPv6 addresses by /64, the smallest block a client
/// usually controls.
fn per_ip_key(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => {
            let mut segments = v6.segments();
            segments[4..].fill(0);
            IpAddr::V6(segments.into())
        }
    }
}

/// Why a scan stopped.
enum Stop {
    /// The client went away.
    Closed,
    /// End the stream with this status.
    Fail(Status),
}

struct Scan {
    blocks: Arc<BlockSource>,
    tip: watch::Receiver<u64>,
    replaying: Arc<Semaphore>,
    filters: ResolvedFilters,
    sender: mpsc::Sender<Result<SubscribeToStateTransitionsResponse, Status>>,
    /// The next height to scan; every height below it has been fully scanned.
    next_height: u64,
}

impl Scan {
    async fn run(mut self) {
        let outcome = self.scan().await;
        if let Err(Stop::Fail(status)) = outcome {
            debug!(code = ?status.code(), message = status.message(), "ending state transition subscription");
            let _ = timeout(SEND_DEADLINE, self.sender.send(Err(status))).await;
        }
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

            // Catching up on many blocks: share the replay capacity, a page at a time.
            let _replay_permit = if tip - self.next_height >= META_PAGE {
                Some(self.replay_permit(&mut last_checkpoint).await?)
            } else {
                None
            };
            let (page_start, page) =
                self.blocks
                    .meta_page(self.next_height, tip)
                    .await
                    .map_err(|e| {
                        Stop::Fail(Status::unavailable(format!("cannot read block metas: {e}")))
                    })?;
            let page_end = page_start + page.len() as u64 - 1;
            while self.next_height <= page_end {
                match page[(self.next_height - page_start) as usize] {
                    HeightMeta::Empty => {}
                    HeightMeta::Unavailable => {
                        return Err(Stop::Fail(Status::out_of_range(format!(
                            "block {} is not available on this node",
                            self.next_height
                        ))));
                    }
                    HeightMeta::HasTxs => {
                        if self.scan_block(self.next_height).await? {
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

    /// Wait for replay capacity, keeping the client informed while waiting.
    async fn replay_permit(
        &mut self,
        last_checkpoint: &mut Instant,
    ) -> Result<OwnedSemaphorePermit, Stop> {
        loop {
            tokio::select! {
                permit = self.replaying.clone().acquire_owned() => {
                    return permit.map_err(|_| Stop::Fail(Status::unavailable("the node is shutting down")));
                }
                _ = self.sender.closed() => return Err(Stop::Closed),
                _ = sleep(CHECKPOINT_INTERVAL.saturating_sub(last_checkpoint.elapsed())) => {
                    self.checkpoint().await?;
                    *last_checkpoint = Instant::now();
                }
            }
        }
    }

    /// Scan one block, sending its matches and then a checkpoint at its height. Returns whether
    /// anything matched.
    async fn scan_block(&mut self, height: u64) -> Result<bool, Stop> {
        let deadline = Instant::now() + UNREADABLE_DEADLINE;
        let block = loop {
            match self.blocks.read(height).await {
                Ok(BlockRead::Block(block)) => break block,
                Ok(BlockRead::NotYet) if Instant::now() < deadline => sleep(NOT_YET_RETRY).await,
                Ok(BlockRead::NotYet) => {
                    return Err(Stop::Fail(Status::unavailable(format!(
                        "block {height} has not become readable; resume from {height}"
                    ))));
                }
                Ok(BlockRead::Unavailable) => {
                    return Err(Stop::Fail(Status::out_of_range(format!(
                        "block {height} is not available on this node"
                    ))));
                }
                Err(error) => {
                    return Err(Stop::Fail(Status::unavailable(format!(
                        "cannot read block {height}: {error}; resume from {height}"
                    ))));
                }
            }
        };
        let platform_version = PlatformVersion::get(block.protocol_version).map_err(|_| {
            Stop::Fail(Status::failed_precondition(format!(
                "block {height} executed under protocol version {}, which this node does not \
                 know; resume from {height} on an upgraded node",
                block.protocol_version
            )))
        })?;

        let mut matched = false;
        for tx in &block.txs {
            let state_transition = StateTransition::deserialize_from_bytes_untrusted(&tx.bytes).map_err(|e| {
                warn!(height, index = tx.index, error = %e, "cannot decode an executed state transition");
                Stop::Fail(Status::failed_precondition(format!(
                    "block {height} transaction {} cannot be decoded by this node ({e}); resume \
                     from {height} on an upgraded node",
                    tx.index
                )))
            })?;
            if let Some(filter_match) = self.filters.matches(&state_transition, platform_version) {
                matched = true;
                self.send(Responses::StateTransition(StateTransitionMatch {
                    block_height: height,
                    block_time_ms: block.time_ms,
                    protocol_version: block.protocol_version,
                    index_in_block: tx.index,
                    state_transition_hash: tx.hash.to_vec(),
                    state_transition: tx.bytes.as_ref().clone(),
                    matched_filters: filter_match.matched_filters,
                    matched_batch_positions: filter_match.matched_batch_positions,
                }))
                .await?;
            }
            // Later transitions match against the contract as this update left it.
            if let StateTransition::DataContractUpdate(update) = &state_transition
                && let Ok(contract) = DataContract::try_from_platform_versioned(
                    update.data_contract().clone(),
                    false,
                    &mut vec![],
                    platform_version,
                )
            {
                self.filters
                    .rebind_data_contract(Arc::new(contract), platform_version);
            }
        }
        if matched {
            self.checkpoint_at(height).await?;
        }
        Ok(matched)
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
            Err(_) => Err(Stop::Fail(Status::resource_exhausted(format!(
                "the client did not read for {}s; resume from the last checkpoint",
                SEND_DEADLINE.as_secs()
            )))),
        }
    }
}

#[cfg(test)]
mod tests;
