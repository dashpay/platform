//! Dash ABCI
//!
//! ABCI is an interface that defines the boundary between the replication engine (the blockchain),
//! and the state machine (the application). Using a socket protocol, a consensus engine running
//! in one process can manage an application state running in another.
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
// Coding conventions
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]
// Test-code heavy lints. The tests in this crate lean on helpers that take
// `&Vec<_>` slices, clone `Copy` types for readability, or reference
// cloned slices. Fixing each site individually would be churn with no runtime
// benefit, so we suppress the lints at the crate level.
#![cfg_attr(test, allow(unused_imports))]
#![cfg_attr(test, allow(unused_mut))]
#![cfg_attr(test, allow(unused_variables))]
#![cfg_attr(test, allow(clippy::useless_vec))]
#![cfg_attr(test, allow(clippy::module_inception))]
#![cfg_attr(test, allow(clippy::cloned_ref_to_slice_refs))]
#![cfg_attr(test, allow(clippy::clone_on_copy))]
#![cfg_attr(test, allow(clippy::useless_conversion))]
#![cfg_attr(test, allow(clippy::unnecessary_mut_passed))]
#![cfg_attr(test, allow(clippy::for_kv_map))]
#![cfg_attr(test, allow(clippy::needless_borrow))]
#![cfg_attr(test, allow(clippy::ptr_arg))]
#![cfg_attr(test, allow(clippy::double_ended_iterator_last))]
#![cfg_attr(test, allow(clippy::redundant_closure))]
#![cfg_attr(test, allow(clippy::type_complexity))]
#![cfg_attr(test, allow(clippy::too_many_arguments))]
#![cfg_attr(test, allow(clippy::needless_borrows_for_generic_args))]
#![cfg_attr(test, allow(clippy::items_after_test_module))]
#![cfg_attr(test, allow(clippy::collapsible_match))]
#![cfg_attr(test, allow(clippy::same_item_push))]
#![cfg_attr(test, allow(clippy::doc_lazy_continuation))]
#![cfg_attr(test, allow(clippy::assertions_on_constants))]
#![cfg_attr(test, allow(clippy::iter_kv_map))]
#![cfg_attr(test, allow(clippy::let_unit_value))]
#![cfg_attr(test, allow(clippy::suspicious_open_options))]
#![cfg_attr(test, allow(clippy::unnecessary_map_or))]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(test, allow(clippy::unnecessary_unwrap))]
#![cfg_attr(test, allow(irrefutable_let_patterns))]

/// ABCI module
pub mod abci;

/// Errors module
pub mod error;

/// Execution module
pub mod execution;

/// Platform configuration
pub mod config;

/// Logging and tracing
pub mod logging;

/// Anything related to 3rd party RPC
pub mod rpc;

/// Core utilities
pub mod core;

/// Metrics subsystem
pub mod metrics;

/// Per-block phase timing, enabled with DRIVE_BLOCK_PERF=1
pub mod perf;

/// Test helpers and fixtures
#[cfg(any(feature = "mocks", test))]
pub mod test;

/// Mimic of block execution for tests
#[cfg(any(feature = "mocks", test))]
pub mod mimic;
/// Platform module
pub mod platform_types;
/// Querying
pub mod query;
/// Various utils
pub mod utils;

/// Replay captured ABCI requests against drive-abci
#[cfg(feature = "replay")]
pub mod replay;
/// Drive server
pub mod server;
/// Shielded-pool genesis snapshot — bake/apply.
///
/// Test-data tooling only: the bake reads a pool seeded by
/// `create_data_for_shielded_pool` and the apply runs from that same seeder's
/// fast-path, so the module has no purpose in a production build. Gated on
/// the `shielded_test_data` Cargo feature (which also enables the underlying
/// grovedb APIs through the dep chain) plus `test`, so the snapshot
/// roundtrip test exercises it under `cargo test`.
#[cfg(any(feature = "shielded_test_data", test))]
pub mod shielded_snapshot;
/// Verification helpers
pub mod verify;
