## Overview

Goal: extract the eventing stack into a dedicated reusable crate `packages/rs-dash-notify` and make rs-dapi, rs-drive-abci, and rs-sdk consume it. The crate provides a generic, non-blocking, memory-safe in-process event bus and a Platform events multiplexer that speaks the existing bi-directional gRPC API. The bus supports fine-grained filtering, automatic cleanup of dead subscribers, and cheap cloning; the mux manages upstream Drive ABCI connections using `AddressList`.

Why now: rs-dapi contains a subscription/dispatch layer (`packages/rs-dapi/src/services/streaming_service/subscriber_manager.rs`) and a Platform events multiplexer (`packages/rs-dapi/src/services/platform_service/subscribe_platform_events.rs`). rs-drive-abci contains a separate in-process bus for publishing Platform-domain events. This duplicates logic and couples implementations to crate-local types. Centralizing into `rs-dash-notify` avoids divergence, lets all processes share subscription semantics, and reduces maintenance.

Non-goals:
- Cross-process pub/sub beyond one process (cross-process streaming remains gRPC via Drive ABCI).
- Persistent storage or replay. Real-time streaming only.

## Current State (before extraction)

Key parts to carry forward while generalizing:
- RAII subscription handles with auto-cleanup when the client drops the stream. See `packages/rs-dapi/src/services/streaming_service/subscriber_manager.rs:34` and the `Drop` impl for `SubscriptionHandleInner` that removes the sub from the map on drop.
- Event dispatch loop that fans out to matching subscribers and prunes dead senders. See `notify()` in the same file.
- Mapping/sub-stream helpers (`map`, `filter_map`) to transform subscription payloads without re-subscribing.

Limitations we will address (at the crate level):
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

- Extracted into `packages/rs-dash-notify` (library crate). Public surface:
  - `event_bus`: generic in-process `EventBus<E, F>` and `Filter<E>` trait.
  - `platform_mux`: `PlatformEventsMux` for upstream Drive ABCI subscription multiplexing.
- rs-drive-abci publishes Platform events using `rs_dash_notify::event_bus` and protobuf-generated types.
- rs-dapi uses `rs_dash_notify::platform_mux::PlatformEventsMux` to proxy public subscriptions to Drive ABCI.
- rs-sdk exposes a simple wrapper, e.g. `Sdk::subscribe(...)`, built on top of the mux.

### Event namespaces (deferred)

The bus is event-agnostic. Concrete `E` and `F` types will be defined by integrating crates later:
- rs-dapi: `StreamingEvent`, `StreamingFilter` (deferred).
- rs-drive-abci: `PlatformEvent`, `PlatformFilter` (deferred).

### Platform events

`PlatformEvent` and `PlatformFilterV0` come from protobuf-generated types in `dapi-grpc`. The crate avoids custom wrappers unless necessary; adapters only bridge protobuf filters to the `Filter<E>` trait for the in-process bus.

### Filtering model

The bus only depends on the `Filter<E>` trait with `matches(&self, &E) -> bool`. Any persistence or stateful matching (e.g., bloom filter updates) lives in the filter implementation, not in the bus. For this task we only provide the trait and generic bus.

### gRPC API

Bi-directional streaming RPC continues to support multiplexed subscriptions over a single connection between rs-dapi and rs-drive-abci. The new mux in `rs-dash-notify` encapsulates this logic and connection pooling.

### Subscription Server (gRPC)

A single bi-directional streaming RPC allows a client to open one connection to Drive ABCI, then add and remove multiple logical subscriptions. Server pushes events tagged with the logical subscription ID. The server-side publisher in rs-drive-abci uses the shared in-process bus from `rs-dash-notify`.

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

### Metrics Integration

- Mechanism: use the existing `metrics` crate macros (`counter!`, `gauge!`, `describe_*`) gated behind the crate feature `metrics`. When enabled, the already-installed Prometheus exporters (as in rs-dapi and rs-drive-abci) pick them up automatically.
- Registration: in `EventBus::new()`, call a `register_metrics_once()` function guarded by `Once` to `describe_*` the keys below. No changes to `packages/rs-drive-abci/src/metrics.rs` are required.
- Metrics (no labels initially; labels can be added later if we add a label-provider hook):
  - `event_bus_active_subscriptions` (gauge): current number of active subscriptions.
  - `event_bus_subscribe_total` (counter): increments on each new subscription creation.
  - `event_bus_unsubscribe_total` (counter): increments when a subscription is removed (explicitly or via RAII drop).
  - `event_bus_events_published_total` (counter): increments for each `notify()` call.
  - `event_bus_events_delivered_total` (counter): increments for each event successfully delivered to a subscriber.
  - `event_bus_events_dropped_total` (counter): increments when delivery to a subscriber fails and the subscriber is pruned.

