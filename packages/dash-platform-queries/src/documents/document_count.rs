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
use drive_proof_verifier::{DocumentCount, FromProof, SplitCountEntry};

/// Fold per-branch counts into one `u64`. A total that does not fit is an
/// `UnusableVerifiedResult` rather than a panic (debug) or a wrapped count
/// (release): the entries are verified, so every honest node's answer
/// overflows alike.
fn fold_count_entries(entries: &[SplitCountEntry]) -> Result<u64, drive_proof_verifier::Error> {
    entries
        .iter()
        .filter_map(|e| e.count)
        .try_fold(0u64, u64::checked_add)
        .ok_or_else(|| drive_proof_verifier::Error::UnusableVerifiedResult {
            reason: "DocumentCount: u64 overflow folding per-branch counts into a single \
                     aggregate. The proof itself verified, but the requested total count \
                     doesn't fit in u64. Use DocumentSplitCounts to receive per-branch \
                     counts and fold them with your own arithmetic."
                .to_string(),
        })
}

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
            .map(|es| fold_count_entries(&es))
            .transpose()?
            .map(DocumentCount);
        Ok((count, mtd, proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(in_key: u8, count: Option<u64>) -> SplitCountEntry {
        SplitCountEntry {
            in_key: Some(vec![in_key]),
            key: vec![0u8],
            count,
        }
    }

    /// The total adds up every verified branch; a branch the verifier proved absent adds 0.
    #[test]
    fn should_add_up_every_verified_branch_count() {
        let entries = [entry(1, Some(100)), entry(2, None), entry(3, Some(50))];
        assert_eq!(fold_count_entries(&entries).expect("fits in u64"), 150);
    }

    /// Branch counts whose total does not fit in `u64` are an unusable verified result, as for
    /// the sum and average views: every honest node's answer overflows alike. The total must
    /// neither panic nor wrap to a wrong count.
    #[test]
    fn should_report_an_overflowing_count_as_an_unusable_verified_result() {
        let entries = [entry(1, Some(u64::MAX)), entry(2, Some(1))];
        match fold_count_entries(&entries) {
            Err(drive_proof_verifier::Error::UnusableVerifiedResult { reason }) => assert!(
                reason.contains("u64 overflow") && reason.contains("DocumentSplitCounts"),
                "{reason}"
            ),
            other => panic!("expected an unusable verified result, got {other:?}"),
        }
    }
}
