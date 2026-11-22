//! Verified per-entry average result.
//!
//! Average-side analog of [`super::document_split_sum::DocumentSplitSums`].
//! Holds one verified `(in_key, key, count, sum)` 4-tuple per matched
//! group — the client divides each `sum / count` to compute per-group
//! averages. Returned by `select=AVG, group_by=[...]` queries;
//! aggregate averages use [`super::document_average::DocumentAverage`]
//! instead.
//!
//! The generic `FromProof<Q>` impl below intentionally rejects
//! calls (matching [`super::document_split_count::DocumentSplitCounts`]'s
//! pattern). Real dispatch lives in the
//! `FromProof<DocumentQuery>` impl in
//! `rs-sdk/src/platform/documents/document_split_averages.rs`,
//! which picks the right per-shape verifier (PCPS carrier-aggregate
//! / primary-key direct read) based on the resolved
//! `DocumentAverageMode`.

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
    /// Aggregated sum at that key, at the 128 bits a sum is kept in
    /// ([`super::document_sum::DocumentSum`]). `Some(n)` for matched
    /// keys; `None` for keys proven absent.
    pub sum: Option<i128>,
}

impl SplitAverageEntry {
    /// Convenience: compute the average for this entry as `f64`, or
    /// `None` if the entry was proven absent (`count`/`sum` is `None`)
    /// or has zero count. A large sum loses its low digits in an
    /// `f64`; use [`Self::to_decimal_string`] for an exact average.
    pub fn as_f64(&self) -> Option<f64> {
        match (self.count, self.sum) {
            (Some(c), Some(s)) if c > 0 => Some(s as f64 / c as f64),
            _ => None,
        }
    }

    /// The average for this entry as a decimal string, exact to
    /// `fraction_digits` places
    /// ([`super::document_average::average_to_decimal_string`]), or
    /// `None` if the entry was proven absent or has zero count.
    pub fn to_decimal_string(&self, fraction_digits: u32) -> Option<String> {
        super::document_average::average_to_decimal_string(self.count?, self.sum?, fraction_digits)
    }
}

/// The full per-entry average result, verified from proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSplitAverages(pub Vec<SplitAverageEntry>);

