## Overview

Goal: introduce a reusable, generic event bus for rs-drive-abci. In this task, we only implement the generic bus itself (no integration into rs-drive-abci or rs-dapi yet). The bus must be non-blocking, memory-safe, support fine-grained filtering, perform automatic cleanup of dead subscribers, and be cheaply clonable.

Why now: rs-dapi already implements a subscription/dispatch layer in `packages/rs-dapi/src/services/streaming_service/subscriber_manager.rs`. It works, but it couples event routing to rs-dapi types, mixes Core/Tenderdash concerns, and duplicates logic we also need in rs-drive-abci (to publish platform-domain events). Centralizing a generic, minimal bus avoids divergence and lets both processes share the same subscription semantics.

Non-goals (for this task):
- Any integration with existing services (no changes to rs-drive-abci ABCI, rs-dapi streaming, or dapi-grpc protos).
- Cross-process pub/sub. The bus is in-process only.
- Persistent storage or replay. Real-time streaming only.

## Current State (rs-dapi)

Key parts to carry forward while generalizing:
- RAII subscription handles with auto-cleanup when the client drops the stream. See `packages/rs-dapi/src/services/streaming_service/subscriber_manager.rs:34` and the `Drop` impl for `SubscriptionHandleInner` that removes the sub from the map on drop.
- Event dispatch loop that fans out to matching subscribers and prunes dead senders. See `notify()` in the same file.
- Mapping/sub-stream helpers (`map`, `filter_map`) to transform subscription payloads without re-subscribing.

Limitations we will address (at the bus level):
- Coupled filter matching: `SubscriberManager` knows all `FilterType` variants and dispatch rules. This prevents reuse with other event types (platform domain events in drive-abci).
- Mixed concerns: current `FilterType` includes Core bloom filters, masternode updates, Platform TX events, etc. The bus should be generic; crates define their own filters and implement matching.
- Unbounded subscriber channels: today we use `tokio::mpsc::UnboundedSender`. We should keep this initially (to match existing behavior) but design for optionally bounded channels and drop policy.

## Design

### Core abstraction

- `EventBus<E, F>`: a generic subscription manager where:
  - `E: Clone + Send + 'static` is the event type.
  - `F: Filter<E> + Send + Sync + 'static` encapsulates event matching.

- `Filter<E>` trait: single method `fn matches(&self, event: &E) -> bool`.

- `SubscriptionHandle<E>`: RAII handle with `recv().await -> Option<E>` and helper `map`/`filter_map` to derive transformed streams without messing with the underlying subscription lifecycle. Dropping the last handle removes the underlying subscription.

Constraints for this task:
- Implementation uses `tokio` primitives only and `BTreeMap` for subscriber registry (ordered, predictable iteration).
- Cheap cloning: `EventBus` holds Arcs for its shared fields (registry and counter), so `Clone` is O(1). No external locking is required by callers; all synchronization lives inside the bus.
- Public API exposes async methods; internal synchronization uses `tokio::sync::{RwLock, mpsc}` and `std::sync::atomic::AtomicU64` for IDs.

This mirrors the existing API shape but removes knowledge of specific filters/events from the bus. Matching is delegated to `F`.

### Module placement and reuse

- Implement the generic bus in `packages/rs-drive-abci/src/event_bus/` and re-export as `drive_abci::event_bus`.
- We will not wire it anywhere in this task. Future work can integrate it into rs-drive-abci and rs-dapi.

### Event namespaces (deferred)

The bus is event-agnostic. Concrete `E` and `F` types will be defined by integrating crates later:
- rs-dapi: `StreamingEvent`, `StreamingFilter` (deferred).
- rs-drive-abci: `PlatformEvent`, `PlatformFilter` (deferred).

### Platform events (deferred)

Defining the specific PlatformEvent set and gRPC messages is out of scope for this task and will be handled during integration.

### Filtering model

The bus only depends on the `Filter<E>` trait with `matches(&self, &E) -> bool`. Any persistence or stateful matching (e.g., bloom filter updates) lives in the filter implementation, not in the bus. For this task we only provide the trait and generic bus.

### gRPC API (deferred)

No protobuf or gRPC changes in this task. We will add a streaming RPC in a later integration phase.

### Backpressure, ordering, and observability

- Ordering: within a bus instance, events are delivered in the order they are published.
- Channels: start with `tokio::mpsc::unbounded_channel` for simplicity; the internal design allows swapping to bounded channels later without breaking the public API.
- Metrics (via `metrics` crate; picked up by the existing Prometheus exporter):
  - `event_bus_active_subscriptions` (gauge)
  - `event_bus_subscribe_total` (counter)
  - `event_bus_unsubscribe_total` (counter)
  - `event_bus_events_published_total` (counter)
  - `event_bus_events_delivered_total` (counter)
  - `event_bus_events_dropped_total` (counter)

## API Sketch (Rust)

Trait and types to be added under `drive_abci::event_bus`:

```
pub trait Filter<E>: Send + Sync {
    fn matches(&self, event: &E) -> bool;
}

pub struct EventBus<E, F> { /* clonable; internal Arcs */ }

impl<E: Clone + Send + 'static, F: Filter<E> + Send + Sync + 'static> EventBus<E, F> {
    pub fn new() -> Self;
    pub async fn add_subscription(&self, filter: F) -> SubscriptionHandle<E, F>;
    pub async fn notify(&self, event: E);
    pub async fn remove_subscription(&self, id: u64);
    pub async fn subscription_count(&self) -> usize;
}

pub struct SubscriptionHandle<E, F> { /* recv(); RAII removal on Drop */ }
```

