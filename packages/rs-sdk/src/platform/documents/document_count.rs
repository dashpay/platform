//! `FromProof` + `Fetch` for [`DocumentCount`] — the single-row
//! aggregate count view of the unified `getDocuments` endpoint.
//!
//! Callers build a [`DocumentQuery`] with
//! `.with_select(Select::Count)`, optionally adding a
//! `with_where(...)` clause; whatever the request shape, this
//! impl returns a single `u64` (the aggregate count). Per-shape
//! proof dispatch lives in
//! [`super::count_proof_helpers::verify_aggregate_count`] so the
//! sibling [`DocumentSplitCounts`] impl can share it for its own
//! `group_by = []` branch.
//!
//! [`DocumentSplitCounts`]: drive_proof_verifier::DocumentSplitCounts

use crate::platform::documents::count_proof_helpers::{
    assert_select_is_count, verify_aggregate_count,
};
use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::Fetch;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::{DocumentCount, FromProof};

impl FromProof<DocumentQuery> for DocumentCount {
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
        let (count, mtd, proof) =
            verify_aggregate_count(request, response, platform_version, provider)?;
        Ok((count.map(DocumentCount), mtd, proof))
    }
}

impl Fetch for DocumentCount {
    type Request = DocumentQuery;
}
