//! Per-mode count-query executors on `impl Drive`. One file per
//! [`super::DocumentCountMode`] variant — the dispatcher
//! ([`super::drive_dispatcher`]) calls into the right one based
//! on the detected mode.
//!
//! Each executor:
//!
//! 1. Picks the right covering index for its mode (returns
//!    `Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty)`
//!    if no index covers the where clauses).
//! 2. Builds the appropriate `DriveDocumentCountQuery`.
//! 3. Runs the matching method on it (`execute_no_proof`,
//!    `execute_range_count_no_proof`,
//!    `execute_aggregate_count_with_proof`,
//!    `execute_distinct_count_with_proof`, or
//!    `execute_point_lookup_count_with_proof`).
//! 4. Returns `Vec<SplitCountEntry>` (no-proof modes) or
//!    `Vec<u8>` proof bytes (proof modes).
//!
//! Splitting along mode boundaries — one file per mode — keeps
//! each executor's index-picking + clause-handling logic local
//! and lets the dispatcher's match arms stay one line each. No
//! re-exports are needed: each file adds methods directly to
//! `impl Drive`, so callers (the dispatcher) just see them on
//! the `Drive` type.

pub mod per_in_value;
pub mod point_lookup_proof;
pub mod range_aggregate_carrier_proof;
pub mod range_distinct_proof;
pub mod range_no_proof;
pub mod range_proof;
pub mod total;
