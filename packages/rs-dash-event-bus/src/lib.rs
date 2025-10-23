//! rs-dash-event-bus: shared event bus and Platform events multiplexer
//!
//! - `event_bus`: generic in-process pub/sub with pluggable filtering
//! - `event_mux`: upstream bi-di gRPC multiplexer for Platform events

pub mod event_bus;

pub use event_bus::{EventBus, Filter, SubscriptionHandle};
