//! rs-dash-event-bus: shared event bus and Platform events multiplexer
//!
//! - `event_bus`: generic in-process pub/sub with pluggable filtering
//! - `platform_mux`: upstream bi-di gRPC multiplexer for Platform events

pub mod event_bus;
pub mod event_mux;
pub mod grpc_producer;
pub mod local_bus_producer;

pub use event_bus::{EventBus, Filter, SubscriptionHandle};
pub use event_mux::{
    EventMux, EventProducer, EventSubscriber, PlatformEventsSubscriptionHandle, result_sender_sink,
    sender_sink,
};
pub use grpc_producer::GrpcPlatformEventsProducer;
pub use local_bus_producer::run_local_platform_events_producer;
