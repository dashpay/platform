//! `FromProof` + `Fetch` for [`DocumentSplitCounts`] — the
//! per-group-entry view of the unified `getDocuments` endpoint.
//!
//! Backed by the same [`DocumentQuery`] as
//! [`drive_proof_verifier::DocumentCount`]; the only difference
//! is response shape — `DocumentSplitCounts` returns the full
//! `entries` list keyed by the splitting property's serialized
//! value, while `DocumentCount` returns the sum.
//!
//! Per-shape proof dispatch lives in
//! [`super::count_proof_helpers::verify_count_query`] — this
//! impl passes the verified entries through unchanged.
//!
//! Shapes emitted by `verify_count_query`:
//! - `group_by = []` (aggregate): one entry with empty `key`
//!   carrying the verified total. `AggregateCountOnRange`,
//!   primary-key CountTree, and point-lookup-on-Equal-only paths
//!   all collapse to this shape.
//! - `group_by = [in_field]` (per-In entries): one entry per
//!   **present** queried In value, with `count: Some(n)`. Absent
//!   In branches are omitted from `entries` — the current
//!   point-lookup path query doesn't request absence proofs,
//!   so grovedb's `verify_query` surfaces only present
//!   `(path, key, Some(Element))` triples. Callers that need to
//!   distinguish "verified zero" from "queried but absent" diff
//!   their request's In array against the returned entries by
//!   `key` (each entry's `key` is `serialize_value_for_key(in_field, v)`).
//! - `group_by = [range_field]` / `[in_field, range_field]`
//!   (distinct walk): one entry per distinct value in the range
//!   (compound queries: per `(in_key, key)` pair). Zero-count
//!   ranges are simply absent — the range itself is unbounded so
//!   there's no enumerable key set to ever-emit.

use crate::platform::documents::count_proof_helpers::{assert_select_is_count, verify_count_query};
use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::Fetch;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::{DocumentSplitCounts, FromProof};

impl FromProof<DocumentQuery> for DocumentSplitCounts {
    type Request = DocumentQuery;
    type Response = GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        assert_select_is_count(&request)?;
        let response: Self::Response = response.into();
        let (entries, mtd, proof) =
            verify_count_query(request, response, platform_version, provider)?;
        Ok((entries.map(DocumentSplitCounts::from_verified), mtd, proof))
    }
}

impl Fetch for DocumentSplitCounts {
    type Query = DocumentQuery;
    type Request = dapi_grpc::platform::v0::GetDocumentsRequest;
}
