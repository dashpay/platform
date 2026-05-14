use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::query::{DriveDocumentQuery, SplitCountEntry};
use std::collections::BTreeMap;

/// The split counts of documents matching a query, verified from proof.
///
/// Each entry carries the serialized split-property value (`key`) as
/// produced by
/// [`DocumentTypeBasicMethods::serialize_value_for_key`], the verified
/// `count`, and an optional `in_key` carrying the In-prefix value for
/// compound range-distinct queries (see the [`SplitCountEntry`]
/// doc for rationale on why compound results stay unmerged).
///
/// For flat queries (per-`In`-value mode without a range, or per-
/// distinct-value-in-range mode without an `In` on prefix) every
/// entry's `in_key` is `None`. Callers can recover the historical
/// `BTreeMap<Vec<u8>, u64>` shape by collecting `(key, count)` pairs
/// — see [`Self::into_flat_map`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentSplitCounts(pub Vec<SplitCountEntry>);

impl DocumentSplitCounts {
    /// Collect entries into a `BTreeMap<Vec<u8>, u64>` keyed by the
    /// terminator `key`, summing across `in_key` forks. Use this when
    /// the caller wants the merged-histogram view of a compound
    /// query (or for backwards compatibility with the pre-no-merge
    /// API shape). Flat queries pass through unchanged.
    pub fn into_flat_map(self) -> BTreeMap<Vec<u8>, u64> {
        let mut out: BTreeMap<Vec<u8>, u64> = BTreeMap::new();
        for entry in self.0 {
            *out.entry(entry.key).or_insert(0) += entry.count.unwrap_or(0);
        }
        out
    }

    /// Build a [`DocumentSplitCounts`] from a verifier-side
    /// `Vec<SplitCountEntry>`. Identity for now; kept as a
    /// constructor in case the internal shape evolves.
    pub fn from_verified(entries: Vec<SplitCountEntry>) -> Self {
        DocumentSplitCounts(entries)
    }
}

