## Overview

Goal: introduce a reusable, generic event bus for rs-drive-abci and wire it into the new Platform events subscription flow used by rs-dapi. The bus must be non-blocking, memory-safe, support fine-grained filtering, perform automatic cleanup of dead subscribers, and be cheaply clonable.

Why now: rs-dapi already implements a subscription/dispatch layer in `packages/rs-dapi/src/services/streaming_service/subscriber_manager.rs`. It works, but it couples event routing to rs-dapi types, mixes Core/Tenderdash concerns, and duplicates logic we also need in rs-drive-abci (to publish platform-domain events). Centralizing a generic, minimal bus avoids divergence and lets both processes share the same subscription semantics.

Non-goals:
- Cross-process pub/sub beyond one process (rs-dapi ↔ rs-drive-abci use gRPC, not a shared in-memory bus).
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

- Implemented generic bus in `packages/rs-drive-abci/src/event_bus/` and re-exported as `drive_abci::event_bus`.
- Wired into drive-abci `subscribePlatformEvents` and proxied in rs-dapi.

### Event namespaces (deferred)

The bus is event-agnostic. Concrete `E` and `F` types will be defined by integrating crates later:
- rs-dapi: `StreamingEvent`, `StreamingFilter` (deferred).
- rs-drive-abci: `PlatformEvent`, `PlatformFilter` (deferred).

### Platform events

`PlatformEvent` messages and `PlatformFilterV0` are part of the public gRPC API. Drive-abci adapts incoming gRPC filters to the internal `EventBus` via `PlatformFilterAdapter`.

### Filtering model

The bus only depends on the `Filter<E>` trait with `matches(&self, &E) -> bool`. Any persistence or stateful matching (e.g., bloom filter updates) lives in the filter implementation, not in the bus. For this task we only provide the trait and generic bus.

### gRPC API

Bi-directional streaming RPC supports multiplexed subscriptions over a single connection between rs-dapi and rs-drive-abci.

### Subscription Server (gRPC)

A single bi-directional streaming RPC allows a client (rs-dapi) to open one connection to rs-drive-abci, then add and remove multiple logical subscriptions over that connection. Server pushes events tagged with the logical subscription ID.

- New RPC in `platform.proto`:
  - `rpc subscribePlatformEvents(stream PlatformEventsCommand) returns (stream PlatformEventsResponse);`

- Commands from client (rs-dapi) to server (rs-drive-abci):
  - `AddSubscription`: `{ client_subscription_id: string, filter: PlatformFilter }`
  - `RemoveSubscription`: `{ client_subscription_id: string }`
  - Optional `Ping`: keepalive/latency measurement.

- Responses from server to client:
  - `Event`: `{ client_subscription_id: string, event: PlatformEvent }`
  - `Ack`: `{ client_subscription_id: string, op: Add|Remove }` (optional, for command confirmation)
  - `Error`: `{ client_subscription_id: string, code: uint32, message: string }`

- Versioning: wrap `PlatformEventsCommand` and `PlatformEventsResponse` in standard versioned envelopes, e.g. `oneof version { v0: ... }`, consistent with other Platform RPCs.

- Types to add to `platform.proto` (v0):
  - `message PlatformEventsCommandV0 { oneof command { AddSubscription add = 1; RemoveSubscription remove = 2; Ping ping = 3; } }`
  - `message AddSubscription { string client_subscription_id = 1; PlatformFilter filter = 2; }`
  - `message RemoveSubscription { string client_subscription_id = 1; }`
  - `message Ping { uint64 nonce = 1; }`
  - `message PlatformEventsResponseV0 { oneof response { Event event = 1; Ack ack = 2; Error error = 3; } }`
  - `message Event { string client_subscription_id = 1; PlatformEvent event = 2; }`
  - `message Ack { string client_subscription_id = 1; string op = 2; }`
  - `message Error { string client_subscription_id = 1; uint32 code = 2; string message = 3; }`
  - `message PlatformFilter { /* initial variants for platform-side filtering; see Filtering model */ }`
  - `message PlatformEvent { /* initial variants for platform events; see above */ }`

Server behavior (rs-drive-abci):
- No separate manager type is required. Within the RPC handler task for a connection:
  - Maintain a simple connection-local map: `client_subscription_id -> SubscriptionHandle`.
  - Process incoming `PlatformEventsCommand` frames: on `AddSubscription`, subscribe to the global in-process `EventBus<PlatformEvent, PlatformFilter>` and store the handle in the map; on `RemoveSubscription`, drop the handle and remove the map entry.
  - For each added subscription, spawn a lightweight forwarder that awaits `handle.recv()` and pushes `Event { client_subscription_id, event }` into the single per-connection response sender.
  - On disconnect, drop all handles (RAII removes bus subscriptions) and end the response stream.
  - Optionally respond with `Ack`/`Error` for command results.

Optional metadata in EventBus:
- If we later need bulk cancellation by connection without keeping a map, we can extend the bus with opaque metadata stored alongside each subscription (e.g., `connection_id`). That would allow calling a `remove_by_tag(connection_id)` helper. For now, a connection-local map is sufficient and minimizes changes to the bus.