Notes on internals for this task:
- Use `BTreeMap<u64, Subscription>` for the registry; IDs generated by `AtomicU64`.
- Protect the registry with `tokio::sync::RwLock`.
- EventBus holds `Arc<RwLock<_>>` for the registry and `Arc<AtomicU64>` for the counter; `Clone` is O(1).
- `Subscription` holds a `filter: F` and an `mpsc::UnboundedSender<E>`.
- `SubscriptionHandle` holds the subscription `id`, a guarded `mpsc::UnboundedReceiver<E>`, and a clone of the `EventBus` to perform removal on drop.
- `Drop` for `SubscriptionHandle` spawns a thread and executes async `remove_subscription(id)` on a Tokio runtime to keep `Drop` non-async.

## Scope for This Task

1) Introduce `packages/rs-drive-abci/src/event_bus/` with the generic `EventBus<E, F>` and `Filter<E>` trait.
2) Implement RAII `SubscriptionHandle` with `recv`, `map`, and `filter_map` helpers.
3) Use `BTreeMap` + `tokio::RwLock` internally; expose a cheap `Clone` for `EventBus`.
4) Keep channels unbounded; prune dead subscribers on send failure.
5) Add unit tests demonstrating basic usage.
6) Instrument with Prometheus-compatible metrics via the `metrics` crate, without adding any exporter code or changing `metrics.rs`.

### Metrics Integration (This Task)

- Mechanism: use the existing `metrics` crate macros (`counter!`, `gauge!`, `describe_*`) so the already-installed Prometheus exporter in rs-drive-abci (`metrics::Prometheus::new(...)`) picks them up automatically.
- Registration: in `EventBus::new()`, call a `register_metrics_once()` function guarded by `Once` to `describe_*` the keys below. No changes to `packages/rs-drive-abci/src/metrics.rs` are required.
- Metrics (no labels initially; labels can be added later if we add a label-provider hook):
  - `event_bus_active_subscriptions` (gauge): current number of active subscriptions.
  - `event_bus_subscribe_total` (counter): increments on each new subscription creation.
  - `event_bus_unsubscribe_total` (counter): increments when a subscription is removed (explicitly or via RAII drop).
  - `event_bus_events_published_total` (counter): increments for each `notify()` call.
  - `event_bus_events_delivered_total` (counter): increments for each event successfully delivered to a subscriber.
  - `event_bus_events_dropped_total` (counter): increments when delivery to a subscriber fails and the subscriber is pruned.

Notes:
- Minimizes changes to rs-drive-abci by keeping metric registration local to the bus module. The existing exporter remains untouched.
- rs-dapi can freely depend on the bus; if no exporter is installed in that process, metrics calls are no-ops. If an exporter is added later, the same keys will be reported.

Optional future enhancement:
- Add an optional, generic label-provider closure on `EventBus` creation, e.g. `with_metrics_labels(fn(&F)->Vec<metrics::Label>)`, to tag counts by filter type or namespace without coupling the bus to concrete filter/event types.

## Example Usage (Test)

Minimal demonstration to include as a unit test in the new module:

```
#[tokio::test]
async fn basic_subscribe_and_notify() {
    #[derive(Clone)]
    enum E { Num(u32) }
    struct EvenOnly;
    impl Filter<E> for EvenOnly {
        fn matches(&self, e: &E) -> bool { matches!(e, E::Num(n) if n % 2 == 0) }
    }

    let bus = EventBus::<E, EvenOnly>::new();
    let sub = bus.add_subscription(EvenOnly).await;

    bus.notify(E::Num(1)).await; // filtered out
    bus.notify(E::Num(2)).await; // delivered

    let got = sub.recv().await.unwrap();
    if let E::Num(n) = got { assert_eq!(n, 2); } else { unreachable!() }
}
```

Additional tests (optional):
- Dropping the `SubscriptionHandle` removes the subscription (count decreases).

## Risks and Mitigations

- Heavy dependency of rs-dapi on rs-drive-abci: we keep the event bus module isolated with no external deps so it can be extracted to a separate small crate later with no API churn.
- Unbounded channels: acceptable for now; we will monitor and can swap to bounded channels later without public API changes.

## TODOs

- Core bus (this task)
  - [x] Create `packages/rs-drive-abci/src/event_bus/mod.rs` with generic `EventBus<E,F>` and `Filter<E>`.
  - [x] Implement internal registry with `BTreeMap<u64, Subscription>` and `tokio::RwLock`.
  - [x] Add RAII `SubscriptionHandle` with `recv` and auto-removal on drop.
  - [x] Implement `add_subscription`, `notify`, `subscription_count` and dead-subscriber pruning.
  - [x] Ensure `EventBus` is `Clone` (cheap) and requires no external locking by callers.
  - [x] Add unit tests: basic subscribe/notify, drop removes sub.
  - [x] Add metrics: register metrics once; update counters/gauges in `add_subscription`, removal/drop, and `notify()` paths.

Implementation Note
- `SubscriptionHandle<E, F>` has bounds `E: Send + 'static`, `F: Send + Sync + 'static`. The drop logic spawns a dedicated thread and runs `EventBus::remove_subscription(id)` on a Tokio runtime in that thread to perform async cleanup from `Drop` safely.

- Deferred integration (future tasks)
  - Define concrete event/filter types in rs-drive-abci and rs-dapi; implement `Filter<E>` for each.
  - Replace rs-dapi `SubscriberManager` with the generic bus.
  - Add gRPC streaming endpoint(s) in dapi-grpc and wire to the bus.
  - Add metrics and configurable backpressure.
