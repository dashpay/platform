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
//!
//! Overflow handling: the fold uses [`i64::checked_add`] and returns
//! a `RequestError` on overflow rather than panicking (debug) or
//! wrapping (release). A grovedb sum-tree's per-node aggregate
//! itself fits in `i64` by construction, but a multi-entry fold
//! across carrier-aggregate / distinct branches can in principle
//! exceed that — the verifier surfaces it explicitly so callers
//! can switch to `DocumentSplitSums` (which preserves per-branch
//! `i64`s and lets the caller pick its own arithmetic).

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::documents::sum_proof_helpers::{assert_select_is_sum, verify_sum_query};
use crate::platform::Fetch;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::{DocumentSum, FromProof, SumEntry};

/// Fold per-branch sums into a single `i64`. Returns `RequestError`
/// on overflow rather than panicking (debug) or wrapping (release).
/// Extracted into a free function so the overflow path is
/// unit-testable without driving a full proof flow.
fn fold_sum_entries(entries: &[SumEntry]) -> Result<i64, drive_proof_verifier::Error> {
    let mut total: i64 = 0;
    for e in entries {
        if let Some(s) = e.sum {
            total =
                total
                    .checked_add(s)
                    .ok_or_else(|| drive_proof_verifier::Error::RequestError {
                        error: "DocumentSum: i64 overflow folding per-branch sums into a single \
                            aggregate. The proof itself verified, but the requested sum \
                            doesn't fit in i64. Use DocumentSplitSums to receive per-branch \
                            i64s and fold them with your own arithmetic (e.g. i128)."
                            .to_string(),
                    })?;
        }
    }
    Ok(total)
}

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
        // Fold per-branch sums into a single `i64` using
        // `checked_add` (via `fold_sum_entries`). A multi-entry
        // fold (carrier-aggregate across many In branches, or
        // distinct-mode across many range buckets) can in
        // principle overflow even though each branch is itself a
        // valid grovedb `i64` sum_value. Surface this as a
        // `RequestError` rather than panicking (debug) or
        // wrapping (release) — callers can switch to
        // `DocumentSplitSums` to preserve per-branch numbers.
        let sum = match entries {
            None => None,
            Some(es) => Some(DocumentSum(fold_sum_entries(&es)?)),
        };
        Ok((sum, mtd, proof))
    }
}

impl Fetch for DocumentSum {
    type Query = super::document_query::DocumentQuery;
    type Request = dapi_grpc::platform::v0::GetDocumentsRequest;
}

#[cfg(test)]
mod tests {
    //! Unit tests for the SUM fold. The fold logic is extracted
    //! into `fold_sum_entries` so we can pin its overflow
    //! behavior without driving a full proof flow.

    use super::*;

    fn entry(in_key: Option<Vec<u8>>, sum: Option<i64>) -> SumEntry {
        SumEntry {
            in_key,
            key: vec![0u8],
            sum,
        }
    }

    /// Single in-range branch: fold returns the branch's value.
    /// Smoke test that the helper hasn't regressed on the
    /// non-overflow path. Mirrors `verify_count_query`'s single-
    /// branch unit-test shape.
    #[test]
    fn fold_sum_entries_single_branch_passes_through() {
        let entries = vec![entry(None, Some(42))];
        let sum = fold_sum_entries(&entries).expect("single branch should fold cleanly");
        assert_eq!(sum, 42);
    }

    /// Multi-branch fold sums all the `Some` values. `None` entries
    /// (verifier emitted `None` for a queried-but-absent branch)
    /// contribute 0, matching the count helper's three-valued
    /// semantics.
    #[test]
    fn fold_sum_entries_multi_branch_with_absent_branches() {
        let entries = vec![
            entry(Some(vec![1]), Some(100)),
            entry(Some(vec![2]), None), // absent branch → contributes 0
            entry(Some(vec![3]), Some(50)),
        ];
        let sum = fold_sum_entries(&entries).expect("absent branches must contribute 0");
        assert_eq!(sum, 150);
    }

    /// Positive overflow: two branches summing to `> i64::MAX` must
    /// surface as a `RequestError`. Pre-fix this would panic in
    /// debug or wrap in release (`.sum::<i64>()`) — both unsafe in
    /// a verifier context. Regression: lock the explicit-error
    /// behavior.
    #[test]
    fn fold_sum_entries_positive_overflow_returns_error() {
        let entries = vec![
            entry(Some(vec![1]), Some(i64::MAX)),
            entry(Some(vec![2]), Some(1)),
        ];
        let err = fold_sum_entries(&entries)
            .expect_err("positive overflow must surface as RequestError, not panic/wrap");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("i64 overflow") && msg.contains("DocumentSplitSums"),
            "error must name the overflow + hint at DocumentSplitSums; got {msg}"
        );
    }

    /// Symmetric negative-overflow guard: two branches summing to
    /// `< i64::MIN` must surface as a `RequestError`. `checked_add`
    /// reports both directions; we pin both so a future switch to
    /// `saturating_*` can't silently regress only one direction.
    #[test]
    fn fold_sum_entries_negative_overflow_returns_error() {
        let entries = vec![
            entry(Some(vec![1]), Some(i64::MIN)),
            entry(Some(vec![2]), Some(-1)),
        ];
        let err =
            fold_sum_entries(&entries).expect_err("negative overflow must surface as RequestError");
        let msg = format!("{err:?}");
        assert!(msg.contains("i64 overflow"));
    }

    /// Empty fold: zero entries → zero sum. Pre-empts a regression
    /// where an empty-branch optimization would return `None` and
    /// FromProof callers would surface "no proof" errors instead
    /// of "verified zero".
    #[test]
    fn fold_sum_entries_empty_returns_zero() {
        let sum = fold_sum_entries(&[]).expect("empty fold must succeed");
        assert_eq!(sum, 0);
    }
}
