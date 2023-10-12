//! # Dash Platform Rust SDK
//!
//! This is the official Rust SDK for the Dash Platform. Dash Platform is a Layer 2 cryptocurrency technology that
//! builds upon the Dash layer 1 network. This SDK provides an abstraction layer to simplify usage of the Dash
//! Platform along with data models based on the Dash Platform Protocol (DPP), a CRUD interface, and bindings
//! for other technologies such as C.
//!
//!
//! ## Dash Platform Protocol Data Model
//!
//! SDK data model uses types defined in [Dash Platform Protocol (DPP)](crate::platform::dpp). At this point, the following
//! types are supported:
//!
//! 1. [`Identity`](crate::platform::Identity)
//! 2. [`Data Contract`](crate::platform::DataContract)
//! 3. [`Document`](crate::platform::Document)
//!
//! To define document search conditions, [`DriveQuery`](crate::platform::DriveQuery) is implemented and can be easily
//! converted to [`DocumentQuery`](crate::platform::DocumentQuery) with the [`From`] trait.
//!
//! Basic DPP objects are re-exported in the [`platform`] module.
//!
//! ## CRUD Interface
//!
//! Operations on data model objects can be executing using traits following CRUD (Create, Read, Update, and Delete)
//! approach. The following traits are already implemented:
//!
//! 1. [`Fetch`](crate::platform::Fetch)
//! 2. [`List`](crate::platform::List)
//!
//! Fetch and List traits return objects based on provided queries. Some example queries include:
//!
//! 1. [`Identifier`](crate::platform::Identifier) - fetches an object by its identifier
//! 2. [`DriveQuery`](crate::platform::DriveQuery) - lists objects based on search conditions; see
//! [query syntax documentation](https://docs.dash.org/projects/platform/en/stable/docs/reference/query-syntax.html)
//! for more details.
//!
//! ## Testability
//!
//! SDK operations can be mocked using [Sdk::new_mock()].
//!
//! Examples can be found in `tests/mock_*.rs`.
//!
//! ## Error handling
//!
//! Errors of type [Error] are returned by the rs-sdk. Note that missing objects ("not found") are not
//! treated as errors; `Ok(None)` is returned instead.
//!
//! Mocking functions often panic instead of returning an error.
//!
//! ## Logging
//!
//! This project uses the `tracing` crate for instrumentation and logging. The `tracing` ecosystem provides a powerful,
//! flexible framework for adding structured, context-aware logs to your program.
//!
//! To enable logging, you can use the `tracing_subscriber` crate which allows applications to customize how events are processed and recorded.
//! An example can be found in `tests/common.rs:setup_logs()`.

#[deny(missing_docs)]
pub mod platform;

pub mod error;

pub mod core;

pub mod sdk;

#[cfg(feature = "mocks")]
pub mod mock;
pub use error::Error;
pub use sdk::{Sdk, SdkBuilder};
