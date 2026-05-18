//! `Fetch` impl for [`DocumentSplitAverages`] — the per-group-entry
//! view of the unified `getDocuments` endpoint for average queries.
//!
//! Average-side analog of [`super::document_split_sums`]. Returned by
//! `select=AVG, group_by=[...]` queries; aggregate averages use
//! [`super::document_average::DocumentAverage`] instead.

use crate::platform::Fetch;
use drive_proof_verifier::DocumentSplitAverages;

impl Fetch for DocumentSplitAverages {
    type Request = super::document_query::DocumentQuery;
}