/// Reject the generic [`FromProof`] entry point for [`DocumentSplitCounts`].
///
/// `DocumentSplitCounts` is reached from rs-sdk via the
/// `FromProof<DocumentQuery>` impl defined alongside the SDK's
/// `DocumentQuery` type (see
/// `rs-sdk/src/platform/documents/document_count.rs`), which
/// dispatches to the right proof shape (CountTree element /
/// aggregate-count / distinct-count) based on
/// `(group_by, where_clauses, prove)`. The generic
/// `FromProof<Q: TryInto<DriveDocumentQuery>>` path doesn't carry
/// enough information to pick a proof shape, so it errors out
/// explicitly — calling this impl directly is a programmer mistake.
impl<'dq, Q> FromProof<Q> for DocumentSplitCounts
where
    Q: TryInto<DriveDocumentQuery<'dq>> + Clone + 'dq,
    Q::Error: std::fmt::Display,
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
        Err(Error::RequestError {
            error: "DocumentSplitCounts can't be verified via the generic FromProof path; \
                 call DocumentSplitCounts::fetch on a DocumentQuery with .with_select(Count), \
                 which routes through the right proof shape (CountTree element / aggregate / \
                 distinct) based on the request"
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    //! Local-only tests for the parts of `DocumentSplitCounts` that
    //! don't need a real grovedb proof or a populated Drive:
    //!
    //! - `into_flat_map` — pure data reduction over the new
    //!   `Vec<SplitCountEntry>` shape (covers the no-merge →
    //!   merged-histogram backwards-compat path).
    //! - `from_verified` — identity constructor wrapping the raw
    //!   verified-entries vec.
    //! - The generic `FromProof<Q>` impl that intentionally errors
    //!   to prevent the silently-empty footgun documented above.
    //!
    //! The actual proof verification (CountTree-element /
    //! aggregate / distinct) is exercised end-to-end by drive's
    //! `range_countable_index_e2e_tests` (running the prover and
    //! verifier on a real Drive); exercising it here would need a
    //! populated Drive + a real proof, which is outside this
    //! crate's feature surface.
    use super::*;

    /// Helper to make a `SplitCountEntry` with the given fields
    /// without each call site needing to type the struct out.
    fn entry(in_key: Option<&[u8]>, key: &[u8], count: u64) -> SplitCountEntry {
        SplitCountEntry {
            in_key: in_key.map(|s| s.to_vec()),
            key: key.to_vec(),
            // Test helper always builds verified entries; `None`
            // entries (caller asked but verifier was silent) are
            // tested via explicit struct construction at the SDK
            // synthesis call site, not through this helper.
            count: Some(count),
        }
    }

    #[test]
    fn from_verified_round_trips_the_input_vec() {
        let entries = vec![
            entry(None, b"red", 5),
            entry(None, b"green", 3),
            entry(None, b"blue", 8),
        ];
        let counts = DocumentSplitCounts::from_verified(entries.clone());
        assert_eq!(counts.0, entries);
    }

    #[test]
    fn from_verified_empty_round_trip() {
        let counts = DocumentSplitCounts::from_verified(Vec::new());
        assert!(counts.0.is_empty());
    }

    #[test]
    fn into_flat_map_passes_through_flat_entries() {
        // No In dimension — every entry has `in_key = None`. The flat
        // map should be one-to-one with the input.
        let counts = DocumentSplitCounts::from_verified(vec![
            entry(None, b"red", 5),
            entry(None, b"green", 3),
            entry(None, b"blue", 8),
        ]);
        let flat = counts.into_flat_map();
        assert_eq!(flat.len(), 3);
        assert_eq!(flat.get(b"red".as_slice()), Some(&5));
        assert_eq!(flat.get(b"green".as_slice()), Some(&3));
        assert_eq!(flat.get(b"blue".as_slice()), Some(&8));
    }

    #[test]
    fn into_flat_map_sums_across_in_key_forks_for_compound_entries() {
        // Compound query result: `brand in [acme, contoso]` × `color in [red, green]`.
        // `into_flat_map` should sum `red` across both brand forks
        // (3 + 2 = 5) — that's the whole point of providing the
        // historical merged-histogram view.
        let counts = DocumentSplitCounts::from_verified(vec![
            entry(Some(b"acme"), b"red", 3),
            entry(Some(b"acme"), b"green", 2),
            entry(Some(b"contoso"), b"red", 2),
            entry(Some(b"contoso"), b"green", 4),
        ]);
        let flat = counts.into_flat_map();
        assert_eq!(flat.len(), 2, "merges by `key` across in_key forks");
        assert_eq!(flat.get(b"red".as_slice()), Some(&5));
        assert_eq!(flat.get(b"green".as_slice()), Some(&6));
    }

    #[test]
    fn into_flat_map_handles_mixed_in_key_and_none_entries() {
        // Edge case: a result set that mixes flat entries (in_key=None)
        // and compound entries (in_key=Some). Both should fold into
        // the same `key` buckets when sharing a terminator value.
        let counts = DocumentSplitCounts::from_verified(vec![
            entry(None, b"red", 1),
            entry(Some(b"acme"), b"red", 2),
            entry(Some(b"contoso"), b"red", 3),
            entry(Some(b"acme"), b"green", 4),
        ]);
        let flat = counts.into_flat_map();
        assert_eq!(flat.get(b"red".as_slice()), Some(&6));
        assert_eq!(flat.get(b"green".as_slice()), Some(&4));
    }

    #[test]
    fn into_flat_map_empty_input_produces_empty_map() {
        let counts = DocumentSplitCounts::from_verified(Vec::new());
        assert!(counts.into_flat_map().is_empty());
    }

    // The generic `FromProof` rejection (returning the explicit
    // "needs a split property" error rather than silently returning
    // `Some(empty)`) is covered by the SDK integration tests, which
    // can construct a valid `DriveDocumentQuery` via dpp's
    // `fixtures-and-mocks` feature. drive-proof-verifier itself
    // doesn't depend on `dpp/fixtures-and-mocks` so we can't build
    // one here.
}
