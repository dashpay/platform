//! `FromProof` + `Fetch` for [`DocumentSplitSums`] — the
//! per-group-entry view of the unified `getDocuments` endpoint
//! for sum queries.
//!
//! Sum-side analog of [`super::document_split_counts`]. Returns
//! the full `entries` list (keyed by the splitting property's
//! serialized value); aggregate sums use
//! [`super::document_sum::DocumentSum`] instead.
//!
//! Per-shape proof dispatch lives in
//! [`super::sum_proof_helpers::verify_sum_query`] — this impl
//! passes the verified entries through unchanged, mapping
//! `SumEntry` to `SplitSumEntry`.

use crate::documents::document_query::DocumentQuery;
use crate::documents::sum_proof_helpers::{assert_select_is_sum, verify_sum_query};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::{DocumentSplitSums, FromProof, SplitSumEntry};

impl FromProof<DocumentQuery> for DocumentSplitSums {
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
        let split = entries.map(|es| {
            DocumentSplitSums(
                es.into_iter()
                    .map(|e| SplitSumEntry {
                        in_key: e.in_key,
                        key: e.key,
                        sum: e.sum,
                    })
                    .collect(),
            )
        });
        Ok((split, mtd, proof))
    }
}
