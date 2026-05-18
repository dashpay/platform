//! Average-query dispatcher entry point.
//!
//! Parallels [`crate::query::drive_document_sum_query::drive_dispatcher`]
//! for the average surface. Routes a parsed [`DocumentAverageRequest`]
//! to the executor that returns `(count, sum)` for the request's shape.
//!
//! Execution body is currently a stub — it returns the same
//! `NotYetImplemented` rejection sum's dispatcher does, since the
//! grovedb-level `AggregateCountAndSumOnRange` proof and PCPS executor
//! bodies that back this surface land in the same follow-up as sum's
//! executors. The wire-stable request/response types are in place so
//! the platform layer's `dispatch_average_v1` can call this method
//! without further refactoring once the executor lands.
//!
//! Why not just compose count + sum?
//!   * Atomicity: the grovedb primitive yields both values from a
//!     single root-hash-committed traversal. Composing two requests
//!     would let the count and sum land on different state versions
//!     (block boundary races) and produce off-by-one inconsistencies
//!     on the client's computed average.
//!   * Cost: two proofs ≈ 2× wire bytes vs one combined proof.
//!   * Index correctness: only PCPS-backed indexes (count flags +
//!     sum flags) can serve the combined primitive; a "compose" path
//!     would silently degrade for indexes that only opt into one.
//! The combined primitive avoids all three.
//!
//! `where_clauses_from_value` / `order_clauses_from_value` are wire-shape
//! adapters; see [`super::super::drive_document_sum_query::drive_dispatcher`]
//! for the canonical implementation (averages reuse those helpers
//! verbatim).

use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::drive_document_average_query::{DocumentAverageRequest, DocumentAverageResponse};
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

#[cfg(feature = "server")]
impl Drive {
    /// Server-side entry point for the average surface. Routes a
    /// [`DocumentAverageRequest`] to the appropriate executor based on
    /// the where-shape, requested mode, and `prove` flag.
    ///
    /// Mirrors [`Drive::execute_document_sum_request`]. Execution
    /// body is the same NotYetImplemented stub sum's dispatcher
    /// currently returns — both wire surfaces unblock the platform
    /// layer's routing decisions ahead of the executor bodies.
    pub fn execute_document_average_request(
        &self,
        _request: DocumentAverageRequest,
        _transaction: TransactionArg,
        _platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        // TODO(avg-feature): mirror `execute_document_sum_request`
        // once the PCPS executor bodies in
        // `drive_document_sum_query/executors/*.rs` are filled in.
        // Average's executor is a thin wrapper around the same PCPS
        // / `AggregateCountAndSumOnRange` machinery — both metrics
        // come out of one traversal, so it's a single executor call
        // returning a `(count, sum)` pair instead of count's `u64`
        // or sum's `i64`.
        Err(Error::Query(QuerySyntaxError::Unsupported(
            "execute_document_average_request: server-side execution waits on the \
             rs-drive count+sum executor bodies in \
             packages/rs-drive/src/query/drive_document_sum_query/executors/* \
             being ported from their count-side analogs, and on grovedb PR 670's \
             `AggregateCountAndSumOnRange` proof primitive. The wire-stable \
             request/response surface (DocumentAverageRequest / DocumentAverageResponse) \
             is in place so the platform-layer dispatcher can call this method \
             without further refactoring once execution lands."
                .to_string(),
        )))
    }
}
