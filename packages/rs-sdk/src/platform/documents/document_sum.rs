//! `Fetch` impl for [`DocumentSum`] — the single-value aggregate
//! sum view of the unified `getDocuments` endpoint.
//!
//! Sum-side analog of [`super::document_count`]. Callers build a
//! `DocumentQuery` with `.with_select(Select::Sum)` and
//! `.with_select_field("amount")`; whatever the request shape, this
//! impl returns a single `i64` (the aggregate sum).
//!
//! Empty entries (verifier emitted `None` for a queried-but-absent
//! branch) contribute 0 to the sum via `filter_map(|e| e.sum)`.

use crate::platform::Fetch;
use drive_proof_verifier::DocumentSum;

impl Fetch for DocumentSum {
    type Request = super::document_query::DocumentQuery;
}
