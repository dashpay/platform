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
//!
//! A `containsAny` over several elements of a typed array is refused, even
//! grouped by the array: summed, a document holding two of the elements would
//! count twice. [`drive_proof_verifier::DocumentSplitCounts`] keeps one count
//! per element.

use crate::documents::count_proof_helpers::{
    assert_select_is_count, check_count_request, verify_count_query,
};
use crate::documents::document_query::DocumentQuery;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::{DocumentCount, FromProof};

impl FromProof<DocumentQuery> for DocumentCount {
    type Request = DocumentQuery;
    type Response = GetDocumentsResponse;

    /// One total adds up every verified entry, so the verifier refuses a `containsAny`
    /// that reads several elements of a typed array even grouped by the array, which
    /// the server answers with one total per element.
    fn check_request(
        request: &Self::Request,
        platform_version: &PlatformVersion,
    ) -> Result<(), drive_proof_verifier::Error> {
        check_count_request(request, true, platform_version)
    }

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
            verify_count_query(request, response, true, platform_version, provider)?;
        let count = entries
            .map(|es| es.iter().filter_map(|e| e.count).sum::<u64>())
            .map(DocumentCount);
        Ok((count, mtd, proof))
    }
}
