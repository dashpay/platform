//! Generic, clonable in-process event bus with pluggable filtering.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{Mutex, RwLock, mpsc};

const DEFAULT_SUBSCRIPTION_CAPACITY: usize = 256;

/// Filter trait for event matching on a specific event type.
pub trait Filter<E>: Send + Sync {
    /// Return true if the event matches the filter.
    fn matches(&self, event: &E) -> bool;
}

/// Internal subscription structure.
///
/// Note: no Clone impl, so that dropping the sender closes the channel.
struct Subscription<E, F> {
    filter: F,
    sender: mpsc::Sender<E>,
}

/// Generic, clonable in-process event bus with pluggable filtering.
pub struct EventBus<E, F> {
    subs: Arc<RwLock<BTreeMap<u64, Subscription<E, F>>>>,
    counter: Arc<AtomicU64>,
    tasks: Arc<Mutex<tokio::task::JoinSet<()>>>, // tasks spawned for this subscription, cancelled on drop
    channel_capacity: usize,
}

impl<E, F> Clone for EventBus<E, F> {
    fn clone(&self) -> Self {
        Self {
            subs: Arc::clone(&self.subs),
            counter: Arc::clone(&self.counter),
            tasks: Arc::clone(&self.tasks),
            channel_capacity: self.channel_capacity,
        }
    }
}

impl<E, F> Default for EventBus<E, F>
where
    E: Clone + Send + 'static,
    F: Filter<E> + Send + Sync + Debug + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<E, F> EventBus<E, F> {
    /// Remove a subscription by id and update metrics.
    pub async fn remove_subscription(&self, id: u64) {
        let mut subs = self.subs.write().await;
        if subs.remove(&id).is_some() {
            metrics_unsubscribe_inc();
            metrics_active_gauge_set(subs.len());
            tracing::debug!("event_bus: removed subscription id={}", id);
        } else {
            tracing::debug!("event_bus: subscription id={} not found, not removed", id);
        }
    }
}

impl<E, F> EventBus<E, F>
where
    E: Clone + Send + 'static,
    F: Filter<E> + Debug + Send + Sync + 'static,
{
    /// Create a new, empty event bus.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_SUBSCRIPTION_CAPACITY)
    }

    /// Create a new event bus with a custom per-subscription channel capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        metrics_register_once();
        Self {
            subs: Arc::new(RwLock::new(BTreeMap::new())),
            counter: Arc::new(AtomicU64::new(0)),
            tasks: Arc::new(Mutex::new(tokio::task::JoinSet::new())),
            channel_capacity: capacity.max(1),
        }
    }

    /// Add a new subscription using the provided filter.
    pub async fn add_subscription(&self, filter: F) -> SubscriptionHandle<E, F> {
        tracing::trace!(?filter, "event_bus: adding subscription");

        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel::<E>(self.channel_capacity);

        let sub = Subscription { filter, sender: tx };

        {
            let mut subs = self.subs.write().await;
            subs.insert(id, sub);
            metrics_active_gauge_set(subs.len());
            metrics_subscribe_inc();
        }
        tracing::debug!(sub_id = id, "event_bus: added subscription");

        SubscriptionHandle {
            id,
            rx: Arc::new(Mutex::new(rx)),
            drop: true,
            event_bus: self.clone(),
        }
    }

    /// Publish an event to all subscribers whose filters match, using
    /// the current Tokio runtime if available, otherwise log a warning.
    ///
    /// This is a best-effort, fire-and-forget variant of `notify`.
    pub fn notify_sync(&self, event: E) {
        let bus = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                bus.notify(event).await;
            });
        } else {
            tracing::warn!("event_bus.notify_sync: no current tokio runtime");
        }
    }

    /// Publish an event to all subscribers whose filters match.
    pub async fn notify(&self, event: E) {
        metrics_events_published_inc();

        let mut targets = Vec::new();
        {
            let subs_guard = self.subs.read().await;
            for (id, sub) in subs_guard.iter() {
                if sub.filter.matches(&event) {
                    targets.push((*id, sub.sender.clone()));
                }
            }
        }

        if targets.is_empty() {
            return;
        }

        let mut dead = Vec::new();

        for (id, sender) in targets.into_iter() {
            let payload = event.clone();

            match sender.try_send(payload) {
                Ok(()) => {
                    metrics_events_delivered_inc();
                    tracing::trace!(subscription_id = id, "event_bus: event delivered");
                }
                Err(TrySendError::Full(_value)) => {
                    metrics_events_dropped_inc();
                    tracing::warn!(
                        subscription_id = id,
                        "event_bus: subscriber queue full, removing laggy subscriber to protect others"
                    );
                    // Drop the event for this subscriber and remove subscription
                    dead.push(id);
                }
                Err(TrySendError::Closed(_value)) => {
                    metrics_events_dropped_inc();
                    dead.push(id);
                }
            }
        }

        for id in dead {
            tracing::debug!(
                subscription_id = id,
                "event_bus: removing dead subscription"
            );
            self.remove_subscription(id).await;
        }
    }

    /// Get the current number of active subscriptions.
    pub async fn subscription_count(&self) -> usize {
        self.subs.read().await.len()
    }

    /// Copy all event messages from an unbounded mpsc receiver into the event bus.
    pub async fn copy_from_unbounded_mpsc(&self, mut rx: mpsc::UnboundedReceiver<E>) {
        let bus = self.clone();
        let mut tasks = self.tasks.lock().await;
        tasks.spawn(async move {
            while let Some(event) = rx.recv().await {
                bus.notify(event).await;
            }
        });
    }
}

