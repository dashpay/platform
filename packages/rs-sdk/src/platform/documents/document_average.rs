//! `Fetch` impl for [`DocumentAverage`] — the single-value
//! aggregate average view of the unified `getDocuments` endpoint.
//!
//! Average-side analog of [`super::document_sum`]. Callers build a
//! `DocumentQuery` with `.with_select(Select::Avg)` and
//! `.with_select_field("score")`; whatever the request shape, this
//! impl returns a single `(count, sum)` pair (the client computes
//! `avg = sum / count` itself).
//!
//! Why not return a pre-computed average? See the proto file's
//! `AverageResults` docstring: returning the pair preserves precision
//! and lets each caller pick its own representation.

use crate::platform::Fetch;
use drive_proof_verifier::DocumentAverage;

impl Fetch for DocumentAverage {
    type Request = super::document_query::DocumentQuery;
}
