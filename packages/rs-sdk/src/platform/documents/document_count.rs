//! `FromProof` + `Fetch` for [`DocumentCount`] — the single-row
//! aggregate count view of the unified `getDocuments` endpoint.
//!
//! Callers build a [`DocumentQuery`] with
//! `.with_select(Select::Count)`, optionally adding a
//! `with_where(...)` clause; whatever the request shape, this
//! impl returns a single `u64` (the aggregate count). Per-shape
//! proof dispatch lives in
//! [`super::count_proof_helpers::verify_count_query`] — this
//! impl just sums the verified entries the helper returns.
//!
//! Empty entries (e.g. a verifier that emitted `None` for a
//! queried-but-absent branch) contribute 0 to the sum via
//! `filter_map(|e| e.count)`.

use crate::platform::documents::count_proof_helpers::{assert_select_is_count, verify_count_query};
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
        let (entries, mtd, proof) =
            verify_count_query(request, response, platform_version, provider)?;
        let count = entries
            .map(|es| es.iter().filter_map(|e| e.count).sum::<u64>())
            .map(DocumentCount);
        Ok((count, mtd, proof))
    }
}

impl Fetch for DocumentCount {
    type Query = DocumentQuery;
    type Request = dapi_grpc::platform::v0::GetDocumentsRequest;
}
