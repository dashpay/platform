//! Verified average result + free-function proof verifiers for the
//! average surface.
//!
//! Average-side analog of [`super::document_sum::DocumentSum`].
//! Holds the `(count, sum)` pair recovered from a count-sum-bearing
//! tree proof (`CountSumTree` / `ProvableCountSumTree` /
//! `ProvableCountProvableSumTree`). `Aggregate` mode returns one
//! [`DocumentAverage`]; `Entries` mode returns
//! [`super::document_split_average::DocumentSplitAverages`] instead.
//!
//! Averages are NOT pre-divided server-side — the verifier surfaces
//! the raw `(count, sum)` and the caller divides. See the proto
//! file's `AverageResults` docstring for the rationale (precision +
//! client-chosen representation).
//!
//! The generic `FromProof<Q>` impl below intentionally rejects
//! calls (matching [`super::document_split_count::DocumentSplitCounts`]'s
//! pattern). Real dispatch lives in the
//! `FromProof<DocumentQuery>` impl in
//! `rs-sdk/src/platform/documents/document_average.rs`, which picks
//! among the free-function verifiers below based on the resolved
//! `DocumentAverageMode`.

use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error};
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::version::PlatformVersion;
use drive::query::drive_document_average_query::AverageEntry;
use drive::query::drive_document_sum_query::DriveDocumentSumQuery;

/// Verify a grovedb point-lookup proof against a count-sum-bearing
/// index terminator and return per-branch `(count, sum)` entries.
/// AVG analog of [`super::document_sum::verify_point_lookup_sum_proof`].
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_point_lookup_count_and_sum_proof`].
/// Used by the prove path's `Aggregate` + Equal/In + no range
/// shape when the chosen index declares BOTH `summable: "<prop>"`
/// AND a `countable` terminator.
pub fn verify_point_lookup_count_and_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<AverageEntry>, Error> {
    let (root_hash, entries) = query
        .verify_point_lookup_count_and_sum_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(entries)
}

/// Verify a per-distinct-key range-AVG proof against an index that
/// declares BOTH `rangeCountable: true` AND `rangeSummable: true`
/// (a `rangeAverageable: true` index) and return per-`(in_key,
/// key)` `(count, sum)` entries. AVG analog of
/// [`super::document_sum::verify_distinct_sum_proof`].
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_distinct_count_and_sum_proof`].
/// Used by the prove path's `GroupByRange` / `GroupByCompound` +
/// range shape on the AVG surface.
pub fn verify_distinct_count_and_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    limit: u16,
    left_to_right: bool,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<AverageEntry>, Error> {
    let (root_hash, entries) = query
        .verify_distinct_count_and_sum_proof(
            &proof.grovedb_proof,
            limit,
            left_to_right,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok(entries)
}

/// The `(count, sum)` pair across documents matching a query,
/// verified from proof. Client computes `avg = sum / count` using
/// whichever precision representation it wants.
///
/// `count` is `u64` (counts are non-negative); `sum` is `i64`
/// (matching `DocumentSum`). The grovedb primitive that backs this
/// is `AggregateCountAndSumOnRange` (PCPS-leaf) for range-filtered
/// queries, or the primary-key count-sum-bearing element direct
/// read for empty-where queries on a
/// `documentsCountable + documentsSummable` doctype.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentAverage {
    /// Total matched-document count for the query.
    pub count: u64,
    /// Total aggregated value of the `sum_property` for the query.
    pub sum: i64,
}

impl DocumentAverage {
    /// Convenience: compute the average as `f64`. Returns `None`
    /// when `count == 0` (preserving the divide-by-zero contract
    /// rather than producing `NaN` / `inf`). Callers that need a
    /// different representation should divide `self.sum /
    /// self.count` directly.
    pub fn as_f64(&self) -> Option<f64> {
        if self.count == 0 {
            None
        } else {
            Some(self.sum as f64 / self.count as f64)
        }
    }
}

// No generic `FromProof<Q>` impl is provided here — see the
// `DocumentSum` docstring for the rationale. Callers reach this
// through `FromProof<DocumentQuery> for DocumentAverage` in
// `rs-sdk/src/platform/documents/document_average.rs`.

/// Verify a leaf-PCPS `AggregateCountAndSumOnRange` proof and the
/// surrounding tenderdash commit, returning the verified
/// `(count, sum)` pair. Used by the prove path's
/// `select=AVG, group_by=[]` with a range clause on an index that
/// declares BOTH `rangeCountable: true` AND `rangeSummable: true`
/// (i.e. the terminator is a `ProvableCountProvableSumTree`).
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_aggregate_count_and_sum_proof`].
/// Both metrics come from one root-hash-committed traversal of the
/// PCPS terminator — no way for the server to splice a count from
/// one set with a sum from another.
pub fn verify_aggregate_count_and_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(u64, i64), Error> {
    let (root_hash, count, sum) = query
        .verify_aggregate_count_and_sum_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok((count, sum))
}

/// Verify a grovedb proof of the document type's primary-key
/// count-sum-bearing element (`CountSumTree` /
/// `ProvableCountSumTree` / `ProvableCountProvableSumTree`) and
/// return the unfiltered `(count, sum)` pair.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_primary_key_count_sum_tree_proof`].
/// Used by the prove path's AVG fast path on a doctype that has
/// both `documentsCountable: true` and `documentsSummable: "<prop>"`
/// set, with empty where clauses — the server proves the
/// primary-key element directly and the SDK extracts both metrics
/// from one verified element.
pub fn verify_primary_key_count_sum_tree_proof(
    contract_id: [u8; 32],
    document_type_name: &str,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(u64, i64), Error> {
    let (root_hash, count, sum) = DriveDocumentSumQuery::verify_primary_key_count_sum_tree_proof(
        &proof.grovedb_proof,
        contract_id,
        document_type_name,
        platform_version,
    )
    .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok((count, sum))
}

/// Verify a **carrier**-PCPS `AggregateCountAndSumOnRange` proof
/// and return the per-`In`-branch `(count, sum)` triples. AVG analog
/// of count's
/// [`super::document_count::verify_carrier_aggregate_count_proof`]
/// and sum's
/// [`super::document_sum::verify_carrier_aggregate_sum_proof`].
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentSumQuery::verify_carrier_aggregate_count_and_sum_proof`].
/// Used by the prove path when the request shape is `select=AVG,
/// group_by=[in_field], where = In(in_field) + range(other_field),
/// prove=true` against a PCPS-eligible index — drive routes it to
/// the carrier-PCPS executor.
///
/// Result: one [`AverageEntry`] per **present** In branch with
/// `in_key = <serialized In value>`, `key = []`, `count = Some(n)`,
/// `sum = Some(v)`. Absent In branches are omitted; the count and
/// sum axes never disagree on present/absent because the proof
/// commits both metrics from the same merk traversal.
pub fn verify_carrier_aggregate_count_and_sum_proof(
    query: &DriveDocumentSumQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    limit: Option<u16>,
    left_to_right: bool,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<Vec<AverageEntry>, Error> {
    let (root_hash, per_key_count_sum) = query
        .verify_carrier_aggregate_count_and_sum_proof(
            &proof.grovedb_proof,
            limit,
            left_to_right,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    let entries = per_key_count_sum
        .into_iter()
        .map(|(in_key, count, sum)| AverageEntry {
            in_key: Some(in_key),
            key: Vec::new(),
            count: Some(count),
            sum: Some(sum),
        })
        .collect();
    Ok(entries)
}
