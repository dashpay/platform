//! Verified per-entry average result.
//!
//! Average-side analog of [`super::document_split_sum::DocumentSplitSums`].
//! Holds one verified `(in_key, key, count, sum)` 4-tuple per matched
//! group — the client divides each `sum / count` to compute per-group
//! averages. Returned by `select=AVG, group_by=[...]` queries;
//! aggregate averages use [`super::document_average::DocumentAverage`]
//! instead.
//!
//! **Status**: skeleton. See `document_average.rs` for the matching
//! grovedb PR 670 dependency note.

use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;

/// A single verified `(in_key?, key, count, sum)` entry from an
/// average query with `group_by`. Mirrors sum's `SplitSumEntry`
/// shape with both metrics carried alongside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitAverageEntry {
    /// Outer In-prefix value for compound `(In, range)` queries;
    /// `None` for flat queries.
    pub in_key: Option<Vec<u8>>,
    /// The terminator key value.
    pub key: Vec<u8>,
    /// Matched-document count at that key. `Some(n)` for matched
    /// keys; `None` for keys proven absent.
    pub count: Option<u64>,
    /// Aggregated sum at that key. `Some(n)` for matched keys;
    /// `None` for keys proven absent.
    pub sum: Option<i64>,
}

impl SplitAverageEntry {
    /// Convenience: compute the average for this entry as `f64`, or
    /// `None` if the entry was proven absent (`count`/`sum` is `None`)
    /// or has zero count.
    pub fn as_f64(&self) -> Option<f64> {
        match (self.count, self.sum) {
            (Some(c), Some(s)) if c > 0 => Some(s as f64 / c as f64),
            _ => None,
        }
    }
}

/// The full per-entry average result, verified from proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSplitAverages(pub Vec<SplitAverageEntry>);

impl DocumentSplitAverages {
    /// Convenience: collapse compound `(in_key, key)` entries into a
    /// flat `BTreeMap<key, (summed_count, summed_sum)>` by combining
    /// each In-fork's contribution at the same terminator key.
    /// Mirrors `DocumentSplitSums::into_flat_map`.
    pub fn into_flat_map(self) -> std::collections::BTreeMap<Vec<u8>, (u64, i64)> {
        let mut out = std::collections::BTreeMap::new();
        for entry in self.0 {
            if let (Some(c), Some(s)) = (entry.count, entry.sum) {
                let acc = out.entry(entry.key).or_insert((0u64, 0i64));
                acc.0 += c;
                acc.1 += s;
            }
        }
        out
    }
}

#[allow(clippy::extra_unused_lifetimes)]
impl<'dq, Q> FromProof<Q> for DocumentSplitAverages
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
        // TODO(avg-feature): mirror `DocumentSplitSums::FromProof`'s
        // implementation. The shape:
        //   1. Match on response.result oneof for `AverageResults::entries`
        //      or `Proof(p)`.
        //   2. For `Proof(p)`, dispatch on the query mode to the
        //      rs-drive `verify_*_count_and_sum_proof` helpers (carrier
        //      / distinct variants), depending on the grovedb PR 670
        //      proof primitives.
        //   3. Verify tenderdash, map verified entries to
        //      `Vec<SplitAverageEntry>`.
        Err(Error::DriveError {
            error: "DocumentSplitAverages::FromProof — not yet wired through this \
                    higher-level SDK layer. The drive-side primitives are \
                    available (verify_carrier_aggregate_count_and_sum_proof); plumbing \
                    them up to FromProof is the pending SDK fan-out follow-up, same \
                    as DocumentSplitSums."
                .to_string(),
        })
    }
}
