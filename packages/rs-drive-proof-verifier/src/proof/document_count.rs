use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsCountResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::grovedb::GroveDb;
use drive::query::{DriveDocumentQuery, PathQuery};

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
/// Counterpart to the materialize-and-count path in the
/// [`FromProof<DriveDocumentQuery> for DocumentCount`] impl above:
/// where that path verifies a regular grovedb proof that yields
/// concrete documents and counts them client-side, this verifies the
/// merk-level aggregate primitive that yields a single u64 directly
/// (capped only by the merk tree size, not `u16::MAX`).
///
/// Caller is expected to build `path_query` via
/// [`drive::query::DriveDocumentCountQuery::aggregate_count_path_query`]
/// — the prover and verifier must produce the *exact same* `PathQuery`
/// for the merk root recomputation to match, so reusing that builder is
/// load-bearing.
pub fn verify_aggregate_count_proof(
    proof: &Proof,
    mtd: &ResponseMetadata,
    path_query: &PathQuery,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<u64, Error> {
    let (root_hash, count) = GroveDb::verify_aggregate_count_query(
        &proof.grovedb_proof,
        path_query,
        &platform_version.drive.grove_version,
    )
    .map_err(|e| Error::GroveDBError {
        proof_bytes: proof.grovedb_proof.clone(),
        path_query: Some(path_query.clone()),
        height: mtd.height,
        time_ms: mtd.time_ms,
        error: e.to_string(),
    })?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(count)
}

/// A single verified `(in_key, key, count)` triple from a distinct-
/// count proof. Mirrors `drive::query::SplitCountEntry`'s shape — see
/// that struct's doc comment for why the In dimension is preserved
/// instead of being merged client-side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSplitCount {
    /// The serialized In-prefix value for compound queries. `None`
    /// for flat queries with no `In` on prefix.
    pub in_key: Option<Vec<u8>>,
    /// The serialized terminator (range-property) value.
    pub key: Vec<u8>,
    /// The verified count for this `(in_key, key)` tuple.
    pub count: u64,
}

