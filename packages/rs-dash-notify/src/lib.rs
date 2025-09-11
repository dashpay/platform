//! rs-dash-notify: shared event bus and Platform events multiplexer
//!
//! - `event_bus`: generic in-process pub/sub with pluggable filtering
//! - `platform_mux`: upstream bi-di gRPC multiplexer for Platform events

pub mod event_bus;
pub mod event_mux;
pub mod grpc_producer;
pub mod local_bus_producer;

pub use ::sender_sink::wrappers::{SinkError, UnboundedSenderSink};
pub use event_bus::{EventBus, Filter, SubscriptionHandle};
pub use event_mux::{EventMux, EventProducer, EventSubscriber, PlatformEventsSubscriptionHandle};
pub use grpc_producer::GrpcPlatformEventsProducer;
pub use local_bus_producer::run_local_platform_events_producer;
