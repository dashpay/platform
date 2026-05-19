//! `FromProof` + `Fetch` for [`DocumentAverage`] — the single-row
//! aggregate `(count, sum)` view of the unified `getDocuments`
//! endpoint.
//!
//! Callers build a [`DocumentQuery`] with
//! `.with_select(Select::Avg)` and `.with_select_field("<prop>")`;
//! whatever the request shape, this impl returns a single
//! `DocumentAverage { count, sum }`. Per-shape proof dispatch lives
//! in [`super::average_proof_helpers::verify_average_query`] — this
//! impl folds the verified entries into a single pair.
//!
//! Empty entries (a verifier that emitted `None` for a queried-but-
//! absent branch — same forward-compat for absence proofs as count)
//! contribute 0 to both axes via `filter_map(|e| e.<field>)`.

use crate::platform::documents::average_proof_helpers::{
    assert_select_is_avg, verify_average_query,
};
use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::Fetch;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::{DocumentAverage, FromProof};

impl FromProof<DocumentQuery> for DocumentAverage {
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
        // Fold per-branch (count, sum) into a single aggregate
        // (count, sum). The verifier already returns the single
        // (count, sum) for the aggregate / primary-key fast path
        // wrapped as one entry; the carrier path returns one entry
        // per In branch, which the user folds.
        let avg = entries.map(|es| {
            let mut total_count: u64 = 0;
            let mut total_sum: i64 = 0;
            for e in es {
                if let Some(c) = e.count {
                    total_count = total_count.saturating_add(c);
                }
                if let Some(s) = e.sum {
                    total_sum = total_sum.saturating_add(s);
                }
            }
            DocumentAverage {
                count: total_count,
                sum: total_sum,
            }
        });
        Ok((avg, mtd, proof))
    }
}

impl Fetch for DocumentAverage {
    type Request = super::document_query::DocumentQuery;
}
