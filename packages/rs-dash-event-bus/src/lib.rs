//! rs-dash-event-bus: shared event bus utilities for Dash Platform components.
//!
//! - `event_bus`: generic in-process pub/sub with pluggable filtering

pub mod event_bus;

pub use event_bus::{EventBus, Filter, SubscriptionHandle};
