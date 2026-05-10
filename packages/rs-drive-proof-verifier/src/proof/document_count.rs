use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsCountResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::grovedb::operations::proof::{
    GroveDBProof, GroveDBProofV0, GroveDBProofV1, LayerProof, MerkOnlyLayerProof, ProofBytes,
};
use drive::grovedb::{
    Element, GroveDb, MerkProofDecoder, MerkProofNode, MerkProofOp, VerifyOptions,
};
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
    // 1. Standard verifier does the hash-chain check: leaf merk →
    //    multi-layer envelope → GroveDB root. The returned `root_hash`
    //    is what tenderdash signed, and every `KVCount` count inside
    //    the proof is bound to it via `node_hash_with_count`.
    //
    // We turn off `absence_proofs_for_non_existing_searched_keys` (the
    // default `true` would require a `limit` on the path query — but
    // distinct-count path queries don't carry one, the result is bounded
    // by the range itself) and `verify_proof_succinctness` (the proof
    // may cover boundary subtrees beyond the strict in-range matches —
    // grovedb's range walker emits AVL-ancestor nodes regardless of
    // whether their keys land in-range, and that's expected here).
    let verify_options = VerifyOptions {
        absence_proofs_for_non_existing_searched_keys: false,
        verify_proof_succinctness: false,
        include_empty_trees_in_result: false,
    };
    let (root_hash, _elements) = GroveDb::verify_query_with_options(
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

    // 2. Re-decode the envelope and walk to the leaf to pluck `KVCount`
    //    ops. Re-decoding is cheap (no I/O) and avoids a parallel
    //    grovedb-side API just for "give me the counts" — the
    //    integrity check above already proved every count is valid, so
    //    we're just reading.
    let config = bincode::config::standard()
        .with_big_endian()
        .with_limit::<{ 256 * 1024 * 1024 }>();
    let (envelope, _): (GroveDBProof, _) = bincode::decode_from_slice(&proof.grovedb_proof, config)
        .map_err(|e| Error::GroveDBError {
            proof_bytes: proof.grovedb_proof.clone(),
            path_query: Some(path_query.clone()),
            height: mtd.height,
            time_ms: mtd.time_ms,
            error: format!("envelope re-decode failed: {}", e),
        })?;

    let mut counts: BTreeMap<Vec<u8>, u64> = BTreeMap::new();
    let target_depth = path_query.path.len();

    fn collect_kv_counts(
        merk_bytes: &[u8],
        counts: &mut BTreeMap<Vec<u8>, u64>,
        proof_bytes: &[u8],
        path_query: &PathQuery,
        mtd: &ResponseMetadata,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for op in MerkProofDecoder::new(merk_bytes) {
            let op = op.map_err(|e| Error::GroveDBError {
                proof_bytes: proof_bytes.to_vec(),
                path_query: Some(path_query.clone()),
                height: mtd.height,
                time_ms: mtd.time_ms,
                error: format!("merk op decode failed: {}", e),
            })?;
            // The property-name layer of a `range_countable` index is
            // a `ProvableCountTree` whose children point to per-value
            // `CountTree` elements. merk emits these matched children
            // as either `KVValueHashFeatureType[WithChildHash]` ops
            // carrying the value bytes (the encoded `Element`) and the
            // AVL-aggregate count via `ProvableCountedMerkNode`.
            //
            // We deserialize the value bytes and read the *local* count
            // via `Element::count_value_or_default()` rather than using
            // the feature-type's count: the feature-type carries
            // `local + left_subtree + right_subtree` (the AVL aggregate
            // for hash recomputation), which conflates the per-lot
            // count with descendant lots' counts in the AVL. The local
            // count from the encoded `CountTree(_, count, _)` element
            // is exactly the per-distinct-value count we want.
            //
            // Both the value bytes and the `ProvableCountedMerkNode`
            // count are bound to the merk root via
            // `node_hash_with_count(kv_hash, l_hash, r_hash, agg_count)`
            // — the local count comes from the value bytes which feed
            // into `kv_hash`. Tampering with either fails the chain.
            let (key, value) = match op {
                MerkProofOp::Push(MerkProofNode::KVValueHashFeatureType(key, value, _, _)) => {
                    (key, value)
                }
                MerkProofOp::Push(MerkProofNode::KVValueHashFeatureTypeWithChildHash(
                    key,
                    value,
                    _,
                    _,
                    _,
                )) => (key, value),
                MerkProofOp::Push(MerkProofNode::KVCount(key, value, _)) => (key, value),
                _ => continue,
            };
            let elem = Element::deserialize(&value, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDBError {
                    proof_bytes: proof_bytes.to_vec(),
                    path_query: Some(path_query.clone()),
                    height: mtd.height,
                    time_ms: mtd.time_ms,
                    error: format!("element value deserialize failed: {}", e),
                })?;
            counts.insert(key, elem.count_value_or_default());
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_v0(
        layer: &MerkOnlyLayerProof,
        depth: usize,
        target: usize,
        path: &[Vec<u8>],
        counts: &mut BTreeMap<Vec<u8>, u64>,
        proof_bytes: &[u8],
        path_query: &PathQuery,
        mtd: &ResponseMetadata,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if depth == target {
            return collect_kv_counts(
                &layer.merk_proof,
                counts,
                proof_bytes,
                path_query,
                mtd,
                platform_version,
            );
        }
        let next_key = &path[depth];
        let lower = layer
            .lower_layers
            .get(next_key)
            .ok_or_else(|| Error::GroveDBError {
                proof_bytes: proof_bytes.to_vec(),
                path_query: Some(path_query.clone()),
                height: mtd.height,
                time_ms: mtd.time_ms,
                error: format!(
                    "distinct-count proof missing lower layer at depth {} for key 0x{}",
                    depth,
                    hex::encode(next_key)
                ),
            })?;
        walk_v0(
            lower,
            depth + 1,
            target,
            path,
            counts,
            proof_bytes,
            path_query,
            mtd,
            platform_version,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_v1(
        layer: &LayerProof,
        depth: usize,
        target: usize,
        path: &[Vec<u8>],
        counts: &mut BTreeMap<Vec<u8>, u64>,
        proof_bytes: &[u8],
        path_query: &PathQuery,
        mtd: &ResponseMetadata,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let merk_bytes = match &layer.merk_proof {
            ProofBytes::Merk(b) => b.as_slice(),
            other => {
                return Err(Error::GroveDBError {
                    proof_bytes: proof_bytes.to_vec(),
                    path_query: Some(path_query.clone()),
                    height: mtd.height,
                    time_ms: mtd.time_ms,
                    error: format!(
                        "distinct-count proof has non-merk leaf bytes at depth {}: {:?}",
                        depth,
                        std::mem::discriminant(other)
                    ),
                });
            }
        };
        if depth == target {
            return collect_kv_counts(
                merk_bytes,
                counts,
                proof_bytes,
                path_query,
                mtd,
                platform_version,
            );
        }
        let next_key = &path[depth];
        let lower = layer
            .lower_layers
            .get(next_key)
            .ok_or_else(|| Error::GroveDBError {
                proof_bytes: proof_bytes.to_vec(),
                path_query: Some(path_query.clone()),
                height: mtd.height,
                time_ms: mtd.time_ms,
                error: format!(
                    "distinct-count proof missing lower layer at depth {} for key 0x{}",
                    depth,
                    hex::encode(next_key)
                ),
            })?;
        walk_v1(
            lower,
            depth + 1,
            target,
            path,
            counts,
            proof_bytes,
            path_query,
            mtd,
            platform_version,
        )
    }

    match envelope {
        GroveDBProof::V0(GroveDBProofV0 { root_layer, .. }) => walk_v0(
            &root_layer,
            0,
            target_depth,
            &path_query.path,
            &mut counts,
            &proof.grovedb_proof,
            path_query,
            mtd,
            platform_version,
        )?,
        GroveDBProof::V1(GroveDBProofV1 { root_layer }) => walk_v1(
            &root_layer,
            0,
            target_depth,
            &path_query.path,
            &mut counts,
            &proof.grovedb_proof,
            path_query,
            mtd,
            platform_version,
        )?,
    }

    // 3. Tenderdash signature on root_hash — same as aggregate path.
    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(counts)
}
