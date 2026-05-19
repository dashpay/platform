//! Verified per-entry sum result.
//!
//! Sum-side analog of [`super::document_split_count::DocumentSplitCounts`].
//! Holds one verified `(in_key, key, sum)` triple per matched group.
//! Returned by `select=SUM, group_by=[...]` queries; aggregate sums
//! use [`super::document_sum::DocumentSum`] instead.
//!
//! The generic `FromProof<Q>` impl below intentionally rejects
//! calls (matching [`super::document_split_count::DocumentSplitCounts`]'s
//! pattern). Real dispatch lives in the
//! `FromProof<DocumentQuery>` impl in
//! `rs-sdk/src/platform/documents/document_split_sums.rs`, which
//! picks the right per-shape verifier (carrier-aggregate /
//! point-lookup) based on the resolved `DocumentSumMode`.

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

// No generic `FromProof<Q>` impl — callers reach this through
// `FromProof<DocumentQuery> for DocumentSplitSums` in
// `rs-sdk/src/platform/documents/document_split_sums.rs`. Same
// rationale as `DocumentSum` / `DocumentSplitCounts`: per-mode
// proof dispatch needs the SDK's `DocumentQuery` shape.
