//! rs-dash-notify: shared event bus and Platform events multiplexer
//!
//! - `event_bus`: generic in-process pub/sub with pluggable filtering
//! - `platform_mux`: upstream bi-di gRPC multiplexer for Platform events

pub mod event_bus;
pub mod platform_mux;

pub use event_bus::{EventBus, Filter, SubscriptionHandle};
pub use platform_mux::{PlatformEventsMux, PlatformEventsSession, PlatformMuxSettings};
