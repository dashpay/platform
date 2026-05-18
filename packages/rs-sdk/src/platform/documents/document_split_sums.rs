//! `Fetch` impl for [`DocumentSplitSums`] — the per-group-entry
//! view of the unified `getDocuments` endpoint for sum queries.
//!
//! Sum-side analog of
//! [`super::document_split_counts`]. Returned by
//! `select=SUM, group_by=[...]` queries; aggregate sums use
//! [`super::document_sum::DocumentSum`] instead.

use crate::platform::Fetch;
use drive_proof_verifier::DocumentSplitSums;

impl Fetch for DocumentSplitSums {
    type Request = super::document_query::DocumentQuery;
}
