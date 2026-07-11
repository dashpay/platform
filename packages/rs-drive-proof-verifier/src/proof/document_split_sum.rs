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
    ///
    /// Uses [`i64::checked_add`] on each accumulator step and
    /// surfaces overflow as
    /// [`Error::RequestError`](crate::Error::RequestError) rather
    /// than panicking (debug) or wrapping (release). Mirrors the
    /// SDK's aggregate-side `DocumentSum::fold_sum_entries` so a
    /// wasm caller that collapses split sums lands at the same
    /// hardening boundary the SDK aggregate path already enforces.
    ///
    /// On overflow, drop back to iterating `DocumentSplitSums.0`
    /// directly — per-branch entries preserve the verified `i64`
    /// values, and the caller can fold with its own arithmetic
    /// (e.g. `i128`).
    pub fn try_into_flat_map(
        self,
    ) -> Result<std::collections::BTreeMap<Vec<u8>, i64>, crate::Error> {
        let mut out: std::collections::BTreeMap<Vec<u8>, i64> = std::collections::BTreeMap::new();
        for entry in self.0 {
            if let Some(sum) = entry.sum {
                let slot = out.entry(entry.key).or_insert(0i64);
                *slot =
                    slot.checked_add(sum).ok_or_else(|| {
                        crate::Error::RequestError {
                    error:
                        "DocumentSplitSums::try_into_flat_map: i64 overflow merging per-In-fork \
                         sums at the same terminator key. The verified per-branch entries are \
                         each valid i64, but their fold exceeds the i64 range — iterate \
                         DocumentSplitSums.0 directly to access per-branch sums and fold with \
                         your own arithmetic (e.g. i128)."
                            .to_string(),
                }
                    })?;
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `try_into_flat_map` over a single In-fork must pass the sum
    /// through unchanged — the baseline check the overflow guard
    /// has to preserve.
    #[test]
    fn try_into_flat_map_passes_through_flat_entries() {
        let splits = DocumentSplitSums(vec![SplitSumEntry {
            in_key: None,
            key: b"key-a".to_vec(),
            sum: Some(123),
        }]);
        let flat = splits.try_into_flat_map().expect("non-overflowing fold");
        assert_eq!(flat.len(), 1);
        assert_eq!(flat.get(b"key-a".as_slice()), Some(&123));
    }

    /// Two In-forks landing at the same terminator key get summed.
    #[test]
    fn try_into_flat_map_sums_across_in_forks() {
        let splits = DocumentSplitSums(vec![
            SplitSumEntry {
                in_key: Some(b"alice".to_vec()),
                key: b"red".to_vec(),
                sum: Some(10),
            },
            SplitSumEntry {
                in_key: Some(b"bob".to_vec()),
                key: b"red".to_vec(),
                sum: Some(32),
            },
            SplitSumEntry {
                in_key: Some(b"alice".to_vec()),
                key: b"blue".to_vec(),
                sum: Some(7),
            },
        ]);
        let flat = splits.try_into_flat_map().expect("non-overflowing fold");
        assert_eq!(flat.get(b"red".as_slice()), Some(&42));
        assert_eq!(flat.get(b"blue".as_slice()), Some(&7));
    }

    /// Crossing `i64::MAX` from two valid per-branch i64s must
    /// surface as `RequestError`, not panic / wrap. This is the
    /// exact boundary the SDK's aggregate-side fold already
    /// hardens for `DocumentSum`.
    #[test]
    fn try_into_flat_map_overflow_surfaces_request_error() {
        let splits = DocumentSplitSums(vec![
            SplitSumEntry {
                in_key: Some(b"alice".to_vec()),
                key: b"shared".to_vec(),
                sum: Some(i64::MAX),
            },
            SplitSumEntry {
                in_key: Some(b"bob".to_vec()),
                key: b"shared".to_vec(),
                sum: Some(1),
            },
        ]);
        let err = splits
            .try_into_flat_map()
            .expect_err("i64::MAX + 1 must surface as overflow error, not wrap or panic");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("i64 overflow") && msg.contains("DocumentSplitSums"),
            "error message must name the overflow and the helper: got {msg}"
        );
    }

    /// Underflow on the negative end must surface symmetrically.
    #[test]
    fn try_into_flat_map_underflow_surfaces_request_error() {
        let splits = DocumentSplitSums(vec![
            SplitSumEntry {
                in_key: Some(b"alice".to_vec()),
                key: b"shared".to_vec(),
                sum: Some(i64::MIN),
            },
            SplitSumEntry {
                in_key: Some(b"bob".to_vec()),
                key: b"shared".to_vec(),
                sum: Some(-1),
            },
        ]);
        let err = splits
            .try_into_flat_map()
            .expect_err("i64::MIN - 1 must surface as overflow error, not wrap or panic");
        let msg = format!("{err:?}");
        assert!(msg.contains("i64 overflow"), "got {msg}");
    }

    /// `None`-sum entries are skipped (the verifier emits them for
    /// keys proven absent). They must not poison the accumulator
    /// or trigger a spurious overflow signal.
    #[test]
    fn try_into_flat_map_skips_absent_entries() {
        let splits = DocumentSplitSums(vec![
            SplitSumEntry {
                in_key: None,
                key: b"key-a".to_vec(),
                sum: None,
            },
            SplitSumEntry {
                in_key: None,
                key: b"key-b".to_vec(),
                sum: Some(5),
            },
        ]);
        let flat = splits.try_into_flat_map().expect("absent entries skipped");
        assert!(!flat.contains_key(b"key-a".as_slice()));
        assert_eq!(flat.get(b"key-b".as_slice()), Some(&5));
    }
}

// No generic `FromProof<Q>` impl — callers reach this through
// `FromProof<DocumentQuery> for DocumentSplitSums` in
// `rs-sdk/src/platform/documents/document_split_sums.rs`. Same
// rationale as `DocumentSum` / `DocumentSplitCounts`: per-mode
// proof dispatch needs the SDK's `DocumentQuery` shape.
