use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsCountResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::grovedb::{GroveDb, VerifyOptions};
use drive::query::{DriveDocumentQuery, PathQuery};
use std::collections::BTreeMap;

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

/// Verify a regular grovedb range proof against a `ProvableCountTree`
/// and the surrounding tenderdash commit, returning the per-distinct-
/// value counts the proof commits to.
///
/// Companion to [`verify_aggregate_count_proof`]: where that one
/// extracts a single `u64` via `AggregateCountOnRange`'s `HashWithCount`
/// collapse, this one walks the standard range proof (no opt-in
/// wrapper) and pulls the per-key counts out of the leaf merk's
/// `KVCount(key, value, count)` ops. Each `count` is bound to the merk
/// root via `node_hash_with_count(kv_hash, l_hash, r_hash, count)`, so
/// the standard hash-chain check
/// (`GroveDb::verify_query_with_options`) is sufficient — once that
/// returns `Ok`, every `count` we extract is cryptographically
/// committed to the same `root_hash` tenderdash signs.
///
/// Caller is expected to build `path_query` via
/// [`drive::query::DriveDocumentCountQuery::distinct_count_path_query`]
/// — the prover and verifier must agree on the exact path/range bytes
/// or the merk chain check fails.
///
/// Trade-off vs. the aggregate path: proof size is O(distinct values
/// matched) rather than O(log n), because each distinct in-range key
/// emits its own `KVCount` op instead of being collapsed into a
/// boundary subtree.
pub fn verify_distinct_count_proof(
    proof: &Proof,
    mtd: &ResponseMetadata,
    path_query: &PathQuery,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<BTreeMap<Vec<u8>, u64>, Error> {
    // Standard verifier does the hash-chain check leaf-merk →
    // multi-layer envelope → GroveDB root, AND returns the matched
    // elements already deserialized. Each element is the per-value
    // `CountTree(_, lot_count, _)` whose count we want — read it via
    // `count_value_or_default()` and we're done. The returned counts
    // are bound to `root_hash` through the same hash chain
    // `verify_query_with_options` just validated.
    //
    // Disable `absence_proofs_for_non_existing_searched_keys` (the
    // default `true` would require a `limit` on the path query — but
    // distinct-count path queries don't carry one, the result is
    // bounded by the range itself, and "absence proofs" only apply to
    // explicit `Query::Key(k)` items which a range-only query has
    // none of). Keep `verify_proof_succinctness: true` (default) so
    // proofs with unrequested extra subtree data are rejected.
    let verify_options = VerifyOptions {
        absence_proofs_for_non_existing_searched_keys: false,
        ..VerifyOptions::default()
    };
    let (root_hash, elements) = GroveDb::verify_query_with_options(
        &proof.grovedb_proof,
        path_query,
        verify_options,
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

    Ok(elements
        .into_iter()
        .filter_map(|(_path, key, elem)| elem.map(|e| (key, e.count_value_or_default())))
        .collect())
}