/// RAII subscription handle; dropping the last clone removes the subscription.
pub struct SubscriptionHandle<E, F>
where
    E: Send + 'static,
    F: Send + Sync + 'static,
{
    id: u64,
    rx: Arc<Mutex<mpsc::Receiver<E>>>,
    event_bus: EventBus<E, F>,
    drop: bool, // true only for primary handles
}

impl<E, F> Clone for SubscriptionHandle<E, F>
where
    E: Send + 'static,
    F: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            rx: Arc::clone(&self.rx),
            event_bus: self.event_bus.clone(),
            drop: self.drop,
        }
    }
}

impl<E, F> SubscriptionHandle<E, F>
where
    E: Send + 'static,
    F: Send + Sync + 'static,
{
    /// Get the unique ID of this subscription.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Receive the next event for this subscription.
    pub async fn recv(&self) -> Option<E> {
        let mut rx = self.rx.lock().await;
        rx.recv().await
    }

    /// Disable automatic unsubscription when the last handle is dropped.
    ///
    /// By default, dropping the final [`SubscriptionHandle`] removes the
    /// subscription from the [`EventBus`]. Calling this method keeps the
    /// subscription registered so that the caller can explicitly remove it
    /// via [`EventBus::remove_subscription`].
    pub fn no_unsubscribe_on_drop(mut self) -> Self {
        self.drop = false;
        self
    }
}

impl<E, F> Drop for SubscriptionHandle<E, F>
where
    E: Send + 'static,
    F: Send + Sync + 'static,
{
    fn drop(&mut self) {
        if self.drop {
            // Remove only when the last clone of this handle is dropped
            if Arc::strong_count(&self.rx) == 1 {
                let bus = self.event_bus.clone();
                let id = self.id;

                // Prefer removing via Tokio if a runtime is available
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        bus.remove_subscription(id).await;
                    });
                } else {
                    // Fallback: best-effort synchronous removal using try_write()
                    if let Ok(mut subs) = bus.subs.try_write()
                        && subs.remove(&id).is_some()
                    {
                        metrics_unsubscribe_inc();
                        metrics_active_gauge_set(subs.len());
                    }
                }
            }
        }
    }
}

// ---- Metrics helpers (gated) ----

#[cfg(feature = "metrics")]
mod met {
    use metrics::{counter, describe_counter, describe_gauge, gauge};
    use std::sync::Once;

    pub const ACTIVE_SUBSCRIPTIONS: &str = "event_bus_active_subscriptions";
    pub const SUBSCRIBE_TOTAL: &str = "event_bus_subscribe_total";
    pub const UNSUBSCRIBE_TOTAL: &str = "event_bus_unsubscribe_total";
    pub const EVENTS_PUBLISHED_TOTAL: &str = "event_bus_events_published_total";
    pub const EVENTS_DELIVERED_TOTAL: &str = "event_bus_events_delivered_total";
    pub const EVENTS_DROPPED_TOTAL: &str = "event_bus_events_dropped_total";

    pub fn register_metrics_once() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            describe_gauge!(
                ACTIVE_SUBSCRIPTIONS,
                "Current number of active event bus subscriptions"
            );
            describe_counter!(
                SUBSCRIBE_TOTAL,
                "Total subscriptions created on the event bus"
            );
            describe_counter!(
                UNSUBSCRIBE_TOTAL,
                "Total subscriptions removed from the event bus"
            );
            describe_counter!(
                EVENTS_PUBLISHED_TOTAL,
                "Total events published to the event bus"
            );
            describe_counter!(
                EVENTS_DELIVERED_TOTAL,
                "Total events delivered to subscribers"
            );
            describe_counter!(
                EVENTS_DROPPED_TOTAL,
                "Total events dropped due to dead subscribers"
            );
        });
    }

    pub fn active_gauge_set(n: usize) {
        gauge!(ACTIVE_SUBSCRIPTIONS).set(n as f64);
    }
    pub fn subscribe_inc() {
        counter!(SUBSCRIBE_TOTAL).increment(1);
    }
    pub fn unsubscribe_inc() {
        counter!(UNSUBSCRIBE_TOTAL).increment(1);
    }
    pub fn events_published_inc() {
        counter!(EVENTS_PUBLISHED_TOTAL).increment(1);
    }
    pub fn events_delivered_inc() {
        counter!(EVENTS_DELIVERED_TOTAL).increment(1);
    }
    pub fn events_dropped_inc() {
        counter!(EVENTS_DROPPED_TOTAL).increment(1);
    }
}

