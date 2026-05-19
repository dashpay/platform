//! `FromProof` + `Fetch` for [`DocumentSum`] — the single-value
//! aggregate sum view of the unified `getDocuments` endpoint.
//!
//! Sum-side analog of [`super::document_count`]. Callers build a
//! `DocumentQuery` with `.with_select(Select::Sum)` and
//! `.with_select_field("amount")`; whatever the request shape,
//! this impl returns a single `i64` (the aggregate sum).
//!
//! Empty entries (verifier emitted `None` for a queried-but-absent
//! branch) contribute 0 to the sum via `filter_map(|e| e.sum)`.

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::documents::sum_proof_helpers::{assert_select_is_sum, verify_sum_query};
use crate::platform::Fetch;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::{DocumentSum, FromProof};

impl FromProof<DocumentQuery> for DocumentSum {
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
        assert_select_is_sum(&request)?;
        let response: Self::Response = response.into();
        let (entries, mtd, proof) =
            verify_sum_query(request, response, platform_version, provider)?;
        let sum = entries
            .map(|es| es.iter().filter_map(|e| e.sum).sum::<i64>())
            .map(DocumentSum);
        Ok((sum, mtd, proof))
    }
}

impl Fetch for DocumentSum {
    type Request = super::document_query::DocumentQuery;
}
