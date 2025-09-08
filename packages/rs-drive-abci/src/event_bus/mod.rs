//! Generic, clonable in-process event bus with pluggable filtering.
//!
//! Provides a generic `EventBus<E, F>` and `Filter<E>` trait, with
//! async subscribe/notify, RAII cleanup, and metrics instrumentation.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Once;

use metrics::{counter, describe_counter, describe_gauge, gauge};
use tokio::sync::{mpsc, Mutex, RwLock};

/// Filter trait for event matching on a specific event type.
pub trait Filter<E>: Send + Sync {
    /// Return true if the event matches the filter.
    fn matches(&self, event: &E) -> bool;
}

struct Subscription<E, F> {
    filter: F,
    sender: mpsc::UnboundedSender<E>,
}

/// Generic, clonable in‑process event bus with pluggable filtering.
pub struct EventBus<E, F> {
    subs: Arc<RwLock<BTreeMap<u64, Subscription<E, F>>>>,
    counter: Arc<AtomicU64>,
}

impl<E, F> Clone for EventBus<E, F> {
    fn clone(&self) -> Self {
        Self {
            subs: Arc::clone(&self.subs),
            counter: Arc::clone(&self.counter),
        }
    }
}

impl<E, F> Default for EventBus<E, F>
where
    E: Clone + Send + 'static,
    F: Filter<E> + Send + Sync + 'static,
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
            counter!(UNSUBSCRIBE_TOTAL).increment(1);
            gauge!(ACTIVE_SUBSCRIPTIONS).set(subs.len() as f64);
        }
    }
}

impl<E, F> EventBus<E, F>
where
    E: Clone + Send + 'static,
    F: Filter<E> + Send + Sync + 'static,
{
    /// Create a new, empty event bus.
    pub fn new() -> Self {
        register_metrics_once();
        Self {
            subs: Arc::new(RwLock::new(BTreeMap::new())),
            counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Add a new subscription using the provided filter.
    pub async fn add_subscription(&self, filter: F) -> SubscriptionHandle<E, F> {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::unbounded_channel::<E>();

        let sub = Subscription { filter, sender: tx };

        {
            let mut subs = self.subs.write().await;
            subs.insert(id, sub);
            gauge!(ACTIVE_SUBSCRIPTIONS).set(subs.len() as f64);
            counter!(SUBSCRIBE_TOTAL).increment(1);
        }

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
            tracing::warn!("unable to get tokio handle to publish event");
        }
    }

    /// Publish an event to all subscribers whose filters match.
    pub async fn notify(&self, event: E) {
        counter!(EVENTS_PUBLISHED_TOTAL).increment(1);

        let subs_guard = self.subs.read().await;
        let mut dead = Vec::new();

        for (id, sub) in subs_guard.iter() {
            if sub.filter.matches(&event) {
                if sub.sender.send(event.clone()).is_ok() {
                    counter!(EVENTS_DELIVERED_TOTAL).increment(1);
                } else {
                    dead.push(*id);
                }
            }
        }
        drop(subs_guard);

        for id in dead {
            counter!(EVENTS_DROPPED_TOTAL).increment(1);
            tracing::debug!("removing dead subscription {}", id);
            self.remove_subscription(id).await;
        }
    }

    /// Get the current number of active subscriptions.
    pub async fn subscription_count(&self) -> usize {
        self.subs.read().await.len()
    }
}

/// RAII subscription handle; dropping the last clone removes the subscription.
pub struct SubscriptionHandle<E, F>
where
    E: Send + 'static,
    F: Send + Sync + 'static,
{
    id: u64,
    rx: Arc<Mutex<mpsc::UnboundedReceiver<E>>>,
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
                    if let Ok(mut subs) = bus.subs.try_write() {
                        if subs.remove(&id).is_some() {
                            counter!(UNSUBSCRIBE_TOTAL).increment(1);
                            gauge!(ACTIVE_SUBSCRIPTIONS).set(subs.len() as f64);
                        }
                    }
                }
            }
        }
    }
}

// ---- Metrics ----
/// Gauge: current number of active event bus subscriptions.
const ACTIVE_SUBSCRIPTIONS: &str = "event_bus_active_subscriptions";
/// Counter: total subscriptions created on the event bus.
const SUBSCRIBE_TOTAL: &str = "event_bus_subscribe_total";
/// Counter: total subscriptions removed from the event bus.
const UNSUBSCRIBE_TOTAL: &str = "event_bus_unsubscribe_total";
/// Counter: total events published to the event bus.
const EVENTS_PUBLISHED_TOTAL: &str = "event_bus_events_published_total";
/// Counter: total events delivered to subscribers.
const EVENTS_DELIVERED_TOTAL: &str = "event_bus_events_delivered_total";
/// Counter: total events dropped due to dead subscribers.
const EVENTS_DROPPED_TOTAL: &str = "event_bus_events_dropped_total";

fn register_metrics_once() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[derive(Clone, Debug, PartialEq)]
    enum Evt {
        Num(u32),
    }

    #[derive(Clone)]
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
}
