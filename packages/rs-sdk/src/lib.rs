//! # Dash Platform Rust SDK
//!
//! This is the official Rust SDK for the Dash Platform. Dash Platform is a Layer 2 cryptocurrency technology that
//! builds upon the Dash layer 1 network. This SDK provides an abstraction layer to simplify usage of the Dash
//! Platform along with data models based on the Dash Platform Protocol (DPP), query traits for reading data, and
//! state transition traits for writing data.
//!
//!
//! ## Dash Platform Protocol Data Model
//!
//! SDK data model uses types defined in [Dash Platform Protocol (DPP)](crate::platform::dpp). The core types are:
//!
//! 1. [`Identity`](crate::platform::Identity)
//! 2. [`DataContract`](crate::platform::DataContract)
//! 3. [`Document`](crate::platform::Document)
//!
//! To define document search conditions, you can use [`DriveDocumentQuery`](crate::platform::DriveDocumentQuery) and convert it
//! to [`DocumentQuery`](crate::platform::DocumentQuery) with the [`From`] trait.
//!
//! Basic DPP objects are re-exported in the [`platform`] module.
//!
//! ## Querying (Reading Data)
//!
//! Data can be read from Platform using the following traits:
//!
//! 1. [`Fetch`](crate::platform::Fetch) - fetch a single object from Platform with proof verification
//! 2. [`FetchMany`](crate::platform::FetchMany) - fetch multiple objects from Platform with proof verification
//! 3. [`FetchUnproved`](crate::platform::FetchUnproved) - fetch a single object without proof verification
//!    (e.g., for node status queries)
//! 4. [`FetchCurrent`](crate::platform::fetch_current_no_parameters::FetchCurrent) - fetch current
//!    state for parameter-free queries (e.g., current epoch, total credits)
//!
//! These traits return objects based on provided queries. Some example queries include:
//!
//! 1. [`Identifier`](crate::platform::Identifier) - fetches an object by its identifier
//! 2. [`DocumentQuery`](crate::platform::DocumentQuery) - fetches documents based on search conditions; see
//!    [query syntax documentation](https://docs.dash.org/projects/platform/en/stable/docs/reference/query-syntax.html)
//!    for more details.
//! 3. [`DriveDocumentQuery`](crate::platform::DriveDocumentQuery) - can be used to build more complex queries
//!
//! ## State Transitions (Writing Data)
//!
//! Data is written to Platform by broadcasting state transitions. The SDK provides traits for common
//! write operations:
//!
//! - [`PutIdentity`](crate::platform::transition::put_identity::PutIdentity) - register a new identity
//! - [`PutContract`](crate::platform::transition::put_contract::PutContract) - publish a data contract
//! - [`PutDocument`](crate::platform::transition::put_document::PutDocument) - create or replace a document
//! - [`TransferToIdentity`](crate::platform::transition::transfer::TransferToIdentity) - transfer credits between identities
//! - [`WithdrawFromIdentity`](crate::platform::transition::withdraw_from_identity::WithdrawFromIdentity) - withdraw credits
//! - [`TopUpIdentity`](crate::platform::transition::top_up_identity::TopUpIdentity) - add credits to an identity
//! - [`PutVote`](crate::platform::transition::vote::PutVote) - cast a masternode vote
//!
//! For document operations, builder-based APIs are also available (see [`platform::documents::transitions`]).
//!
//! ## Testability
//!
//! SDK operations can be mocked using [Sdk::new_mock()].
//!
//! Examples can be found in `tests/fetch/mock_fetch.rs` and `tests/fetch/mock_fetch_many.rs`.
//!
//! ## Error handling
//!
//! Errors of type [Error] are returned by the SDK. Note that missing objects ("not found") are not
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
//! An example can be found in `tests/fetch/common.rs:setup_logs()`.
//!
// TODO re-enable when docs are complete
// #![warn(missing_docs)]
#![allow(rustdoc::private_intra_doc_links)]
#![allow(clippy::result_large_err)]

pub mod core;
pub mod error;
mod internal_cache;
pub mod mock;
pub mod platform;
pub mod sdk;

pub use error::Error;
pub use sdk::{RequestSettings, Sdk, SdkBuilder};

pub use dapi_grpc;
pub use dpp;
#[cfg(feature = "core_spv")]
pub use dpp::dash_spv;
#[cfg(feature = "core_rpc_client")]
pub use dpp::dashcore_rpc;
pub use drive;
pub use drive_proof_verifier::types as query_types;
pub use drive_proof_verifier::Error as ProofVerifierError;
#[cfg(feature = "shielded")]
pub use grovedb_commitment_tree;
pub use rs_dapi_client as dapi_client;
pub mod sync;

/// Version of the SDK
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
