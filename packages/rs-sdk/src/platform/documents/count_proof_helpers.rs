//! Shared helpers used by the [`DocumentCount`] and
//! [`DocumentSplitCounts`] proof verifiers.
//!
//! Both `FromProof` impls validate the same `select` field,
//! translate the same `u32`-with-`0`-sentinel limit into the
//! verifier's `u16`, and — for the aggregate shapes — dispatch
//! through the same per-shape proof verification logic. Keeping
//! those helpers in one place removes the cross-impl delegation
//! that lived here before: each `FromProof` impl now becomes a
//! thin wrapper that calls [`verify_aggregate_count`] and reshapes
//! the resulting `Option<u64>` into its own response type.
//!
//! [`DocumentCount`]: drive_proof_verifier::DocumentCount
//! [`DocumentSplitCounts`]: drive_proof_verifier::DocumentSplitCounts

use crate::platform::documents::document_query::DocumentQuery;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Select;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters},
};
use drive::query::DriveDocumentCountQuery;
use drive_proof_verifier::{
    verify_aggregate_count_proof, verify_distinct_count_proof, verify_point_lookup_count_proof,
    verify_primary_key_count_tree_proof,
};

/// Validate that the caller-built [`DocumentQuery`] actually
/// targets the count surface. Without this check a caller who
/// forgets `.with_select(Select::Count)` would silently send a
/// `Documents` request and then fail much later inside the
/// proof verifier with an inscrutable "wrong wire shape" error;
/// this surfaces the misuse at the SDK boundary with a clear
/// pointer to the fix.
pub(super) fn assert_select_is_count(
    request: &DocumentQuery,
) -> Result<(), drive_proof_verifier::Error> {
    if request.select != Select::Count {
        return Err(drive_proof_verifier::Error::RequestError {
            error: format!(
                "DocumentCount / DocumentSplitCounts require `select = Count`, got {:?}. \
                 Call `.with_select(Select::Count)` on the DocumentQuery before fetching.",
                request.select
            ),
        });
    }
    Ok(())
}

/// Translate the SDK's `u32`-with-`0`-sentinel limit into the
/// `u16` the proof verifier wants to rebuild the prover's path
/// query.
///
/// `0` falls back to [`drive::config::DEFAULT_QUERY_LIMIT`] — the
/// same compile-time constant the server's prove-distinct
/// dispatcher reads (NOT the operator-tunable
/// `drive_config.default_query_limit`, which the SDK can't see).
/// With both sides anchored to the shared constant the path-query
/// bytes match byte-for-byte across operators, so merk-root
/// recomputation succeeds regardless of any operator's tuning.
///
/// Non-zero values must fit in `u16` since the wire's
/// `optional uint32` is wider than the verifier's path-query
/// representation. We `try_from` rather than truncate so a caller
/// passing `limit > u16::MAX` fails loudly at the SDK boundary
/// rather than silently producing a mismatched path query.
pub(super) fn limit_to_u16_or_default(limit: u32) -> Result<u16, drive_proof_verifier::Error> {
    if limit == 0 {
        return Ok(drive::config::DEFAULT_QUERY_LIMIT);
    }
    u16::try_from(limit).map_err(|_| drive_proof_verifier::Error::RequestError {
        error: format!(
            "limit {} exceeds u16::MAX; the prove-distinct path query cannot represent it",
            limit
        ),
    })
}