impl DocumentSplitAverages {
    /// Convenience: collapse compound `(in_key, key)` entries into a
    /// flat `BTreeMap<key, (summed_count, summed_sum)>` by combining
    /// each In-fork's contribution at the same terminator key.
    /// Mirrors [`super::document_split_sum::DocumentSplitSums::try_into_flat_map`].
    ///
    /// Uses [`u64::checked_add`] on the count axis and
    /// [`i128::checked_add`] on the sum axis; overflow on either
    /// surfaces as [`Error::RequestError`](crate::Error::RequestError)
    /// rather than panicking (debug) or wrapping (release).
    /// Mirrors the SDK's aggregate-side
    /// `DocumentAverage::fold_average_entries`, which hardens the
    /// same two axes for the single-aggregate response shape.
    ///
    /// On overflow, drop back to iterating `DocumentSplitAverages.0`
    /// directly — per-branch entries preserve the verified
    /// `(u64, i128)` pair, and the caller can fold with its own
    /// arithmetic.
    pub fn try_into_flat_map(
        self,
    ) -> Result<std::collections::BTreeMap<Vec<u8>, (u64, i128)>, crate::Error> {
        let mut out: std::collections::BTreeMap<Vec<u8>, (u64, i128)> =
            std::collections::BTreeMap::new();
        for entry in self.0 {
            if let (Some(c), Some(s)) = (entry.count, entry.sum) {
                let acc = out.entry(entry.key).or_insert((0u64, 0i128));
                acc.0 = acc
                    .0
                    .checked_add(c)
                    .ok_or_else(|| crate::Error::RequestError {
                        error: "DocumentSplitAverages::try_into_flat_map: u64 overflow merging \
                         per-In-fork counts at the same terminator key. Iterate \
                         DocumentSplitAverages.0 directly to access per-branch counts and \
                         fold with your own arithmetic."
                            .to_string(),
                    })?;
                acc.1 = acc
                    .1
                    .checked_add(s)
                    .ok_or_else(|| crate::Error::RequestError {
                        error: "DocumentSplitAverages::try_into_flat_map: i128 overflow merging \
                         per-In-fork sums at the same terminator key. Iterate \
                         DocumentSplitAverages.0 directly to access per-branch sums and fold \
                         with your own arithmetic."
                            .to_string(),
                    })?;
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Single-fork pass-through: count + sum preserved unchanged.
    #[test]
    fn try_into_flat_map_passes_through_flat_entries() {
        let splits = DocumentSplitAverages(vec![SplitAverageEntry {
            in_key: None,
            key: b"key-a".to_vec(),
            count: Some(3),
            sum: Some(42),
        }]);
        let flat = splits.try_into_flat_map().expect("non-overflowing fold");
        assert_eq!(flat.get(b"key-a".as_slice()), Some(&(3u64, 42i128)));
    }

    /// Two In-forks at the same terminator key fold both axes.
    #[test]
    fn try_into_flat_map_sums_across_in_forks() {
        let splits = DocumentSplitAverages(vec![
            SplitAverageEntry {
                in_key: Some(b"alice".to_vec()),
                key: b"red".to_vec(),
                count: Some(2),
                sum: Some(10),
            },
            SplitAverageEntry {
                in_key: Some(b"bob".to_vec()),
                key: b"red".to_vec(),
                count: Some(5),
                sum: Some(32),
            },
        ]);
        let flat = splits.try_into_flat_map().expect("non-overflowing fold");
        assert_eq!(flat.get(b"red".as_slice()), Some(&(7u64, 42i128)));
    }

    /// `u64::MAX + 1` on count surfaces a `RequestError` (not
    /// panic / wrap). Independent of the sum axis.
    #[test]
    fn try_into_flat_map_count_overflow_surfaces_request_error() {
        let splits = DocumentSplitAverages(vec![
            SplitAverageEntry {
                in_key: Some(b"alice".to_vec()),
                key: b"shared".to_vec(),
                count: Some(u64::MAX),
                sum: Some(0),
            },
            SplitAverageEntry {
                in_key: Some(b"bob".to_vec()),
                key: b"shared".to_vec(),
                count: Some(1),
                sum: Some(0),
            },
        ]);
        let err = splits
            .try_into_flat_map()
            .expect_err("u64::MAX + 1 on the count axis must surface as overflow error");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("u64 overflow") && msg.contains("DocumentSplitAverages"),
            "error must name the count-axis overflow and the helper: got {msg}"
        );
    }

    /// Two In-forks whose sums together pass 64 bits fold to the
    /// whole 128-bit sum, and average exactly.
    #[test]
    fn try_into_flat_map_folds_past_64_bits() {
        let splits = DocumentSplitAverages(vec![
            SplitAverageEntry {
                in_key: Some(b"alice".to_vec()),
                key: b"shared".to_vec(),
                count: Some(1),
                sum: Some(i128::from(i64::MAX)),
            },
            SplitAverageEntry {
                in_key: Some(b"bob".to_vec()),
                key: b"shared".to_vec(),
                count: Some(1),
                sum: Some(i128::from(i64::MAX)),
            },
        ]);
        let flat = splits.try_into_flat_map().expect("the fold fits 128 bits");
        let (count, sum) = flat[b"shared".as_slice()];
        assert_eq!((count, sum), (2, 2 * i128::from(i64::MAX)));
        let entry = SplitAverageEntry {
            in_key: None,
            key: b"shared".to_vec(),
            count: Some(count),
            sum: Some(sum),
        };
        assert_eq!(
            entry.to_decimal_string(4).as_deref(),
            Some("9223372036854775807.0000")
        );
    }

    /// `i128::MAX + 1` on sum surfaces a `RequestError` independently
    /// of count.
    #[test]
    fn try_into_flat_map_sum_overflow_surfaces_request_error() {
        let splits = DocumentSplitAverages(vec![
            SplitAverageEntry {
                in_key: Some(b"alice".to_vec()),
                key: b"shared".to_vec(),
                count: Some(0),
                sum: Some(i128::MAX),
            },
            SplitAverageEntry {
                in_key: Some(b"bob".to_vec()),
                key: b"shared".to_vec(),
                count: Some(0),
                sum: Some(1),
            },
        ]);
        let err = splits
            .try_into_flat_map()
            .expect_err("i128::MAX + 1 on the sum axis must surface as overflow error");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("i128 overflow") && msg.contains("DocumentSplitAverages"),
            "error must name the sum-axis overflow and the helper: got {msg}"
        );
    }

    /// An entry where EITHER half is `None` (proven absent) is
    /// skipped entirely — must not poison the accumulator.
    #[test]
    fn try_into_flat_map_skips_partial_or_absent_entries() {
        let splits = DocumentSplitAverages(vec![
            SplitAverageEntry {
                in_key: None,
                key: b"present".to_vec(),
                count: Some(2),
                sum: Some(5),
            },
            SplitAverageEntry {
                in_key: None,
                key: b"absent".to_vec(),
                count: None,
                sum: Some(99),
            },
            SplitAverageEntry {
                in_key: None,
                key: b"absent".to_vec(),
                count: Some(99),
                sum: None,
            },
        ]);
        let flat = splits.try_into_flat_map().expect("partial entries skipped");
        assert_eq!(flat.get(b"present".as_slice()), Some(&(2u64, 5i128)));
        assert!(!flat.contains_key(b"absent".as_slice()));
    }
}

// No generic `FromProof<Q>` impl — callers reach this through
// `FromProof<DocumentQuery> for DocumentSplitAverages` in
// `rs-sdk/src/platform/documents/document_split_averages.rs`.
