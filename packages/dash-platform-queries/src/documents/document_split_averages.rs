//! `FromProof` + `Fetch` for [`DocumentSplitAverages`] — the
//! per-group-entry view of the unified `getDocuments` endpoint
//! for average queries.
//!
//! Average-side analog of [`super::document_split_sums`]. Returns
//! the full `entries` list keyed by the splitting property's
//! serialized value; aggregate averages use
//! [`super::document_average::DocumentAverage`] instead.
//!
//! Per-shape proof dispatch lives in
//! [`super::average_proof_helpers::verify_average_query`] — this
//! impl passes the verified entries through unchanged, mapping
//! `AverageEntry` to `SplitAverageEntry`.

use crate::documents::average_proof_helpers::{assert_select_is_avg, verify_average_query};
use crate::documents::document_query::DocumentQuery;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::{DocumentSplitAverages, FromProof, SplitAverageEntry};

impl FromProof<DocumentQuery> for DocumentSplitAverages {
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
        assert_select_is_avg(&request)?;
        let response: Self::Response = response.into();
        let (entries, mtd, proof) =
            verify_average_query(request, response, platform_version, provider)?;
        let split = entries.map(|es| {
            DocumentSplitAverages(
                es.into_iter()
                    .map(|e| SplitAverageEntry {
                        in_key: e.in_key,
                        key: e.key,
                        count: e.count,
                        sum: e.sum,
                    })
                    .collect(),
            )
        });
        Ok((split, mtd, proof))
    }
}
