//! EventMux: a generic multiplexer between multiple Platform event subscribers
//! and producers. Subscribers send `PlatformEventsCommand` and receive
//! `PlatformEventsResponse`. Producers receive commands and generate responses.
//!
//! Features:
//! - Multiple subscribers and producers
//! - Round-robin dispatch of commands to producers
//! - Register per-subscriber filters on Add, remove on Remove
//! - Fan-out responses to all subscribers whose filters match

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use dapi_grpc::platform::v0::platform_events_command::platform_events_command_v0::Command as Cmd;
use dapi_grpc::platform::v0::platform_events_command::Version as CmdVersion;
use dapi_grpc::platform::v0::platform_events_response::platform_events_response_v0::Response as Resp;
use dapi_grpc::platform::v0::PlatformEventsCommand;
use dapi_grpc::tonic::Status;
use futures::SinkExt;
use sender_sink::wrappers::{SinkError, UnboundedSenderSink};
use tokio::join;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{mpsc, Mutex};

use crate::event_bus::{EventBus, Filter as EventFilter, SubscriptionHandle};
use dapi_grpc::platform::v0::PlatformEventsResponse;
use dapi_grpc::platform::v0::PlatformFilterV0;

pub type EventsCommandResult = Result<PlatformEventsCommand, Status>;
pub type EventsResponseResult = Result<PlatformEventsResponse, Status>;

pub type CommandSender = UnboundedSender<EventsCommandResult>;
pub type CommandReceiver = UnboundedReceiver<EventsCommandResult>;

pub type ResponseSender = UnboundedSender<EventsResponseResult>;
pub type ResponseReceiver = UnboundedReceiver<EventsResponseResult>;

/// EventMux: manages subscribers and producers, routes commands and responses.
pub struct EventMux {
    bus: EventBus<PlatformEventsResponse, IdFilter>,
    producers: Arc<Mutex<Vec<Option<CommandSender>>>>,
    rr_counter: Arc<AtomicUsize>,
    tasks: Arc<Mutex<tokio::task::JoinSet<()>>>,
    subscriptions: Arc<std::sync::Mutex<BTreeMap<SubscriptionKey, SubscriptionInfo>>>,
    next_subscriber_id: Arc<AtomicUsize>,
}

impl Default for EventMux {
    fn default() -> Self {
        Self::new()
    }
}

impl EventMux {
    async fn handle_subscriber_disconnect(&self, subscriber_id: u64) {
        tracing::debug!(subscriber_id, "event_mux: handling subscriber disconnect");
        self.remove_subscriber(subscriber_id).await;
    }
    /// Create a new, empty EventMux without producers or subscribers.
    pub fn new() -> Self {
        Self {
            bus: EventBus::new(),
            producers: Arc::new(Mutex::new(Vec::new())),
            rr_counter: Arc::new(AtomicUsize::new(0)),
            tasks: Arc::new(Mutex::new(tokio::task::JoinSet::new())),
            subscriptions: Arc::new(std::sync::Mutex::new(BTreeMap::new())),
            next_subscriber_id: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// Register a new producer. Returns an `EventProducer` comprised of:
    /// - `cmd_rx`: producer receives commands from the mux
    /// - `resp_tx`: producer sends generated responses into the mux
    pub async fn add_producer(&self) -> EventProducer {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<EventsCommandResult>();
        let (resp_tx, resp_rx) = mpsc::unbounded_channel::<EventsResponseResult>();

        // Store command sender so mux can forward commands via round-robin
        {
            let mut prods = self.producers.lock().await;
            prods.push(Some(cmd_tx));
        }

        // Route producer responses into the event bus
        let bus = self.bus.clone();
        let mux = self.clone();
        let producer_index = {
            let prods = self.producers.lock().await;
            prods.len().saturating_sub(1)
        };
        {
            let mut tasks = self.tasks.lock().await;
            tasks.spawn(async move {
                let mut rx = resp_rx;
                while let Some(resp) = rx.recv().await {
                    match resp {
                        Ok(response) => {
                            bus.notify(response).await;
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "event_mux: producer response error");
                        }
                    }
                }

                // producer disconnected
                tracing::warn!(index = producer_index, "event_mux: producer disconnected");
                mux.on_producer_disconnected(producer_index).await;
            });
        }

        EventProducer { cmd_rx, resp_tx }
    }

    /// Register a new subscriber.
    ///
    /// Subscriber is automatically cleaned up when channels are closed.
    pub async fn add_subscriber(&self) -> EventSubscriber {
        let (sub_cmd_tx, sub_cmd_rx) = mpsc::unbounded_channel::<EventsCommandResult>();
        let (sub_resp_tx, sub_resp_rx) = mpsc::unbounded_channel::<EventsResponseResult>();

        let mux = self.clone();
        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed) as u64;

        {
            let mut tasks = self.tasks.lock().await;
            tasks.spawn(async move {
                mux.run_subscriber_loop(subscriber_id, sub_cmd_rx, sub_resp_tx)
                    .await;
            });
        }

        EventSubscriber {
            cmd_tx: sub_cmd_tx,
            resp_rx: sub_resp_rx,
        }
    }

