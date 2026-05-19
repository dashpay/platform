//! Verified sum result + free-function proof verifiers for the
//! sum-tree surface.
//!
//! Sum-side analog of [`super::document_count`]. Holds the
//! aggregated `i64` recovered from a sum-tree proof — `Aggregate`
//! mode returns one [`DocumentSum`]; `Entries` mode returns
//! [`super::document_split_sum::DocumentSplitSums`] instead.
//!
//! The generic `FromProof<Q>` impl below intentionally rejects
//! calls (matching [`super::document_split_count::DocumentSplitCounts`]'s
//! pattern): the underlying proof primitive depends on the per-mode
//! routing of `(group_by, where_clauses, prove)`, which the generic
//! `Q` constraint can't carry. The real dispatch lives in the
//! `FromProof<DocumentQuery>` impl in
//! `rs-sdk/src/platform/documents/document_sum.rs`, which picks
//! among the free-function verifiers below based on the resolved
//! `DocumentSumMode`.

use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error};
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::version::PlatformVersion;
use drive::query::drive_document_sum_query::{DriveDocumentSumQuery, SumEntry};

/// The aggregated sum of an integer property across documents matching
/// a query, verified from proof.
///
/// Signed because grovedb's `SumTree` value type is `i64` — sums can
/// in principle be negative (typically signaling i64 overflow into
/// negative space, which the verifier surfaces explicitly). For
/// tip-jar-style non-negative aggregations this stays ≥ 0.
///
/// No generic `FromProof<Q>` impl is provided here — the real
/// proof dispatch needs the `DocumentQuery` request shape to pick
/// the right per-mode verifier (range-aggregate / point-lookup /
/// primary-key SumTree / In-carrier). Callers reach this through
/// `FromProof<DocumentQuery> for DocumentSum` in
/// `rs-sdk/src/platform/documents/document_sum.rs`, which routes to
/// the per-shape verifier free functions below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSum(pub i64);

/// Verify a grovedb `AggregateSumOnRange` proof and the surrounding
/// tenderdash commit, returning the verified `i64` sum from one
/// range traversal. Used by the prove path's
/// `select=SUM, group_by=[]` with a range clause on a
/// `rangeSummable: true` index.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_aggregate_sum_proof`] in rs-drive
/// (which does the merk-level verification via
/// `GroveDb::verify_aggregate_sum_query`). Both helpers share the
/// prover's `aggregate_sum_path_query` so the path query bytes
/// match byte-for-byte across the network.
pub fn verify_aggregate_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<i64, Error> {
    let (root_hash, sum) = query
        .verify_aggregate_sum_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(sum)
}

/// Verify a grovedb proof of the document type's primary-key
/// `SumTree` element and return the unfiltered total sum.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_primary_key_sum_tree_proof`].
/// Used by the prove path's `documents_summable: "<prop>"` fast
/// path — when the where clauses are empty and the document type
/// has a matching `documents_summable`, the server proves the
/// primary-key SumTree element directly and the SDK extracts the
/// sum from the verified element.
pub fn verify_primary_key_sum_tree_proof(
    contract_id: [u8; 32],
    document_type_name: &str,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<i64, Error> {
    let (root_hash, sum) = DriveDocumentSumQuery::verify_primary_key_sum_tree_proof(
        &proof.grovedb_proof,
        contract_id,
        document_type_name,
        platform_version,
    )
    .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(sum)
}

/// Verify a grovedb point-lookup sum proof and return the per-branch
/// entries. Sum analog of count's
/// [`super::document_count::verify_point_lookup_count_proof`].
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_point_lookup_sum_proof`]. Used
/// by the prove path's Equal/`In` sum queries against a
/// `summable: "<prop>"` index — one entry per **present** queried
/// key (absent keys are silently omitted because today's path
/// query doesn't request absence proofs, matching count's
/// behavior).
pub fn verify_point_lookup_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<SumEntry>, Error> {
    let (root_hash, entries) = query
        .verify_point_lookup_sum_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(entries)
}

/// Verify a per-distinct-key range-sum proof against a
/// `rangeSummable: true` index and return the per-`(in_key, key)`
/// sums.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_distinct_sum_proof`]. Sum analog
/// of count's
/// [`super::document_count::verify_distinct_count_proof`]. Used by
/// the prove path's `RangeDistinctProof` mode (GroupByRange /
/// GroupByCompound + range + prove against a `rangeSummable: true`
/// index).
pub fn verify_distinct_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    limit: u16,
    left_to_right: bool,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<SumEntry>, Error> {
    let (root_hash, entries) = query
        .verify_distinct_sum_proof(&proof.grovedb_proof, limit, left_to_right, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(entries)
}

/// Verify a **carrier** `AggregateSumOnRange` proof against a
/// `rangeSummable: true` index and return the per-`In`-branch sums.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_carrier_aggregate_sum_proof`].
/// Sum analog of count's
/// [`super::document_count::verify_carrier_aggregate_count_proof`].
/// Used by the prove path when the request shape is
/// `select=SUM, group_by=[in_field], where = In(in_field) +
/// range(other_field), prove=true` — drive's `detect_sum_mode`
/// routes that shape to
/// `DocumentSumMode::RangeAggregateCarrierProof`, which collapses
/// each In branch's range into a single committed `i64`.
///
/// Result: one [`SumEntry`] per **present** In branch with
/// `in_key = <serialized In value>`, `key = []`, `sum = Some(n)`.
/// Absent In branches are omitted; callers that need to surface
/// "queried but absent" diff their In array against the returned
/// `in_key`s.
pub fn verify_carrier_aggregate_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    limit: Option<u16>,
    left_to_right: bool,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<SumEntry>, Error> {
    let (root_hash, per_key_sums) = query
        .verify_carrier_aggregate_sum_proof(
            &proof.grovedb_proof,
            limit,
            left_to_right,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    // Map drive's `Vec<(Vec<u8>, i64)>` carrier shape onto the
    // SDK's `Vec<SumEntry>` so the call sites stay uniform.
    let entries = per_key_sums
        .into_iter()
        .map(|(in_key, sum)| SumEntry {
            in_key: Some(in_key),
            key: Vec::new(),
            sum: Some(sum),
        })
        .collect();
    Ok(entries)
}
