//! Count-query executors on `impl Drive`. One file per no-proof
//! [`super::DocumentCountMode`] variant, and one for every proof mode:
//! the dispatcher ([`super::drive_dispatcher`]) calls into the right one
//! based on the detected mode.
//!
//! Each no-proof executor:
//!
//! 1. Picks the right covering index for its mode (returns
//!    `Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty)`
//!    if no index covers the where clauses).
//! 2. Builds the appropriate `DriveDocumentCountQuery`.
//! 3. Runs the matching method on it (`execute_no_proof` or
//!    `execute_range_count_no_proof`) and returns
//!    `Vec<SplitCountEntry>`.
//!
//! The proof executor ([`proof`]) instead proves what the request's
//! [`super::CountProofPlan`] reads, the plan the SDK's verifier rebuilds
//! the path query from, and returns the `Vec<u8>` proof bytes.
//!
//! No re-exports are needed: each file adds methods directly to
//! `impl Drive`, so callers (the dispatcher) just see them on
//! the `Drive` type.

pub mod per_in_value;
pub mod proof;
pub mod range_no_proof;
pub mod total;