#[cfg(feature = "metrics")]
#[inline]
fn metrics_register_once() {
    met::register_metrics_once()
}
#[cfg(not(feature = "metrics"))]
#[inline]
fn metrics_register_once() {}

#[cfg(feature = "metrics")]
#[inline]
fn metrics_active_gauge_set(n: usize) {
    met::active_gauge_set(n)
}
#[cfg(not(feature = "metrics"))]
#[inline]
fn metrics_active_gauge_set(_n: usize) {}

#[cfg(feature = "metrics")]
#[inline]
fn metrics_subscribe_inc() {
    met::subscribe_inc()
}
#[cfg(not(feature = "metrics"))]
#[inline]
fn metrics_subscribe_inc() {}

#[cfg(feature = "metrics")]
#[inline]
fn metrics_unsubscribe_inc() {
    met::unsubscribe_inc()
}
#[cfg(not(feature = "metrics"))]
#[inline]
fn metrics_unsubscribe_inc() {}

#[cfg(feature = "metrics")]
#[inline]
fn metrics_events_published_inc() {
    met::events_published_inc()
}
#[cfg(not(feature = "metrics"))]
#[inline]
fn metrics_events_published_inc() {}

#[cfg(feature = "metrics")]
#[inline]
fn metrics_events_delivered_inc() {
    met::events_delivered_inc()
}
#[cfg(not(feature = "metrics"))]
#[inline]
fn metrics_events_delivered_inc() {}

#[cfg(feature = "metrics")]
#[inline]
fn metrics_events_dropped_inc() {
    met::events_dropped_inc()
}
#[cfg(not(feature = "metrics"))]
#[inline]
fn metrics_events_dropped_inc() {}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, timeout};

    #[derive(Clone, Debug, PartialEq)]
    enum Evt {
        Num(u32),
    }

    #[derive(Clone, Debug)]
    struct EvenOnly;

    impl Filter<Evt> for EvenOnly {
        fn matches(&self, e: &Evt) -> bool {
            matches!(e, Evt::Num(n) if n % 2 == 0)
        }
    }

    #[tokio::test]
    async fn basic_subscribe_and_notify() {
        let bus: EventBus<Evt, EvenOnly> = EventBus::new();
        let sub = bus.add_subscription(EvenOnly).await;

        bus.notify(Evt::Num(1)).await; // filtered out
        bus.notify(Evt::Num(2)).await; // delivered

        let got = timeout(Duration::from_millis(200), sub.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got, Evt::Num(2));
    }

    #[tokio::test]
    async fn drop_removes_subscription() {
        let bus: EventBus<Evt, EvenOnly> = EventBus::new();
        let sub = bus.add_subscription(EvenOnly).await;
        assert_eq!(bus.subscription_count().await, 1);
        drop(sub);

        for _ in 0..10 {
            if bus.subscription_count().await == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(bus.subscription_count().await, 0);
    }

    #[tokio::test]
    async fn multiple_events_delivered() {
        let bus: EventBus<Evt, EvenOnly> = EventBus::new();
        let sub = bus.add_subscription(EvenOnly).await;

        bus.notify(Evt::Num(2)).await;
        bus.notify(Evt::Num(12)).await;

        let a = timeout(Duration::from_millis(200), sub.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(a, Evt::Num(2));
        let b = timeout(Duration::from_millis(200), sub.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(b, Evt::Num(12));
    }

    #[tokio::test]
    async fn no_unsubscribe_on_drop_allows_manual_cleanup() {
        let bus: EventBus<Evt, EvenOnly> = EventBus::new();
        let handle = bus
            .add_subscription(EvenOnly)
            .await
            .no_unsubscribe_on_drop();
        let id = handle.id();

        drop(handle);
        // Automatic removal should not happen
        assert_eq!(bus.subscription_count().await, 1);

        bus.remove_subscription(id).await;
        assert_eq!(bus.subscription_count().await, 0);
    }

    #[tokio::test]
    async fn unsubscribe() {
        let bus: EventBus<Evt, EvenOnly> = EventBus::new();
        let sub = bus.add_subscription(EvenOnly).await;

        bus.notify(Evt::Num(2)).await;
        bus.notify(Evt::Num(12)).await;

        bus.remove_subscription(sub.id()).await;

        bus.notify(Evt::Num(3)).await; // not delivered as we already unsubscribed

        let a = timeout(Duration::from_millis(200), sub.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(a, Evt::Num(2));
        let b = timeout(Duration::from_millis(200), sub.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(b, Evt::Num(12));

        let c = timeout(Duration::from_millis(200), sub.recv()).await;
        assert!(c.unwrap().is_none(), "only two events should be received",);
    }
}
