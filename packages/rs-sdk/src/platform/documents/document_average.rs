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
use drive_proof_verifier::{AverageEntry, DocumentAverage, FromProof};

/// Fold per-branch `(count, sum)` into a single aggregate
/// `(count, sum)`. Uses `checked_add` on BOTH axes so a multi-entry
/// fold that exceeds `u64::MAX` (count) or `i64::MAX` / underflows
/// below `i64::MIN` (sum) surfaces as a `RequestError` rather than
/// silently saturating.
///
/// The prior `saturating_add` was unsafe: a saturated count or sum
/// could pin the computed average to a wrong value (e.g., a sum
/// that saturated at `i64::MAX` divided by an accurate count would
/// understate the true average). Extracted into a free function so
/// the overflow paths are unit-testable.
fn fold_average_entries(
    entries: &[AverageEntry],
) -> Result<DocumentAverage, drive_proof_verifier::Error> {
    let mut total_count: u64 = 0;
    let mut total_sum: i64 = 0;
    for e in entries {
        if let Some(c) = e.count {
            total_count = total_count.checked_add(c).ok_or_else(|| {
                drive_proof_verifier::Error::RequestError {
                    error: "DocumentAverage: u64 overflow folding per-branch counts into a \
                            single aggregate. The proof itself verified, but the requested \
                            total count doesn't fit in u64. Use DocumentSplitAverages to \
                            receive per-branch (u64, i64) and fold with your own arithmetic."
                        .to_string(),
                }
            })?;
        }
        if let Some(s) = e.sum {
            total_sum = total_sum.checked_add(s).ok_or_else(|| {
                drive_proof_verifier::Error::RequestError {
                    error: "DocumentAverage: i64 over/underflow folding per-branch sums into \
                            a single aggregate. The proof itself verified, but the requested \
                            total sum doesn't fit in i64. Use DocumentSplitAverages to \
                            receive per-branch (u64, i64) and fold with your own arithmetic \
                            (e.g. i128)."
                        .to_string(),
                }
            })?;
        }
    }
    Ok(DocumentAverage {
        count: total_count,
        sum: total_sum,
    })
}

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
        // Fold per-branch (count, sum) into a single aggregate via
        // `fold_average_entries` — checked arithmetic on both axes,
        // see helper docstring.
        let avg = match entries {
            None => None,
            Some(es) => Some(fold_average_entries(&es)?),
        };
        Ok((avg, mtd, proof))
    }
}

impl Fetch for DocumentAverage {
    type Query = super::document_query::DocumentQuery;
    type Request = dapi_grpc::platform::v0::GetDocumentsRequest;
}

#[cfg(test)]
mod tests {
    //! Unit tests for the AVG fold. The fold logic is extracted
    //! into `fold_average_entries` so we can pin overflow /
    //! underflow behavior on both axes without driving a full
    //! proof flow. The prior `saturating_add` implementation
    //! would silently pin overflow results to numeric bounds and
    //! produce a wrong average — these tests lock the explicit-
    //! error behavior.

    use super::*;

    fn entry(in_key: Option<Vec<u8>>, count: Option<u64>, sum: Option<i64>) -> AverageEntry {
        AverageEntry {
            in_key,
            key: vec![0u8],
            count,
            sum,
        }
    }

    /// Single-branch fold: pass-through. Smoke test.
    #[test]
    fn fold_average_entries_single_branch_passes_through() {
        let entries = vec![entry(None, Some(10), Some(250))];
        let avg = fold_average_entries(&entries).expect("single branch should fold cleanly");
        assert_eq!(
            avg,
            DocumentAverage {
                count: 10,
                sum: 250
            }
        );
    }

    /// Multi-branch with absent (verifier-emitted `None`) branches:
    /// `None` on either axis contributes 0 to that axis.
    #[test]
    fn fold_average_entries_multi_branch_with_absent_axes() {
        let entries = vec![
            entry(Some(vec![1]), Some(5), Some(100)),
            entry(Some(vec![2]), None, None), // fully absent → contributes (0, 0)
            entry(Some(vec![3]), Some(3), Some(50)),
            entry(Some(vec![4]), Some(2), None), // sum absent, count present
        ];
        let avg = fold_average_entries(&entries)
            .expect("absent axes must contribute 0 on their respective axis");
        assert_eq!(
            avg,
            DocumentAverage {
                count: 10,
                sum: 150
            }
        );
    }

    /// `u64::MAX` count + 1 → must error, not saturate. Regression:
    /// the prior `saturating_add` would pin the count to
    /// `u64::MAX` and produce a wrong average (saturated count /
    /// accurate sum understates the average).
    #[test]
    fn fold_average_entries_count_overflow_returns_error() {
        let entries = vec![
            entry(Some(vec![1]), Some(u64::MAX), Some(0)),
            entry(Some(vec![2]), Some(1), Some(0)),
        ];
        let err = fold_average_entries(&entries)
            .expect_err("u64 count overflow must surface as RequestError, not saturate");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("u64 overflow") && msg.contains("DocumentSplitAverages"),
            "error must name the count overflow + hint at DocumentSplitAverages; got {msg}"
        );
    }

    /// `i64::MAX` sum + positive → must error. Same regression as
    /// count overflow.
    #[test]
    fn fold_average_entries_positive_sum_overflow_returns_error() {
        let entries = vec![
            entry(Some(vec![1]), Some(0), Some(i64::MAX)),
            entry(Some(vec![2]), Some(0), Some(1)),
        ];
        let err = fold_average_entries(&entries)
            .expect_err("positive i64 sum overflow must surface as RequestError");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("i64 over/underflow") && msg.contains("DocumentSplitAverages"),
            "error must name the sum over/underflow + hint at DocumentSplitAverages; got {msg}"
        );
    }

    /// `i64::MIN` sum + negative → must error (the underflow
    /// direction). Symmetric to the positive case so a future
    /// switch to `saturating_*` can't silently regress only one
    /// direction.
    #[test]
    fn fold_average_entries_negative_sum_underflow_returns_error() {
        let entries = vec![
            entry(Some(vec![1]), Some(0), Some(i64::MIN)),
            entry(Some(vec![2]), Some(0), Some(-1)),
        ];
        let err = fold_average_entries(&entries)
            .expect_err("negative i64 sum underflow must surface as RequestError");
        let msg = format!("{err:?}");
        assert!(msg.contains("i64 over/underflow"));
    }

    /// Empty fold returns `(0, 0)` — same as count's `0` empty
    /// fold and SUM's `0`.
    #[test]
    fn fold_average_entries_empty_returns_zero_pair() {
        let avg = fold_average_entries(&[]).expect("empty fold must succeed");
        assert_eq!(avg, DocumentAverage { count: 0, sum: 0 });
    }
}