/// Verify a regular grovedb range proof against a `ProvableCountTree`
/// and the surrounding tenderdash commit, returning the verified
/// per-(in_key, key) counts the proof commits to.
///
/// Companion to [`verify_aggregate_count_proof`]: where that one
/// extracts a single `u64` via `AggregateCountOnRange`'s `HashWithCount`
/// collapse, this one walks the standard range proof (no opt-in
/// wrapper) and pulls the per-key counts out of the leaf merk's
/// `KVCount(key, value, count)` ops. Each `count` is bound to the merk
/// root via `node_hash_with_count(kv_hash, l_hash, r_hash, count)`, so
/// the standard hash-chain check is sufficient — once `verify_query`
/// returns `Ok`, every `count` we extract is cryptographically
/// committed to the same `root_hash` tenderdash signs.
///
/// Caller is expected to build `path_query` via
/// [`drive::query::DriveDocumentCountQuery::distinct_count_path_query`]
/// — the prover and verifier must agree on the exact path/range bytes
/// or the merk chain check fails.
///
/// ## No cross-fork merge
///
/// For compound queries (an `In` clause on a prefix property) each
/// emitted element retains its `in_key` (the In value for that fork)
/// alongside the terminator `key`. Cross-fork aggregation is
/// intentionally NOT done here — callers reduce by `key` client-side
/// if they want a flat histogram. This makes verification a near
/// pass-through over what `verify_query` returns, avoids the
/// pre-merge undercount that biases proofs when `limit` truncates
/// elements before the merge can run, and means a malicious server
/// omitting one whole `In` branch shows up as missing entries
/// (rather than as a silently-undersummed total).
///
/// ## Trade-off vs. the aggregate path
///
/// Proof size is O(distinct (in_key, terminator) pairs matched)
/// rather than O(log n), because each distinct in-range pair emits
/// its own `KVCount` op instead of being collapsed into a boundary
/// subtree. Still strictly smaller than materialize-and-count.
pub fn verify_distinct_count_proof(
    proof: &Proof,
    mtd: &ResponseMetadata,
    path_query: &PathQuery,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<VerifiedSplitCount>, Error> {
    // `GroveDb::verify_query` is appropriate here for both flat and
    // compound shapes:
    // - For flat queries (no `In` on prefix) the path query has a
    //   single range `QueryItem` and no explicit `Key` items; range
    //   items can't be enumerated for absence checks anyway
    //   (`Query::terminal_keys_inner` errors `NotSupported` on
    //   unbounded ranges).
    // - For compound queries (`In` on prefix) the outer Query has
    //   explicit `Key` items per In value, but because we no longer
    //   sum across forks, a missing `Key` branch surfaces as missing
    //   entries with that `in_key` rather than as a wrong total —
    //   the caller can detect "I asked for 3 In values but only got
    //   entries for 2" directly. We do NOT need
    //   `absence_proofs_for_non_existing_searched_keys: true` for
    //   correctness here; it would be a useful future addition for
    //   "prove this In value has zero entries" but isn't required
    //   to make distinct-count proofs sound.
    //
    // `verify_proof_succinctness: true` (the default) is kept so
    // proofs with unrequested extra subtree data are still rejected.
    let (root_hash, elements) = GroveDb::verify_query(
        &proof.grovedb_proof,
        path_query,
        &platform_version.drive.grove_version,
    )
    .map_err(|e| Error::GroveDBError {
        proof_bytes: proof.grovedb_proof.clone(),
        path_query: Some(path_query.clone()),
        height: mtd.height,
        time_ms: mtd.time_ms,
        error: e.to_string(),
    })?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    // Convert `(path, key, Option<Element>)` triples into
    // `VerifiedSplitCount`. For compound queries the In value sits at
    // `path[base_path_len]` (the first extra path segment beyond the
    // path query's `path`); for flat queries the emitted path equals
    // `path_query.path` so the in_key is `None`.
    let base_path_len = path_query.path.len();
    let mut out: Vec<VerifiedSplitCount> = Vec::with_capacity(elements.len());
    for (path, key, elem) in elements {
        if let Some(e) = elem {
            let count = e.count_value_or_default();
            if count == 0 {
                continue;
            }
            let in_key = if path.len() > base_path_len {
                Some(path[base_path_len].clone())
            } else {
                None
            };
            out.push(VerifiedSplitCount { in_key, key, count });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    //! Local-only tests for parts of this module that don't need a
    //! real grovedb proof or a populated Drive. The full happy-path
    //! verification of `verify_aggregate_count_proof` /
    //! `verify_distinct_count_proof` is covered end-to-end in the
    //! drive crate's range_countable_index_e2e_tests (where the
    //! prover and verifier roundtrip on a real Drive), and in the
    //! rs-sdk integration tests. Here we cover:
    //!
    //! - `VerifiedSplitCount` struct invariants (constructor /
    //!   equality / clone).
    //! - The error-mapping branch of `verify_aggregate_count_proof`
    //!   and `verify_distinct_count_proof` for garbage proof bytes
    //!   (the `map_err` chain into `Error::GroveDBError`).
    use super::*;
    use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use drive::query::PathQuery;
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

    /// Builds an arbitrary PathQuery — the verify happy path needs a
    /// real proof generated against this exact path query, but for
    /// the error-mapping path the contents don't matter: we want
    /// grovedb-side verification to fail and the error to be
    /// wrapped in `Error::GroveDBError`.
    fn arbitrary_path_query() -> PathQuery {
        use drive::grovedb::{Query, SizedQuery};
        let query = Query::new();
        PathQuery::new(vec![vec![0u8]], SizedQuery::new(query, None, None))
    }

    fn arbitrary_metadata() -> ResponseMetadata {
        ResponseMetadata {
            height: 1,
            time_ms: 0,
            ..Default::default()
        }
    }

    #[test]
    fn verified_split_count_struct_constructs_and_clones() {
        // Round-trip the struct fields through Clone + PartialEq to
        // pin the public-API shape and guard against accidental
        // field-order changes that would break call sites pattern-
        // matching on it.
        let a = VerifiedSplitCount {
            in_key: Some(b"acme".to_vec()),
            key: b"red".to_vec(),
            count: 42,
        };
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(a.in_key.as_deref(), Some(b"acme".as_slice()));
        assert_eq!(a.key, b"red".to_vec());
        assert_eq!(a.count, 42);

        // Flat-query variant: in_key absent.
        let flat = VerifiedSplitCount {
            in_key: None,
            key: b"green".to_vec(),
            count: 7,
        };
        assert!(flat.in_key.is_none());
        assert_eq!(flat.key, b"green".to_vec());
        assert_eq!(flat.count, 7);

        // Inequality across each dimension.
        let different_in_key = VerifiedSplitCount {
            in_key: Some(b"contoso".to_vec()),
            ..a.clone()
        };
        assert_ne!(a, different_in_key);
        let different_key = VerifiedSplitCount {
            key: b"blue".to_vec(),
            ..a.clone()
        };
        assert_ne!(a, different_key);
        let different_count = VerifiedSplitCount { count: 99, ..a };
        assert_ne!(b, different_count);
    }

    #[test]
    fn verify_aggregate_count_proof_garbage_bytes_returns_grovedb_error() {
        // Garbage bytes can't decode as a valid AggregateCountOnRange
        // proof envelope. The error-mapping branch wraps the grovedb
        // error in `Error::GroveDBError` and surfaces the original
        // request metadata (height/time_ms) plus the path query so
        // callers can correlate it with their request.
        let proof = Proof {
            grovedb_proof: vec![0xffu8; 16],
            ..Default::default()
        };
        let mtd = arbitrary_metadata();
        let path_query = arbitrary_path_query();
        let err = verify_aggregate_count_proof(
            &proof,
            &mtd,
            &path_query,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::GroveDBError {
                proof_bytes,
                path_query: pq,
                height,
                time_ms,
                ..
            } => {
                assert_eq!(proof_bytes, vec![0xffu8; 16]);
                assert!(pq.is_some(), "path_query must be threaded into error");
                assert_eq!(height, mtd.height);
                assert_eq!(time_ms, mtd.time_ms);
            }
            other => panic!("expected GroveDBError, got: {other:?}"),
        }
    }

    #[test]
    fn verify_distinct_count_proof_garbage_bytes_returns_grovedb_error() {
        // Same error-mapping path as the aggregate helper above —
        // pin it independently so a future refactor that decouples
        // the two helpers can't silently regress one.
        let proof = Proof {
            grovedb_proof: vec![0xffu8; 16],
            ..Default::default()
        };
        let mtd = arbitrary_metadata();
        let path_query = arbitrary_path_query();
        let err = verify_distinct_count_proof(
            &proof,
            &mtd,
            &path_query,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::GroveDBError {
                proof_bytes,
                path_query: pq,
                height,
                time_ms,
                ..
            } => {
                assert_eq!(proof_bytes, vec![0xffu8; 16]);
                assert!(pq.is_some());
                assert_eq!(height, mtd.height);
                assert_eq!(time_ms, mtd.time_ms);
            }
            other => panic!("expected GroveDBError, got: {other:?}"),
        }
    }

    #[test]
    fn verify_aggregate_count_proof_empty_bytes_returns_grovedb_error() {
        // Empty bytes are a distinct decoding failure mode from
        // garbage bytes — exercise the same error mapping with a
        // different grovedb-side rejection cause.
        let proof = Proof {
            grovedb_proof: Vec::new(),
            ..Default::default()
        };
        let mtd = arbitrary_metadata();
        let path_query = arbitrary_path_query();
        let err = verify_aggregate_count_proof(
            &proof,
            &mtd,
            &path_query,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::GroveDBError { .. }));
    }
}