    async fn run_subscriber_loop(
        self,
        subscriber_id: u64,
        mut sub_cmd_rx: CommandReceiver,
        sub_resp_tx: ResponseSender,
    ) {
        tracing::debug!(subscriber_id, "event_mux: starting subscriber loop");

        loop {
            let cmd = match sub_cmd_rx.recv().await {
                Some(Ok(c)) => c,
                Some(Err(e)) => {
                    tracing::warn!(subscriber_id, error=%e, "event_mux: subscriber command error");
                    continue;
                }
                None => {
                    tracing::debug!(
                        subscriber_id,
                        "event_mux: subscriber command channel closed"
                    );
                    break;
                }
            };

            if let Some(CmdVersion::V0(v0)) = &cmd.version {
                match &v0.command {
                    Some(Cmd::Add(add)) => {
                        let id = add.client_subscription_id.clone();
                        tracing::debug!(subscriber_id, subscription_id = %id, "event_mux: adding subscription");

                        // If a subscription with this id already exists for this subscriber,
                        // remove it first to avoid duplicate fan-out and leaked handles.
                        if let Some((prev_sub_id, prev_handle_id, prev_assigned)) = {
                            let subs = self.subscriptions.lock().unwrap();
                            subs.get(&SubscriptionKey {
                                subscriber_id,
                                id: id.clone(),
                            })
                            .map(|info| {
                                (info.subscriber_id, info.handle.id(), info.assigned_producer)
                            })
                        } {
                            if prev_sub_id == subscriber_id {
                                tracing::warn!(
                                    subscriber_id,
                                    subscription_id = %id,
                                    "event_mux: duplicate Add detected, removing previous subscription first"
                                );
                                // Remove previous bus subscription
                                self.bus.remove_subscription(prev_handle_id).await;
                                // Notify previously assigned producer about removal
                                if let Some(prev_idx) = prev_assigned {
                                    if let Some(tx) = self.get_producer_tx(prev_idx).await {
                                        let remove_cmd = PlatformEventsCommand {
                                            version: Some(CmdVersion::V0(
                                                dapi_grpc::platform::v0::platform_events_command::PlatformEventsCommandV0 {
                                                    command: Some(Cmd::Remove(
                                                        dapi_grpc::platform::v0::RemoveSubscriptionV0 {
                                                            client_subscription_id: id.clone(),
                                                        },
                                                    )),
                                                },
                                            )),
                                        };
                                        let _ = tx.send(Ok(remove_cmd));
                                    }
                                }
                                // Drop previous mapping entry (it will be replaced below)
                                let _ = {
                                    self.subscriptions.lock().unwrap().remove(&SubscriptionKey {
                                        subscriber_id,
                                        id: id.clone(),
                                    })
                                };
                            }
                        }

                        // Create subscription filtered by client_subscription_id and forward events
                        let handle = self
                            .bus
                            .add_subscription(IdFilter { id: id.clone() })
                            .await
                            .no_unsubscribe_on_drop();

                        {
                            let mut subs = self.subscriptions.lock().unwrap();
                            subs.insert(
                                SubscriptionKey {
                                    subscriber_id,
                                    id: id.clone(),
                                },
                                SubscriptionInfo {
                                    subscriber_id,
                                    filter: add.filter.clone(),
                                    assigned_producer: None,
                                    handle: handle.clone(),
                                },
                            );
                        }

                        // Assign producer for this subscription
                        if let Some((_idx, prod_tx)) = self
                            .assign_producer_for_subscription(subscriber_id, &id)
                            .await
                        {
                            if prod_tx.send(Ok(cmd)).is_err() {
                                tracing::debug!(subscription_id = %id, "event_mux: failed to send Add to producer - channel closed");
                            }
                        } else {
                            // TODO: handle no producers available, possibly spawned jobs didn't start yet
                            tracing::warn!(subscription_id = %id, "event_mux: no producers available for Add");
                        }

                        // Start fan-out task for this subscription
                        let tx = sub_resp_tx.clone();
                        let mux = self.clone();
                        let sub_id = subscriber_id;
                        let mut tasks = self.tasks.lock().await;
                        tasks.spawn(async move {
                            let h = handle;
                            loop {
                                match h.recv().await {
                                    Some(resp) => {
                                        if tx.send(Ok(resp)).is_err() {
                                            tracing::debug!(subscription_id = %id, "event_mux: failed to send response - subscriber channel closed");
                                            mux.handle_subscriber_disconnect(sub_id).await;
                                            break;
                                        }
                                    }
                                    None => {
                                        tracing::debug!(subscription_id = %id, "event_mux: subscription ended");
                                        mux.handle_subscriber_disconnect(sub_id).await;
                                        break;
                                    }
                                }
                            }
                        });
                    }
                    Some(Cmd::Remove(rem)) => {
                        let id = rem.client_subscription_id.clone();
                        tracing::debug!(subscriber_id, subscription_id = %id, "event_mux: removing subscription");

                        // Remove subscription from bus and registry, and get assigned producer
                        let removed = {
                            self.subscriptions.lock().unwrap().remove(&SubscriptionKey {
                                subscriber_id,
                                id: id.clone(),
                            })
                        };
                        let assigned = if let Some(info) = removed {
                            self.bus.remove_subscription(info.handle.id()).await;
                            info.assigned_producer
                        } else {
                            None
                        };

                        if let Some(idx) = assigned {
                            if let Some(tx) = self.get_producer_tx(idx).await {
                                if tx.send(Ok(cmd)).is_err() {
                                    tracing::debug!(subscription_id = %id, "event_mux: failed to send Remove to producer - channel closed");
                                    self.handle_subscriber_disconnect(subscriber_id).await;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // subscriber disconnected: use the centralized cleanup method
        tracing::debug!(subscriber_id, "event_mux: subscriber disconnected");
        self.handle_subscriber_disconnect(subscriber_id).await;
    }

    /// Remove a subscriber and clean up all associated resources
    pub async fn remove_subscriber(&self, subscriber_id: u64) {
        tracing::debug!(subscriber_id, "event_mux: removing subscriber");

        // Get all subscription IDs for this subscriber by iterating through subscriptions
        let keys: Vec<SubscriptionKey> = {
            let subs = self.subscriptions.lock().unwrap();
            subs.iter()
                .filter_map(|(key, info)| {
                    if info.subscriber_id == subscriber_id {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };

        tracing::debug!(
            subscriber_id,
            subscription_count = keys.len(),
            "event_mux: found subscriptions for subscriber"
        );

        // Remove each subscription from the bus and notify producers
        for key in keys {
            let id = key.id.clone();
            let removed = { self.subscriptions.lock().unwrap().remove(&key) };
            let assigned = if let Some(info) = removed {
                self.bus.remove_subscription(info.handle.id()).await;
                tracing::debug!(subscription_id = %id, "event_mux: removed subscription from bus");
                info.assigned_producer
            } else {
                None
            };

            // Send remove command to assigned producer
            if let Some(idx) = assigned {
                if let Some(tx) = self.get_producer_tx(idx).await {
                    let cmd = PlatformEventsCommand {
                        version: Some(CmdVersion::V0(
                            dapi_grpc::platform::v0::platform_events_command::PlatformEventsCommandV0 {
                                command: Some(Cmd::Remove(
                                    dapi_grpc::platform::v0::RemoveSubscriptionV0 {
                                        client_subscription_id: id.clone(),
                                    },
                                )),
                            },
                        )),
                    };
                    if tx.send(Ok(cmd)).is_err() {
                        tracing::debug!(subscription_id = %id, "event_mux: failed to send Remove to producer - channel closed");
                    } else {
                        tracing::debug!(subscription_id = %id, "event_mux: sent Remove command to producer");
                    }
                }
            }
        }

        tracing::debug!(subscriber_id, "event_mux: subscriber removed");
    }

    async fn assign_producer_for_subscription(
        &self,
        subscriber_id: u64,
        subscription_id: &str,
    ) -> Option<(usize, mpsc::UnboundedSender<EventsCommandResult>)> {
        let prods_guard = self.producers.lock().await;
        if prods_guard.is_empty() {
            return None;
        }
        // Prefer existing assignment
        {
            let subs = self.subscriptions.lock().unwrap();
            if let Some(info) = subs.get(&SubscriptionKey {
                subscriber_id,
                id: subscription_id.to_string(),
            }) {
                if let Some(idx) = info.assigned_producer {
                    if let Some(Some(tx)) = prods_guard.get(idx) {
                        return Some((idx, tx.clone()));
                    }
                }
            }
        }
        // Use round-robin assignment for new subscriptions
        let idx = self.rr_counter.fetch_add(1, Ordering::Relaxed) % prods_guard.len();
        let mut chosen_idx = idx;

        // Find first alive producer starting from round-robin position
        let chosen = loop {
            if let Some(Some(tx)) = prods_guard.get(chosen_idx) {
                break Some((chosen_idx, tx.clone()));
            }
            chosen_idx = (chosen_idx + 1) % prods_guard.len();
            if chosen_idx == idx {
                break None; // Cycled through all producers
            }
        };

        drop(prods_guard);
        if let Some((idx, tx)) = chosen {
            if let Some(info) = self
                .subscriptions
                .lock()
                .unwrap()
                .get_mut(&SubscriptionKey {
                    subscriber_id,
                    id: subscription_id.to_string(),
                })
            {
                info.assigned_producer = Some(idx);
            }
            Some((idx, tx))
        } else {
            None
        }
    }

    async fn get_producer_tx(
        &self,
        idx: usize,
    ) -> Option<mpsc::UnboundedSender<EventsCommandResult>> {
        let prods = self.producers.lock().await;
        prods.get(idx).and_then(|o| o.as_ref().cloned())
    }

    async fn on_producer_disconnected(&self, index: usize) {
        // mark slot None
        {
            let mut prods = self.producers.lock().await;
            if index < prods.len() {
                prods[index] = None;
            }
        }
        // collect affected subscribers
        let affected_subscribers: BTreeSet<u64> = {
            let subs = self.subscriptions.lock().unwrap();
            subs.iter()
                .filter_map(|(_id, info)| {
                    if info.assigned_producer == Some(index) {
                        Some(info.subscriber_id)
                    } else {
                        None
                    }
                })
                .collect()
        };

        // Remove all affected subscribers using the centralized method
        for sub_id in affected_subscribers {
            tracing::warn!(
                subscriber_id = sub_id,
                producer_index = index,
                "event_mux: closing subscriber due to producer disconnect"
            );
            self.remove_subscriber(sub_id).await;
        }
        // Note: reconnection of the actual producer transport is delegated to the caller.
    }
}

// Hashing moved to murmur3::murmur3_32 for deterministic producer selection.

impl Clone for EventMux {
    fn clone(&self) -> Self {
        Self {
            bus: self.bus.clone(),
            producers: self.producers.clone(),
            rr_counter: self.rr_counter.clone(),
            tasks: self.tasks.clone(),
            subscriptions: self.subscriptions.clone(),
            next_subscriber_id: self.next_subscriber_id.clone(),
        }
    }
}

impl EventMux {
    /// Convenience API: subscribe directly with a filter and receive a subscription handle.
    /// This method creates an internal subscription keyed by a generated client_subscription_id,
    /// assigns a producer, sends the Add command upstream, and returns the id with an event bus handle.
    pub async fn subscribe(
        &self,
        filter: PlatformFilterV0,
    ) -> Result<(String, SubscriptionHandle<PlatformEventsResponse, IdFilter>), Status> {
        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed) as u64;
        let id = format!("sub-{}", subscriber_id);

        // Create bus subscription and register mapping
        let handle = self.bus.add_subscription(IdFilter { id: id.clone() }).await;
        {
            let mut subs = self.subscriptions.lock().unwrap();
            subs.insert(
                SubscriptionKey {
                    subscriber_id,
                    id: id.clone(),
                },
                SubscriptionInfo {
                    subscriber_id,
                    filter: Some(filter.clone()),
                    assigned_producer: None,
                    handle: handle.clone(),
                },
            );
        }

        // Assign producer and send Add
        if let Some((_idx, tx)) = self
            .assign_producer_for_subscription(subscriber_id, &id)
            .await
        {
            let cmd = PlatformEventsCommand {
                version: Some(CmdVersion::V0(
                    dapi_grpc::platform::v0::platform_events_command::PlatformEventsCommandV0 {
                        command: Some(Cmd::Add(dapi_grpc::platform::v0::AddSubscriptionV0 {
                            client_subscription_id: id.clone(),
                            filter: Some(filter.clone()),
                        })),
                    },
                )),
            };
            let _ = tx.send(Ok(cmd));

            Ok((id, handle))
        } else {
            tracing::warn!(subscription_id = %id, "event_mux: no producers available for Add");
            Err(Status::unavailable("no producers available"))
        }
    }
}

/// Handle used by application code to implement a concrete producer.
/// - `cmd_rx`: read commands from the mux
/// - `resp_tx`: send generated responses into the mux
pub struct EventProducer {
    pub cmd_rx: CommandReceiver,
    pub resp_tx: ResponseSender,
}

impl EventProducer {
    /// Forward all messages from  cmd_rx to self.cmd_tx and form resp_rx to self.resp_tx
    pub async fn forward<C, R>(self, mut cmd_tx: C, resp_rx: R)
    where
        C: futures::Sink<EventsCommandResult> + Unpin + Send + 'static,
        R: futures::Stream<Item = EventsResponseResult> + Unpin + Send + 'static,
        //  R: AsyncRead + Unpin + ?Sized,
        // W: AsyncWrite + Unpin + ?Sized,
    {
        use futures::stream::StreamExt;

        let mut cmd_rx = self.cmd_rx;

        let resp_tx = self.resp_tx;
        // let workers = JoinSet::new();
        let cmd_worker = tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                if cmd_tx.send(cmd).await.is_err() {
                    tracing::warn!("event_mux: failed to forward command to producer");
                    break;
                }
            }
            tracing::error!("event_mux: command channel closed, stopping producer forwarder");
        });

        let resp_worker = tokio::spawn(async move {
            let mut rx = resp_rx;
            while let Some(resp) = rx.next().await {
                if resp_tx.send(resp).is_err() {
                    tracing::warn!("event_mux: failed to forward response to mux");
                    break;
                }
            }
            tracing::error!(
                "event_mux: response channel closed, stopping producer response forwarder"
            );
        });

        let _ = join!(cmd_worker, resp_worker);
    }
}
/// Handle used by application code to implement a concrete subscriber.
/// Subscriber is automatically cleaned up when channels are closed.
pub struct EventSubscriber {
    pub cmd_tx: CommandSender,
    pub resp_rx: ResponseReceiver,
}

impl EventSubscriber {
    /// Forward all messages from cmd_rx to self.cmd_tx and from self.resp_rx to resp_tx
    pub async fn forward<C, R>(self, cmd_rx: C, mut resp_tx: R)
    where
        C: futures::Stream<Item = EventsCommandResult> + Unpin + Send + 'static,
        R: futures::Sink<EventsResponseResult> + Unpin + Send + 'static,
    {
        use futures::stream::StreamExt;

        let cmd_tx = self.cmd_tx;
        let mut resp_rx = self.resp_rx;

        let cmd_worker = tokio::spawn(async move {
            let mut rx = cmd_rx;
            while let Some(cmd) = rx.next().await {
                if cmd_tx.send(cmd).is_err() {
                    tracing::warn!("event_mux: failed to forward command from subscriber");
                    break;
                }
            }
            tracing::error!(
                "event_mux: subscriber command channel closed, stopping command forwarder"
            );
        });

        let resp_worker = tokio::spawn(async move {
            while let Some(resp) = resp_rx.recv().await {
                if resp_tx.send(resp).await.is_err() {
                    tracing::warn!("event_mux: failed to forward response to subscriber");
                    break;
                }
            }
            tracing::error!(
                "event_mux: subscriber response channel closed, stopping response forwarder"
            );
        });

        let _ = join!(cmd_worker, resp_worker);
    }
} // ---- Filters ----

#[derive(Clone)]
pub struct IdFilter {
    id: String,
}

impl EventFilter<PlatformEventsResponse> for IdFilter {
    fn matches(&self, event: &PlatformEventsResponse) -> bool {
        if let Some(dapi_grpc::platform::v0::platform_events_response::Version::V0(v0)) =
            &event.version
        {
            match &v0.response {
                Some(Resp::Event(ev)) => ev.client_subscription_id == self.id,
                Some(Resp::Ack(ack)) => ack.client_subscription_id == self.id,
                Some(Resp::Error(err)) => err.client_subscription_id == self.id,
                None => false,
            }
        } else {
            false
        }
    }
}

struct SubscriptionInfo {
    subscriber_id: u64,
    #[allow(dead_code)]
    filter: Option<PlatformFilterV0>,
    assigned_producer: Option<usize>,
    handle: SubscriptionHandle<PlatformEventsResponse, IdFilter>,
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct SubscriptionKey {
    subscriber_id: u64,
    id: String,
}

/// Public alias for platform events subscription handle used by SDK and DAPI.
pub type PlatformEventsSubscriptionHandle = SubscriptionHandle<PlatformEventsResponse, IdFilter>;

/// Create a Sink from an UnboundedSender that maps errors to tonic::Status
pub fn unbounded_sender_sink<T>(
    sender: UnboundedSender<T>,
) -> impl futures::Sink<Result<T, Status>, Error = Status> {
    let cmd_sink = Box::pin(
        UnboundedSenderSink::from(sender)
            .sink_map_err(|e: SinkError| {
                Status::internal(format!(
                    "Failed to send command to PlatformEventsMux: {:?}",
                    e
                ))
            })
            .with(|v| async { v }),
    );

    cmd_sink
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::platform_event_v0 as pe;
    use dapi_grpc::platform::v0::platform_events_command::PlatformEventsCommandV0;
    use dapi_grpc::platform::v0::platform_events_response::PlatformEventsResponseV0;
    use dapi_grpc::platform::v0::{PlatformEventMessageV0, PlatformEventV0, PlatformFilterV0};
    use sender_sink::wrappers::UnboundedSenderSink;
    use std::collections::HashMap;
    use tokio::time::{timeout, Duration};

    fn make_add_cmd(id: &str) -> PlatformEventsCommand {
        PlatformEventsCommand {
            version: Some(CmdVersion::V0(PlatformEventsCommandV0 {
                command: Some(Cmd::Add(dapi_grpc::platform::v0::AddSubscriptionV0 {
                    client_subscription_id: id.to_string(),
                    filter: Some(PlatformFilterV0::default()),
                })),
            })),
        }
    }

    fn make_event_resp(id: &str) -> PlatformEventsResponse {
        let meta = pe::BlockMetadata {
            height: 1,
            time_ms: 0,
            block_id_hash: vec![],
        };
        let evt = PlatformEventV0 {
            event: Some(pe::Event::BlockCommitted(pe::BlockCommitted {
                meta: Some(meta),
                tx_count: 0,
            })),
        };

        PlatformEventsResponse {
            version: Some(
                dapi_grpc::platform::v0::platform_events_response::Version::V0(
                    PlatformEventsResponseV0 {
                        response: Some(Resp::Event(PlatformEventMessageV0 {
                            client_subscription_id: id.to_string(),
                            event: Some(evt),
                        })),
                    },
                ),
            ),
        }
    }

    #[tokio::test]
    async fn should_deliver_events_once_per_subscriber_with_shared_id() {
        let mux = EventMux::new();

        // Single producer captures Add/Remove commands and accepts responses
        let EventProducer {
            mut cmd_rx,
            resp_tx,
        } = mux.add_producer().await;

        // Two subscribers share the same client_subscription_id
        let EventSubscriber {
            cmd_tx: mut sub1_cmd_tx,
            resp_rx: mut resp_rx1,
        } = mux.add_subscriber().await;
        let EventSubscriber {
            cmd_tx: mut sub2_cmd_tx,
            resp_rx: mut resp_rx2,
        } = mux.add_subscriber().await;

        let sub_id = "dup-sub";

        sub1_cmd_tx
            .send(Ok(make_add_cmd(sub_id)))
            .expect("send add for subscriber 1");
        sub2_cmd_tx
            .send(Ok(make_add_cmd(sub_id)))
            .expect("send add for subscriber 2");

        // Ensure producer receives both Add commands
        for _ in 0..2 {
            let got = timeout(Duration::from_secs(1), cmd_rx.recv())
                .await
                .expect("timeout waiting for Add")
                .expect("producer channel closed")
                .expect("Add command error");
            match got.version.and_then(|v| match v {
                CmdVersion::V0(v0) => v0.command,
            }) {
                Some(Cmd::Add(a)) => assert_eq!(a.client_subscription_id, sub_id),
                other => panic!("expected Add command, got {:?}", other),
            }
        }

        // Emit a single event targeting the shared subscription id
        resp_tx
            .send(Ok(make_event_resp(sub_id)))
            .expect("failed to send event into mux");

        let extract_id = |resp: PlatformEventsResponse| -> String {
            match resp.version.and_then(|v| match v {
                dapi_grpc::platform::v0::platform_events_response::Version::V0(v0) => {
                    v0.response.and_then(|r| match r {
                        Resp::Event(m) => Some(m.client_subscription_id),
                        _ => None,
                    })
                }
            }) {
                Some(id) => id,
                None => panic!("unexpected response variant"),
            }
        };

        let ev1 = timeout(Duration::from_secs(1), resp_rx1.recv())
            .await
            .expect("timeout waiting for subscriber1 event")
            .expect("subscriber1 channel closed")
            .expect("subscriber1 event error");
        let ev2 = timeout(Duration::from_secs(1), resp_rx2.recv())
            .await
            .expect("timeout waiting for subscriber2 event")
            .expect("subscriber2 channel closed")
            .expect("subscriber2 event error");

        assert_eq!(extract_id(ev1), sub_id);
        assert_eq!(extract_id(ev2), sub_id);

        // Ensure no duplicate deliveries per subscriber
        assert!(timeout(Duration::from_millis(100), resp_rx1.recv())
            .await
            .is_err());
        assert!(timeout(Duration::from_millis(100), resp_rx2.recv())
            .await
            .is_err());

        // Drop subscribers to trigger Remove for both
        drop(sub1_cmd_tx);
        drop(resp_rx1);
        drop(sub2_cmd_tx);
        drop(resp_rx2);

        for _ in 0..2 {
            let got = timeout(Duration::from_secs(1), cmd_rx.recv())
                .await
                .expect("timeout waiting for Remove")
                .expect("producer channel closed")
                .expect("Remove command error");
            match got.version.and_then(|v| match v {
                CmdVersion::V0(v0) => v0.command,
            }) {
                Some(Cmd::Remove(r)) => assert_eq!(r.client_subscription_id, sub_id),
                other => panic!("expected Remove command, got {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn mux_chain_three_layers_delivers_once_per_subscriber() {
        use tokio_stream::wrappers::UnboundedReceiverStream;

        // Build three muxes
        let mux1 = EventMux::new();
        let mux2 = EventMux::new();
        let mux3 = EventMux::new();

        // Bridge: Mux1 -> Producer1a -> Subscriber2a -> Mux2
        //      and Mux1 -> Producer1b -> Subscriber2b -> Mux2
        let prod1a = mux1.add_producer().await;
        let sub2a = mux2.add_subscriber().await;
        // Use a sink that accepts EventsCommandResult directly (no extra Result nesting)
        let sub2a_cmd_sink = UnboundedSenderSink::from(sub2a.cmd_tx.clone());
        let sub2a_resp_stream = UnboundedReceiverStream::new(sub2a.resp_rx);
        tokio::spawn(async move { prod1a.forward(sub2a_cmd_sink, sub2a_resp_stream).await });

        let prod1b = mux1.add_producer().await;
        let sub2b = mux2.add_subscriber().await;
        let sub2b_cmd_sink = UnboundedSenderSink::from(sub2b.cmd_tx.clone());
        let sub2b_resp_stream = UnboundedReceiverStream::new(sub2b.resp_rx);
        tokio::spawn(async move { prod1b.forward(sub2b_cmd_sink, sub2b_resp_stream).await });

        // Bridge: Mux2 -> Producer2 -> Subscriber3 -> Mux3
        let prod2 = mux2.add_producer().await;
        let sub3 = mux3.add_subscriber().await;
        let sub3_cmd_sink = UnboundedSenderSink::from(sub3.cmd_tx.clone());
        let sub3_resp_stream = UnboundedReceiverStream::new(sub3.resp_rx);
        tokio::spawn(async move { prod2.forward(sub3_cmd_sink, sub3_resp_stream).await });

        // Deepest producers where we will capture commands and inject events
        let p3a = mux3.add_producer().await;
        let p3b = mux3.add_producer().await;
        let mut p3a_cmd_rx = p3a.cmd_rx;
        let p3a_resp_tx = p3a.resp_tx;
        let mut p3b_cmd_rx = p3b.cmd_rx;
        let p3b_resp_tx = p3b.resp_tx;

        // Three top-level subscribers on Mux1
        let mut sub1a = mux1.add_subscriber().await;
        let mut sub1b = mux1.add_subscriber().await;
        let mut sub1c = mux1.add_subscriber().await;
        let id_a = "s1a";
        let id_b = "s1b";
        let id_c = "s1c";

        // Send Add commands downstream from each subscriber
        sub1a
            .cmd_tx
            .send(Ok(make_add_cmd(id_a)))
            .expect("send add a");
        sub1b
            .cmd_tx
            .send(Ok(make_add_cmd(id_b)))
            .expect("send add b");
        sub1c
            .cmd_tx
            .send(Ok(make_add_cmd(id_c)))
            .expect("send add c");

        // Ensure deepest producers receive each Add exactly once and not on both
        let mut assigned: HashMap<String, usize> = HashMap::new();
        for _ in 0..3 {
            let (which, got_opt) = timeout(Duration::from_secs(2), async {
                tokio::select! {
                    c = p3a_cmd_rx.recv() => (0usize, c),
                    c = p3b_cmd_rx.recv() => (1usize, c),
                }
            })
            .await
            .expect("timeout waiting for downstream add");

            let got = got_opt
                .expect("p3 cmd channel closed")
                .expect("downstream add error");

            match got.version.and_then(|v| match v {
                CmdVersion::V0(v0) => v0.command,
            }) {
                Some(Cmd::Add(a)) => {
                    let id = a.client_subscription_id;
                    if let Some(prev) = assigned.insert(id.clone(), which) {
                        panic!(
                            "subscription {} was dispatched to two producers: {} and {}",
                            id, prev, which
                        );
                    }
                }
                _ => panic!("expected Add at deepest producer"),
            }
        }
        assert!(
            assigned.contains_key(id_a)
                && assigned.contains_key(id_b)
                && assigned.contains_key(id_c)
        );

        // Emit one event per subscription id via the assigned deepest producer
        match assigned.get(id_a) {
            Some(0) => p3a_resp_tx
                .send(Ok(make_event_resp(id_a)))
                .expect("emit event a"),
            Some(1) => p3b_resp_tx
                .send(Ok(make_event_resp(id_a)))
                .expect("emit event a"),
            _ => panic!("missing assignment for id_a"),
        }
        match assigned.get(id_b) {
            Some(0) => p3a_resp_tx
                .send(Ok(make_event_resp(id_b)))
                .expect("emit event b"),
            Some(1) => p3b_resp_tx
                .send(Ok(make_event_resp(id_b)))
                .expect("emit event b"),
            _ => panic!("missing assignment for id_b"),
        }
        match assigned.get(id_c) {
            Some(0) => p3a_resp_tx
                .send(Ok(make_event_resp(id_c)))
                .expect("emit event c"),
            Some(1) => p3b_resp_tx
                .send(Ok(make_event_resp(id_c)))
                .expect("emit event c"),
            _ => panic!("missing assignment for id_c"),
        }

        // Receive each exactly once at the top-level subscribers
        let a_first = timeout(Duration::from_secs(2), sub1a.resp_rx.recv())
            .await
            .expect("timeout waiting for a event")
            .expect("a subscriber closed")
            .expect("a event error");
        let b_first = timeout(Duration::from_secs(2), sub1b.resp_rx.recv())
            .await
            .expect("timeout waiting for b event")
            .expect("b subscriber closed")
            .expect("b event error");
        let c_first = timeout(Duration::from_secs(2), sub1c.resp_rx.recv())
            .await
            .expect("timeout waiting for c event")
            .expect("c subscriber closed")
            .expect("c event error");

        let get_id = |resp: PlatformEventsResponse| -> String {
            match resp.version.and_then(|v| match v {
                dapi_grpc::platform::v0::platform_events_response::Version::V0(v0) => {
                    v0.response.and_then(|r| match r {
                        Resp::Event(m) => Some(m.client_subscription_id),
                        _ => None,
                    })
                }
            }) {
                Some(id) => id,
                None => panic!("unexpected response variant"),
            }
        };

        assert_eq!(get_id(a_first.clone()), id_a);
        assert_eq!(get_id(b_first.clone()), id_b);
        assert_eq!(get_id(c_first.clone()), id_c);

        // Ensure no duplicates by timing out on the next recv
        let a_dup = timeout(Duration::from_millis(200), sub1a.resp_rx.recv()).await;
        assert!(a_dup.is_err(), "unexpected duplicate for subscriber a");
        let b_dup = timeout(Duration::from_millis(200), sub1b.resp_rx.recv()).await;
        assert!(b_dup.is_err(), "unexpected duplicate for subscriber b");
        let c_dup = timeout(Duration::from_millis(200), sub1c.resp_rx.recv()).await;
        assert!(c_dup.is_err(), "unexpected duplicate for subscriber c");
    }
}