Notes:
- Registration lives in the shared crate (bus and mux modules). Exporters in consuming processes remain untouched.
- If no exporter is installed, metrics calls are no-ops.

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

## New Architecture

- Shared crate: `packages/rs-dash-notify`
  - `event_bus`: generic bus and tests (async subscribe/notify, RAII cleanup, optional metrics, extensive `tracing` logging).
- `platform_mux`: upstream connection pool for Drive ABCI bi-di stream built on top of the shared EventBus. It uses protobuf types end-to-end, requires UUID `client_subscription_id` (pass-through across layers), and provides `PlatformEventsMux::new(addresses: rs_dapi_client::AddressList, settings: PlatformMuxSettings)`.
  - Feature flags: `metrics` enables Prometheus-compatible instrumentation via `metrics` crate.

- Drive ABCI server endpoint (consumer of the bus)
  - Uses `rs_dash_notify::event_bus::EventBus<PlatformEvent, PlatformFilterAdapter>`.
  - Connection-local routing map stores `client_subscription_id -> SubscriptionHandle` and forwards events to the response stream.
  - Handles `Add`, `Remove`, `Ping` with ACK/error responses using protobuf-generated types.

- rs-dapi proxy (consumer of the mux)
  - Replaces in-repo mux with `rs_dash_notify::platform_mux::PlatformEventsMux`.
  - Per-client sessions bind to an upstream connection; `client_subscription_id` (UUID) is preserved across all layers; `Ping` handled locally.
  - Command loop processing moved into the shared crate via `spawn_client_command_processor(session, inbound, out_tx)`.
  - Optional metrics via `metrics` feature; logs via `tracing` with structured context.

## Risks and Mitigations

- Heavy dependency of rs-dapi on rs-drive-abci: we keep the event bus module isolated with no external deps so it can be extracted to a separate small crate later with no API churn.
- Unbounded channels: acceptable for now; we will monitor and can swap to bounded channels later without public API changes.

## TODOs

- New crate: `packages/rs-dash-notify`
  - [x] Create library crate with `event_bus` and `platform_mux` modules.
  - [x] Move `packages/rs-drive-abci/src/event_bus/mod.rs` into `event_bus` with minimal API changes; convert local paths to crate paths.
  - [x] Add `tracing` logs throughout (subscribe, notify, drop, mux connect, route, error paths).
  - [x] Gate metrics behind `features = ["metrics"]`; reuse existing metric keys; register once via `Once`.
  - [x] Implement `PlatformEventsMux::new(addresses: rs_dapi_client::AddressList, settings: PlatformMuxSettings)`; reuse protobuf types from `dapi-grpc` end-to-end.
  - [x] Provide graceful shutdown in mux (cancellable via CancellationToken).
  - [x] Use EventBus internally in `platform_mux` for response fan-out and id-based filtering.

- rs-dapi integration
  - [x] Replace `services/platform_service/subscribe_platform_events.rs` with calls into `rs-dash-notify::platform_mux`.
  - [ ] Remove `streaming_service/subscriber_manager.rs` where duplicated; use bus/mux from the crate.
  - [ ] Wire `tracing` spans and enable `metrics` feature as needed.

- rs-drive-abci integration
  - [x] Replace duplicate event handling with `rs-dash-notify::event_bus`.
  - [x] Use protobuf-generated types directly (no custom wrappers).
  - [x] Ensure server method uses the shared bus; keep filter adapter minimal.

- rs-sdk integration
  - [ ] Expose convenience APIs, e.g. `Sdk::subscribe(filter) -> Stream<PlatformEventsResponse>` using `PlatformEventsMux`.
  - [ ] Accept `AddressList` in SDK builder and plumb to mux.
  - [ ] Generate UUID `client_subscription_id` in SDK and keep it unchanged across layers; align downstream channel type with shared mux.
  - [ ] Update or remove `packages/rs-sdk/examples/platform_events.rs` to match the actual SDK API (currently refers to missing `platform::events` types).

- Docs and tests
  - [ ] Update rs-dapi DESIGN.md to reflect shared crate usage.
  - [ ] Add unit/integration tests for mux routing and ID rewrite.
  - [ ] Add examples in `rs-sdk/examples/platform_events.rs` using the new wrapper.

Implementation Note
- `SubscriptionHandle<E, F>` retains bounds `E: Send + 'static`, `F: Send + Sync + 'static`. Remove-on-drop prefers `tokio::spawn` (if a runtime is present) or best-effort synchronous removal via `try_write()`.