rs-dapi proxy:
- Maintain one persistent bi-directional stream to rs-drive-abci and multiplex all client (public) subscriptions over it:
  - Public gRPC: expose `subscribePlatformEvents` (server-streaming) with a simple request carrying `PlatformFilter` and a generated `client_subscription_id` per public subscriber.
  - On new public subscriber: send `AddSubscription` upstream with a unique `client_subscription_id`, route all `Event` frames matching that ID back to the public subscriber’s stream.
  - On public stream drop: send `RemoveSubscription` upstream and clean up the routing entry.
  - Reconnection: on upstream disconnect, re-establish the connection and re-add active subscriptions. Document at‑least‑once delivery and potential gaps during reconnection.

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

Notes on internals:
- Use `BTreeMap<u64, Subscription>` for the registry; IDs generated by `AtomicU64`.
- Protect the registry with `tokio::sync::RwLock`.
- EventBus holds `Arc<RwLock<_>>` for the registry and `Arc<AtomicU64>` for the counter; `Clone` is O(1).
- `Subscription` holds a `filter: F` and an `mpsc::UnboundedSender<E>`.
- `SubscriptionHandle` holds the subscription `id`, a guarded `mpsc::UnboundedReceiver<E>`, and a clone of the `EventBus` to perform removal on drop.
- `Drop` for `SubscriptionHandle` removes the subscription when the last handle is dropped, preferring `tokio::spawn` if a runtime is available and falling back to a best-effort synchronous removal via `try_write()`.

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

## Implemented

- Generic bus and tests
  - `packages/rs-drive-abci/src/event_bus/mod.rs:1`
  - Async subscribe/notify, RAII cleanup, metrics counters/gauges, unit tests.

- Drive ABCI server endpoint
  - `packages/rs-drive-abci/src/query/service.rs:854`
  - Implements `subscribePlatformEvents` using `EventBus<PlatformEvent, PlatformFilterAdapter>`.
  - Connection-local routing map stores `client_subscription_id -> SubscriptionHandle` and forwards events to a per-connection sender feeding the response stream.
  - Handles `Add`, `Remove`, and `Ping` with ACK/error responses.

- Filter adapter in drive-abci
  - `packages/rs-drive-abci/src/query/service.rs:260`
  - `PlatformFilterAdapter` implements `event_bus::Filter<PlatformEvent>` by delegating to `PlatformFilterV0` kinds.
  - Current semantics:
    - `All(true)`: match all events; `All(false)` matches none.
    - `TxHash(h)`: matches only `StateTransitionResult` events where `tx_hash == h`.

- DAPI proxy
  - `packages/rs-dapi/src/services/platform_service/mod.rs:1`
  - `packages/rs-dapi/src/services/platform_service/subscribe_platform_events.rs:1`
  - Maintains a small pool of upstream bi-di connections to drive-abci (currently `UPSTREAM_CONN_COUNT = 2`).
  - Per-client session assigns a unique upstream subscription id prefix per chosen upstream and rewrites IDs so multiple public subscriptions share one upstream stream.
  - Routes upstream events/acks/errors back to the original public `client_subscription_id`.
  - Handles local `Ping` without forwarding upstream.
  - Metrics (Prometheus via rs-dapi):
    - `rsdapi_platform_events_active_sessions` (gauge)
    - `rsdapi_platform_events_commands_total{op}` (counter; op=add|remove|ping|invalid|invalid_version|stream_error)
    - `rsdapi_platform_events_forwarded_events_total` (counter)
    - `rsdapi_platform_events_forwarded_acks_total` (counter)
    - `rsdapi_platform_events_forwarded_errors_total` (counter)
    - `rsdapi_platform_events_upstream_streams_total` (counter)

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
  - [x] Fix Drop cleanup path: prefer `tokio::spawn` (when a runtime is present) or synchronous removal via `try_write()`.

Implementation Note
- `SubscriptionHandle<E, F>` has bounds `E: Send + 'static`, `F: Send + Sync + 'static`. The drop logic must not depend on an async closure inside `std::thread::spawn` (which won’t be awaited). Use `tokio::spawn` if `Handle::try_current()` succeeds, or remove synchronously with a non-async write when possible. See the TODO above.

- Deferred integration (future tasks)
  - Define concrete event/filter types in rs-drive-abci and rs-dapi; implement `Filter<E>` for each.
  - Replace rs-dapi `SubscriberManager` with the generic bus.
  - Add metrics and configurable backpressure.

- New: Subscription server and proxying
  - [x] Update `packages/dapi-grpc/protos/platform/v0/platform.proto` with `subscribePlatformEvents` bi-di stream and new messages (Commands/Responses, PlatformFilter, PlatformEvent) under `v0`.
  - [x] Regenerate dapi-grpc code and update dependent crates.
  - [x] Implement `subscribePlatformEvents` in rs-drive-abci:
    - [x] Connection-local routing map (`client_subscription_id -> SubscriptionHandle`).
    - [x] Forwarder tasks per subscription push events into a per-connection sender feeding the response stream.
    - [x] Handle `AddSubscription`, `RemoveSubscription`, `Ping`, and clean disconnect.
    - [ ] Instrument metrics (connections, logical subs, commands, acks/errors, events forwarded).
  - [x] Implement rs-dapi proxy:
    - [x] Upstream connection pool (const size = 2; extensible; no reconnect yet).
    - [x] Public DAPI `subscribePlatformEvents` (server-streaming) that allocates `client_subscription_id`s and routes events.
    - [x] Removal on client drop and upstream `RemoveSubscription`.
    - [x] Metrics for public subs and routing.