/// Verify a count-shape proof and return the aggregate `u64` it
/// commits to, plus the response metadata and proof.
///
/// Both `DocumentCount` and `DocumentSplitCounts` (for the
/// aggregate `group_by = []` branch) need exactly this: a verified
/// total count, regardless of which proof primitive the server
/// emitted. The four sub-cases:
///
/// 1. **range + non-empty `group_by`** (`GroupByRange` /
///    `GroupByCompound`) — server emitted a `RangeDistinctProof`
///    (per-key `KVCount` ops); verify via
///    `verify_distinct_count_proof` and sum the per-key counts.
///    Path-query reconstruction uses
///    [`limit_to_u16_or_default`] anchored to
///    `DEFAULT_QUERY_LIMIT` so proof bytes are operator-tuning-
///    independent.
/// 2. **range + empty `group_by`** (`Aggregate`) — server emitted
///    a single `AggregateCountOnRange` proof; verify via
///    `verify_aggregate_count_proof`.
/// 3. **no range + empty where + `documents_countable`**
///    (`Aggregate`) — server proved the doctype's primary-key
///    CountTree element directly; verify via
///    `verify_primary_key_count_tree_proof`.
/// 4. **no range + covering `countable: true` index** — server
///    proved per-branch CountTree elements; verify via
///    `verify_point_lookup_count_proof` and sum the per-branch
///    counts (`filter_map` drops `None` entries the verifier may
///    emit for queried-but-absent branches — they don't
///    contribute to the verified total).
pub(super) fn verify_aggregate_count<'a>(
    request: DocumentQuery,
    response: GetDocumentsResponse,
    platform_version: &PlatformVersion,
    provider: &'a dyn ContextProvider,
) -> Result<(Option<u64>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
    // Range queries arrive with a grovedb `AggregateCountOnRange`
    // proof (when `group_by` is empty) or a `RangeDistinctProof`
    // (per-key `KVCount` ops, when `group_by` is non-empty).
    if request
        .where_clauses
        .iter()
        .any(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator))
    {
        let document_type = request
            .data_contract
            .document_type_for_name(&request.document_type_name)
            .map_err(|e| drive_proof_verifier::Error::RequestError {
                error: format!(
                    "document type {} not found in contract: {}",
                    request.document_type_name, e
                ),
            })?;
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &request.where_clauses,
        )
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "range count requires a `range_countable: true` index whose last \
                    property matches the range field"
                .to_string(),
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id: request.data_contract.id().to_buffer(),
            document_type_name: request.document_type_name.clone(),
            index,
            where_clauses: request.where_clauses.clone(),
        };
        let proof = response
            .proof()
            .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
        let mtd = response
            .metadata()
            .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;

        if !request.group_by.is_empty() {
            // RangeDistinctProof: per-key `KVCount` ops. Sum to
            // collapse to a single aggregate u64.
            let limit_u16 = limit_to_u16_or_default(request.limit)?;
            let left_to_right = request
                .order_by_clauses
                .first()
                .map(|c| c.ascending)
                .unwrap_or(true);
            let entries = verify_distinct_count_proof(
                &count_query,
                proof,
                mtd,
                limit_u16,
                left_to_right,
                platform_version,
                provider,
            )?;
            let total: u64 = entries.iter().filter_map(|e| e.count).sum();
            return Ok((Some(total), mtd.clone(), proof.clone()));
        }

        // AggregateCountOnRange: single u64 verified out.
        let count =
            verify_aggregate_count_proof(&count_query, proof, mtd, platform_version, provider)?;
        return Ok((Some(count), mtd.clone(), proof.clone()));
    }

    // No range: count-tree proof primitives.
    let document_type = request
        .data_contract
        .document_type_for_name(&request.document_type_name)
        .map_err(|e| drive_proof_verifier::Error::RequestError {
            error: format!(
                "document type {} not found in contract: {}",
                request.document_type_name, e
            ),
        })?;
    let proof = response
        .proof()
        .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
    let mtd = response
        .metadata()
        .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;

    // documents_countable fast path: empty where + the document
    // type opts into a primary-key CountTree.
    if request.where_clauses.is_empty() && document_type.documents_countable() {
        let contract_id = request.data_contract.id().to_buffer();
        let count = verify_primary_key_count_tree_proof(
            contract_id,
            &request.document_type_name,
            proof,
            mtd,
            platform_version,
            provider,
        )?;
        return Ok((Some(count), mtd.clone(), proof.clone()));
    }

    // PointLookupProof against a covering `countable: true` index.
    // Sum the per-branch verified counts; `filter_map` drops any
    // `None` entries the verifier emits for queried-but-absent
    // branches — those don't contribute to the verified total.
    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &request.where_clauses,
    )
    .ok_or_else(|| drive_proof_verifier::Error::RequestError {
        error: "prove count requires a `countable: true` index whose properties \
                exactly match the where clause fields, or `documentsCountable: \
                true` on the document type for unfiltered total counts"
            .to_string(),
    })?;
    let count_query = DriveDocumentCountQuery {
        document_type,
        contract_id: request.data_contract.id().to_buffer(),
        document_type_name: request.document_type_name.clone(),
        index,
        where_clauses: request.where_clauses.clone(),
    };
    let entries =
        verify_point_lookup_count_proof(&count_query, proof, mtd, platform_version, provider)?;
    let total: u64 = entries.iter().filter_map(|e| e.count).sum();
    Ok((Some(total), mtd.clone(), proof.clone()))
}
