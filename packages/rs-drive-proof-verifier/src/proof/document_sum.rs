//! Verified sum result.
//!
//! Sum-side analog of [`super::document_count::DocumentCount`]. Holds
//! the aggregated `i64` recovered from a sum-tree proof — `Aggregate`
//! mode returns one of these; `Entries` mode returns
//! [`super::document_split_sum::DocumentSplitSums`] instead.
//!
//! **Status**: skeleton. The `FromProof` impl below is a placeholder
//! that returns `Error::NotYetImplemented` until grovedb PR 670 lands
//! `verify_aggregate_sum_query` and the rs-drive
//! `DriveDocumentSumQuery::verify_*_proof` helpers are authored.

use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;

/// The aggregated sum of an integer property across documents matching
/// a query, verified from proof.
///
/// Signed because grovedb's `SumTree` value type is `i64` — sums can
/// in principle be negative (typically signaling i64 overflow into
/// negative space, which the verifier surfaces explicitly). For
/// tip-jar-style non-negative aggregations this stays ≥ 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSum(pub i64);

impl<'dq, Q> FromProof<Q> for DocumentSum
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
        // TODO(sum-feature): mirror `DocumentCount::FromProof`'s
        // implementation (~50 lines in document_count.rs). The shape:
        //   1. Convert request to `DriveDocumentSumQuery` (mirror of
        //      count's `TryInto<DriveDocumentQuery>` bound).
        //   2. Match on the response's `result` oneof:
        //      `SumResults::aggregate_sum` → wrap in `Some(DocumentSum(sum))`,
        //      `SumResults::entries` → reject (caller should use
        //      `DocumentSplitSums` for entries mode),
        //      `Proof(p)` → call
        //      `DriveDocumentSumQuery::verify_proof(p, ...)` (this
        //      helper depends on grovedb PR 670's
        //      `verify_aggregate_sum_query`).
        //   3. Verify tenderdash signature via
        //      `verify_tenderdash_proof`.
        //   4. Return the single i64 in a DocumentSum wrapper.
        Err(Error::NotImplemented(
            "DocumentSum::FromProof — waits on grovedb PR 670's \
             verify_aggregate_sum_query and the rs-drive \
             DriveDocumentSumQuery::verify_*_proof helpers (see \
             drive_document_sum_query/executors/* for the executor \
             scaffolding)."
                .to_string(),
        ))
    }
}
