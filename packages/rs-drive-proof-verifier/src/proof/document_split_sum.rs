//! Verified per-entry sum result.
//!
//! Sum-side analog of [`super::document_split_count::DocumentSplitCounts`].
//! Holds one verified `(in_key, key, sum)` triple per matched group.
//! Returned by `select=SUM, group_by=[...]` queries; aggregate sums
//! use [`super::document_sum::DocumentSum`] instead.
//!
//! **Status**: skeleton. See `document_sum.rs` for the matching
//! grovedb PR 670 dependency note.

use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;

/// A single verified `(in_key?, key, sum)` entry from a sum query
/// with `group_by`. Mirrors count's `SplitCountEntry` shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitSumEntry {
    /// Outer In-prefix value for compound `(In, range)` queries;
    /// `None` for flat queries.
    pub in_key: Option<Vec<u8>>,
    /// The terminator key value.
    pub key: Vec<u8>,
    /// The aggregated sum at that key. `Some(n)` for matched keys;
    /// `None` for keys proven absent.
    pub sum: Option<i64>,
}

/// The full per-entry sum result, verified from proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSplitSums(pub Vec<SplitSumEntry>);

impl DocumentSplitSums {
    /// Convenience: collapse compound `(in_key, key)` entries into a
    /// flat `BTreeMap<key, summed_sum>` by combining each In-fork's
    /// contribution at the same terminator key. Same shape as
    /// `DocumentSplitCounts::into_flat_map` on the count side.
    pub fn into_flat_map(self) -> std::collections::BTreeMap<Vec<u8>, i64> {
        let mut out = std::collections::BTreeMap::new();
        for entry in self.0 {
            if let Some(sum) = entry.sum {
                *out.entry(entry.key).or_insert(0) += sum;
            }
        }
        out
    }
}

impl<'dq, Q> FromProof<Q> for DocumentSplitSums
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
        // TODO(sum-feature): mirror `DocumentSplitCounts::FromProof`'s
        // implementation. The shape:
        //   1. Match on response.result oneof for `SumResults::entries`
        //      or `Proof(p)`.
        //   2. For `Proof(p)`, dispatch on the query mode to either
        //      `DriveDocumentSumQuery::verify_distinct_sum_proof` (per
        //      distinct value) or `verify_carrier_aggregate_sum_proof`
        //      (compound In+range, one entry per branch).
        //   3. Verify tenderdash, map verified entries to
        //      `Vec<SplitSumEntry>`.
        Err(Error::NotImplemented(
            "DocumentSplitSums::FromProof — pending the same grovedb PR 670 \
             dependencies as DocumentSum (verify_aggregate_sum_query, \
             verify_aggregate_sum_query_per_key, verify_distinct_sum_proof). See \
             document_sum.rs for the full dependency catalog."
                .to_string(),
        ))
    }
}
