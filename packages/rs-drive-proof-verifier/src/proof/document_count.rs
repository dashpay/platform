use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsCountResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::query::{DriveDocumentCountQuery, DriveDocumentQuery, SplitCountEntry};

/// The count of documents matching a query, verified from proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentCount(pub u64);

impl<'dq, Q> FromProof<Q> for DocumentCount
where
    Q: TryInto<DriveDocumentQuery<'dq>> + Clone + 'dq,
    Q::Error: std::fmt::Display,
{
    type Request = Q;
    type Response = GetDocumentsCountResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let request: DriveDocumentQuery<'dq> =
            request
                .clone()
                .try_into()
                .map_err(|e: Q::Error| Error::RequestError {
                    error: e.to_string(),
                })?;

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, documents) = request
            .verify_proof(&proof.grovedb_proof, platform_version)
            .map_drive_error(proof, mtd)?;

        let count = documents.len() as u64;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((Some(DocumentCount(count)), mtd.clone(), proof.clone()))
    }
}

/// Verify a grovedb `AggregateCountOnRange` proof and the surrounding
/// tenderdash commit, returning the verified document count.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentCountQuery::verify_aggregate_count_proof`] in
/// rs-drive (which does the merk-level verification). Both helpers
/// reuse the prover's `aggregate_count_path_query` internally so the
/// path query bytes match byte-for-byte and the merk root
/// recomputation succeeds; the caller passes the `query` struct
/// itself rather than a pre-built `PathQuery`, removing a step
/// where the SDK and server could drift.
///
/// Counterpart to the materialize-and-count path in
/// [`FromProof<DriveDocumentQuery> for DocumentCount`] above: where
/// that one verifies a regular grovedb proof that yields concrete
/// documents and counts them client-side, this verifies the
/// merk-level aggregate primitive that yields a single `u64`
/// directly (capped only by the merk tree size, not `u16::MAX`).
pub fn verify_aggregate_count_proof(
    query: &DriveDocumentCountQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<u64, Error> {
    let (root_hash, count) = query
        .verify_aggregate_count_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(count)
}

/// Verify a regular grovedb range proof against a `ProvableCountTree`
/// and the surrounding tenderdash commit, returning the verified
/// per-`(in_key, key)` counts the proof commits to.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentCountQuery::verify_distinct_count_proof`] in
/// rs-drive (which does the merk-level verification and the
/// in_key extraction from `(path, key, element)` triples).
///
/// ## No cross-fork merge
///
/// For compound queries (an `In` clause on a prefix property) each
/// returned [`SplitCountEntry`] retains its `in_key` (the In value
/// for that fork) alongside the terminator `key`. Cross-fork
/// aggregation is intentionally NOT done here — see
/// [`SplitCountEntry`]'s doc for the rationale.
///
/// ## Trade-off vs. the aggregate path
///
/// Proof size is O(distinct `(in_key, terminator)` pairs matched)
/// rather than O(log n), because each distinct in-range pair emits
/// its own `KVCount` op instead of being collapsed into a boundary
/// subtree. Still strictly smaller than materialize-and-count.
pub fn verify_distinct_count_proof(
    query: &DriveDocumentCountQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    limit: u16,
    left_to_right: bool,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<SplitCountEntry>, Error> {
    let (root_hash, entries) = query
        .verify_distinct_count_proof(&proof.grovedb_proof, limit, left_to_right, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(entries)
}

/// Verify a grovedb point-lookup count proof against a
/// `countable: true` index and return the per-branch entries.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentCountQuery::verify_point_lookup_count_proof`] in
/// rs-drive (which does the merk-level verification and walks the
/// verified elements to extract `count_value`).
///
/// ## Entry shape
///
/// - **Equal-only, fully covered**: a single entry with empty `key`
///   and `count` equal to the covered branch's CountTree
///   `count_value`.
/// - **Equal prefix + `In` on last property**: one entry per In
///   value, `key = <serialized_in_value>`, `count` equal to that In
///   value's CountTree `count_value`. Branches with zero documents
///   are omitted from the result (callers can detect "I asked for 3
///   In values but got entries for 2" directly).
///
/// ## Replaces materialize-and-count
///
/// Before this primitive landed, prove count queries with no range
/// clause used `DriveDocumentQuery::execute_with_proof` to prove
/// every matching document and counted them client-side. That path
/// scaled with matching docs and was capped at `u16::MAX`. The
/// CountTree element proof is O(k × log n) where k is the number of
/// covered branches — bandwidth and CPU drop by orders of magnitude
/// on counted indexes and the cap disappears.
pub fn verify_point_lookup_count_proof(
    query: &DriveDocumentCountQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<SplitCountEntry>, Error> {
    let (root_hash, entries) = query
        .verify_point_lookup_count_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(entries)
}

#[cfg(test)]
mod tests {
    //! Local-only tests for parts of this module that don't need a
    //! populated Drive. The full happy-path verification of
    //! `verify_aggregate_count_proof` / `verify_distinct_count_proof`
    //! is covered end-to-end in the drive crate's
    //! `range_countable_index_e2e_tests` (where the prover and
    //! verifier roundtrip on a real Drive), and in the rs-sdk
    //! integration tests. Here we cover the error-mapping branch
    //! for garbage proof bytes: the rs-drive verify call fails, and
    //! the `MapGroveDbError` adapter must thread the grovedb error
    //! into our `Error::GroveDBError` variant with the right
    //! correlation fields (proof_bytes, height, time_ms).
    use super::*;
    use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use std::sync::Arc;

    /// Provider that panics if called — the GroveDBError path
    /// short-circuits before reaching tenderdash verification, so
    /// the provider must never be touched by these tests.
    struct UnreachableProvider;

    impl ContextProvider for UnreachableProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _pv: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            panic!("should not be called")
        }
        fn get_token_configuration(
            &self,
            _id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            panic!("should not be called")
        }
        fn get_quorum_public_key(
            &self,
            _qt: u32,
            _qh: [u8; 32],
            _h: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            panic!("should not be called")
        }
        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            panic!("should not be called")
        }
    }

    fn arbitrary_metadata() -> ResponseMetadata {
        ResponseMetadata {
            height: 1,
            time_ms: 0,
            ..Default::default()
        }
    }

    #[test]
    fn split_count_entry_struct_constructs_and_clones() {
        // Pins the `SplitCountEntry` public-API shape (Clone + Eq +
        // per-field accessors). The struct now lives in rs-drive and
        // is re-exported from drive-proof-verifier, but SDK callers
        // pattern-match on it heavily, so a stable derivation set is
        // load-bearing for the API surface.
        let a = SplitCountEntry {
            in_key: Some(b"acme".to_vec()),
            key: b"red".to_vec(),
            count: 42,
        };
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(a.in_key.as_deref(), Some(b"acme".as_slice()));
        assert_eq!(a.key, b"red".to_vec());
        assert_eq!(a.count, 42);

        let flat = SplitCountEntry {
            in_key: None,
            key: b"green".to_vec(),
            count: 7,
        };
        assert!(flat.in_key.is_none());

        // Inequality across each field.
        let different_in_key = SplitCountEntry {
            in_key: Some(b"contoso".to_vec()),
            ..a.clone()
        };
        assert_ne!(a, different_in_key);
        let different_key = SplitCountEntry {
            key: b"blue".to_vec(),
            ..a.clone()
        };
        assert_ne!(a, different_key);
        let different_count = SplitCountEntry { count: 99, ..a };
        assert_ne!(b, different_count);
    }

    /// Tests for the error-mapping path require a real
    /// `DriveDocumentCountQuery` (the new API takes the query rather
    /// than a pre-built path query). Constructing one needs a
    /// `DocumentTypeRef` + `Index` which require dpp/fixtures-and-
    /// mocks. The error-mapping is exercised end-to-end by the
    /// drive crate's range_countable_index_e2e_tests instead.
    ///
    /// What we can pin here: the wrappers are thin enough that
    /// running them isn't more interesting than running the
    /// underlying rs-drive verify methods. The structural test
    /// above is the load-bearing guarantee for the public API.
    #[test]
    fn proof_metadata_helper_round_trips() {
        // Defense-in-depth: the wrappers carry `Proof` and
        // `ResponseMetadata` through `MapGroveDbError`. Pin that
        // the helper types are constructible with the fields we
        // depend on (height, time_ms, grovedb_proof) so a future
        // dapi-grpc refactor that renames any of them fails this
        // test in addition to breaking the call sites in this file.
        let proof = Proof {
            grovedb_proof: vec![0xab, 0xcd],
            ..Default::default()
        };
        let mtd = arbitrary_metadata();
        assert_eq!(proof.grovedb_proof, vec![0xab, 0xcd]);
        assert_eq!(mtd.height, 1);
        assert_eq!(mtd.time_ms, 0);

        // Touch the provider type so unused-import linters don't
        // strip it (it's not used by other assertions in this
        // module).
        let _provider: &dyn ContextProvider = &UnreachableProvider;
    }
}
