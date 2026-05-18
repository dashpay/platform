//! Verified average result.
//!
//! Average-side analog of [`super::document_sum::DocumentSum`]. Holds
//! the `(count, sum)` pair recovered from a `CountSumTree` /
//! `ProvableCountProvableSumTree` (PCPS) proof. `Aggregate` mode
//! returns one of these; `Entries` mode returns
//! [`super::document_split_average::DocumentSplitAverages`] instead.
//!
//! Averages are NOT pre-divided server-side — the verifier surfaces
//! the raw `(count, sum)` and the caller divides. See the proto file's
//! `AverageResults` docstring for the rationale (precision +
//! client-chosen representation).
//!
//! **Status**: skeleton. The `FromProof` impl below is a placeholder
//! that returns `Error::NotYetImplemented` until grovedb PR 670 lands
//! `verify_aggregate_count_and_sum_query` and the rs-drive
//! `DriveDocumentAverageQuery::verify_*_proof` helpers are authored.
//! Same status as `DocumentSum` / `DocumentSplitSums`.

use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;

/// The `(count, sum)` pair across documents matching a query,
/// verified from proof. Client computes `avg = sum / count` using
/// whichever precision representation it wants.
///
/// `count` is `u64` (counts are non-negative); `sum` is `i64`
/// (matching `DocumentSum`). The grovedb primitive that backs this is
/// `AggregateCountAndSumOnRange` — both metrics from one
/// root-hash-committed traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentAverage {
    /// Total matched-document count for the query.
    pub count: u64,
    /// Total aggregated value of the `sum_property` for the query.
    pub sum: i64,
}

impl DocumentAverage {
    /// Convenience: compute the average as `f64`. Returns `None` when
    /// `count == 0` (preserving the divide-by-zero contract rather
    /// than producing `NaN` / `inf`). Callers that need a different
    /// representation should divide `self.sum / self.count` directly.
    pub fn as_f64(&self) -> Option<f64> {
        if self.count == 0 {
            None
        } else {
            Some(self.sum as f64 / self.count as f64)
        }
    }
}

#[allow(clippy::extra_unused_lifetimes)]
impl<'dq, Q> FromProof<Q> for DocumentAverage
where
    Q: Clone + 'dq,
{
    type Request = Q;
    type Response = GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        _response: O,
        _network: Network,
        _platform_version: &PlatformVersion,
        _provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        // TODO(avg-feature): mirror `DocumentSum::FromProof`'s
        // implementation once it lands. The shape:
        //   1. Convert request to `DriveDocumentAverageQuery`.
        //   2. Match on the response's `result` oneof:
        //      `AverageResults::aggregate_average` → wrap in
        //      `Some(DocumentAverage { count, sum })`,
        //      `AverageResults::entries` → reject (caller should use
        //      `DocumentSplitAverages`),
        //      `Proof(p)` → call the rs-drive
        //      `verify_aggregate_count_and_sum_proof` helper (depends
        //      on grovedb PR 670's `verify_aggregate_count_and_sum_query`).
        //   3. Verify tenderdash signature.
        //   4. Return the `(count, sum)` pair in a DocumentAverage wrapper.
        Err(Error::DriveError {
            error: "DocumentAverage::FromProof — not yet wired through this \
                    higher-level SDK layer. The drive-side primitives are \
                    available (DriveDocumentSumQuery::verify_aggregate_count_and_sum_proof); \
                    plumbing them up to FromProof is the pending SDK fan-out follow-up, \
                    same as DocumentSum."
                .to_string(),
        })
    }
}
