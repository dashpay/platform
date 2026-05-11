use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsCountResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::Document;
use dpp::document::DocumentV0Getters;
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
            *out.entry(entry.key).or_insert(0) += entry.count;
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
/// Splitting requires the split-property name, which isn't carried by
/// `DriveDocumentQuery`. Earlier versions of this impl silently returned an
/// empty map under proof, which made `prove=true` callers think there were
/// zero documents per group. To stop that footgun, the generic
/// [`FromProof`] now returns an explicit error; SDK-level callers must use
/// [`DocumentSplitCounts::maybe_from_proof_with_split_property`] (or, in
/// `rs-sdk`, the [`Fetch`](dash_sdk::platform::Fetch) impl on
/// `DocumentSplitCountQuery`) which threads the split property through.
impl<'dq, Q> FromProof<Q> for DocumentSplitCounts
where
    Q: TryInto<DriveDocumentQuery<'dq>> + Clone + 'dq,
    Q::Error: std::fmt::Display,
{
    type Request = Q;
    type Response = GetDocumentsCountResponse;

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
            error: "DocumentSplitCounts requires a split-property; call \
                 DocumentSplitCounts::maybe_from_proof_with_split_property \
                 (or use the rs-sdk Fetch impl on DocumentSplitCountQuery)"
                .to_string(),
        })
    }
}

impl DocumentSplitCounts {
    /// Verify a `GetDocumentsCount` proof and aggregate the verified
    /// documents into per-key counts using `split_property` as the grouping
    /// key.
    ///
    /// `Q` is anything that can be turned into a [`DriveDocumentQuery`] —
    /// typically a `DocumentSplitCountQuery` from `rs-sdk` or a
    /// `DriveDocumentQuery` directly.
    ///
    /// Returns `(Some(splits), metadata, proof)` even when no documents
    /// matched (in which case `splits.0` is empty).
    pub fn maybe_from_proof_with_split_property<'dq, 'a, Q, I, O>(
        request: I,
        split_property: &str,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Q: TryInto<DriveDocumentQuery<'dq>> + Clone + 'dq,
        Q::Error: std::fmt::Display,
        I: Into<Q>,
        O: Into<GetDocumentsCountResponse>,
        Self: 'a,
    {
        let request: Q = request.into();
        let response: GetDocumentsCountResponse = response.into();

        let drive_query: DriveDocumentQuery<'dq> =
            request
                .clone()
                .try_into()
                .map_err(|e: Q::Error| Error::RequestError {
                    error: e.to_string(),
                })?;

        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, documents) = drive_query
            .verify_proof(&proof.grovedb_proof, platform_version)
            .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let aggregated = aggregate_documents_by_property(
            &documents,
            drive_query.document_type,
            split_property,
            platform_version,
        )?;

        // PerInValue mode (materialize-and-count path) has no In
        // dimension distinct from the value being counted — the
        // split property IS the In field. So `in_key = None` and
        // `key = serialized In value` per SplitCountEntry's flat
        // convention.
        let entries: Vec<SplitCountEntry> = aggregated
            .into_iter()
            .map(|(key, count)| SplitCountEntry {
                in_key: None,
                key,
                count,
            })
            .collect();

        Ok((
            Some(DocumentSplitCounts(entries)),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

/// Group documents by the byte-encoded value of `split_property` and return
/// the per-key counts. Documents that don't carry the property are skipped
/// (mirroring the server-side CountTree path, which only counts documents
/// whose primary-key tree path includes the property).
fn aggregate_documents_by_property(
    documents: &[Document],
    document_type: dpp::data_contract::document_type::DocumentTypeRef<'_>,
    split_property: &str,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<Vec<u8>, u64>, Error> {
    let mut counts: BTreeMap<Vec<u8>, u64> = BTreeMap::new();

    for document in documents {
        let value = match document.properties().get(split_property) {
            Some(v) => v,
            None => continue,
        };

        let key = document_type
            .serialize_value_for_key(split_property, value, platform_version)
            .map_err(|e| Error::ResponseDecodeError {
                error: format!(
                    "Failed to serialize split property `{}` for grouping: {}",
                    split_property, e
                ),
            })?;

        *counts.entry(key).or_insert(0) += 1;
    }

    Ok(counts)
}

// Aggregation unit tests live in higher-level crates with full test fixtures:
//   - SDK: packages/rs-sdk/tests/fetch/document_split_count.rs
//   - drive-abci: src/query/document_split_count_query/v0/mod.rs tests
// (drive-proof-verifier's feature surface doesn't expose dpp test helpers)
//
// Below are unit tests that don't require a real `DriveDocumentQuery`
// or a populated Drive — they cover the helpers and the
// generic-`FromProof`-rejection footgun guard.

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
    //! The actual `maybe_from_proof_with_split_property` flow is
    //! covered by the SDK integration tests at
    //! `packages/rs-sdk/tests/fetch/document_split_count.rs` —
    //! exercising it here would need a populated Drive + a real
    //! proof, which is outside this crate's feature surface.
    use super::*;

    /// Helper to make a `SplitCountEntry` with the given fields
    /// without each call site needing to type the struct out.
    fn entry(in_key: Option<&[u8]>, key: &[u8], count: u64) -> SplitCountEntry {
        SplitCountEntry {
            in_key: in_key.map(|s| s.to_vec()),
            key: key.to_vec(),
            count,
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
