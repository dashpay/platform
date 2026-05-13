//! SDK-side count surface for the `getDocuments` endpoint.
//!
//! Callers build a [`DocumentQuery`] and opt into the count
//! surface via [`DocumentQuery::with_select`]`(Select::Count)`
//! plus an optional [`DocumentQuery::with_group_by`] for per-
//! group entries. The same [`DocumentQuery`] value drives three
//! different `Fetch` implementations depending on which response
//! type the caller asks for:
//!
//!   - [`Document`] / `Documents` (in `document_query.rs`) — when
//!     `select = Documents`.
//!   - [`DocumentCount`] (here) — when `select = Count, group_by = []`,
//!     or when collapsing per-group entries into a single
//!     aggregate.
//!   - [`DocumentSplitCounts`] (here) — when `select = Count,
//!     group_by = [<field>]`, or when the caller wants the
//!     aggregate-as-single-empty-key-entry shape.
//!
//! Dispatch reads `request.group_by` directly: `[]` routes to
//! the aggregate verifier path, `[field]` / `[field_a, field_b]`
//! to the distinct verifier path. There is no implicit grouping
//! anywhere — the FFI and wasm-sdk surfaces also expose
//! `group_by` directly, mirroring the wire shape one-to-one.
//!
//! [`Document`]: dpp::document::Document

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::Fetch;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Select;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters},
};
use drive::query::DriveDocumentCountQuery;
use drive_proof_verifier::{
    verify_aggregate_count_proof, verify_distinct_count_proof, verify_point_lookup_count_proof,
    verify_primary_key_count_tree_proof, DocumentCount, DocumentSplitCounts, FromProof,
    SplitCountEntry,
};

/// Validate that the caller-built [`DocumentQuery`] actually
/// targets the count surface. Without this check a caller who
/// forgets `.with_select(Select::Count)` would silently send a
/// `Documents` request and then fail much later inside the
/// proof verifier with an inscrutable "wrong wire shape" error;
/// this surfaces the misuse at the SDK boundary with a clear
/// pointer to the fix.
fn assert_select_is_count(request: &DocumentQuery) -> Result<(), drive_proof_verifier::Error> {
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
fn limit_to_u16_or_default(limit: u32) -> Result<u16, drive_proof_verifier::Error> {
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

impl FromProof<DocumentQuery> for DocumentCount {
    type Request = DocumentQuery;
    type Response = GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        assert_select_is_count(&request)?;

        // Range queries arrive with a grovedb `AggregateCountOnRange`
        // proof (produced by `Drive::execute_document_count_range_proof`)
        // or a `RangeDistinctProof` (per-key `KVCount` ops) depending
        // on whether the caller grouped by the range field. Both are
        // decoded against a `DriveDocumentCountQuery` built from the
        // SDK request — same builder both sides share, so the path
        // query bytes match byte-for-byte.
        if request
            .where_clauses
            .iter()
            .any(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator))
        {
            let response: Self::Response = response.into();

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

            // Distinct (non-empty `group_by`) vs aggregate (empty
            // `group_by`) selects which proof shape the server
            // emits. `(range, prove=true, group_by=[g])` routes to
            // `RangeDistinctProof` (emits per-key `KVCount` ops);
            // `(range, prove=true, group_by=[])` routes to
            // `RangeProof` (emits a single `AggregateCountOnRange`
            // aggregate). The two proof shapes are NOT
            // interchangeable — decoding a distinct proof with the
            // aggregate verifier fails merk-root recomputation
            // because the path queries differ
            // structurally.
            if !request.group_by.is_empty() {
                // Rebuild the same path query the prover signed. The
                // limit anchors to the compile-time `DEFAULT_QUERY_LIMIT`
                // constant (matching `drive_dispatcher.rs`'s
                // `RangeDistinctProof` arm) so proof bytes are
                // deterministic across operators. Direction comes from
                // the first `order_by` clause, defaulting to ascending.
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
                // `DocumentCount` collapses to a single aggregate u64.
                // Sum the verified per-key counts. The proof's
                // `KVCount` ops are merk-root-bound via
                // `node_hash_with_count`, so the sum is
                // cryptographically committed — same forge-resistance
                // as `AggregateCountOnRange`, just expressed as a
                // post-verification reduction in Rust.
                //
                // `flatten` drops `None` entries — distinct-walk
                // verifier never emits them today (every emitted
                // entry corresponds to a verified `KVCount` op),
                // but this keeps the aggregate honest if a future
                // synthesis step on this code path ever does.
                let total: u64 = entries.iter().filter_map(|e| e.count).sum();
                return Ok((Some(DocumentCount(total)), mtd.clone(), proof.clone()));
            }

            // Range + prove + empty group_by: aggregate proof path.
            // The verifier helper rebuilds the prover's path query
            // internally via `count_query.aggregate_count_path_query`
            // — same builder both sides share.
            let count =
                verify_aggregate_count_proof(&count_query, proof, mtd, platform_version, provider)?;
            return Ok((Some(DocumentCount(count)), mtd.clone(), proof.clone()));
        }

        // No range clause: route through the count-tree proof
        // primitives. Two sub-cases mirror the server-side dispatch:
        //
        // 1. **documents_countable + empty where**: the doctype's
        //    primary-key tree is itself a CountTree. Server proves
        //    that element directly; SDK verifies and extracts
        //    `count_value`. O(log n) proof, no index.
        // 2. **Else**: must have a `countable: true` index whose
        //    properties exactly match the where clauses. Server
        //    proves the per-branch CountTree elements; SDK sums their
        //    `count_value`s.
        let response: Self::Response = response.into();
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
            return Ok((Some(DocumentCount(count)), mtd.clone(), proof.clone()));
        }

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
        // For Equal-only fully-covered the verifier returns a single
        // entry (empty `key`) and the sum is just that entry's count;
        // for Equal-prefix + In-on-last it sums the per-In-value
        // counts. A branch with zero docs is omitted by the verifier
        // so missing entries contribute 0. `filter_map` drops `None`
        // entries that downstream synthesis might have added (e.g.
        // SDK marking absent In branches as `None` to avoid
        // conflating "no proof" with "zero").
        let total: u64 = entries.iter().filter_map(|e| e.count).sum();
        Ok((Some(DocumentCount(total)), mtd.clone(), proof.clone()))
    }
}

impl Fetch for DocumentCount {
    type Request = DocumentQuery;
}

/// Per-key counts view of the unified count endpoint.
///
/// Backed by the same [`DocumentQuery`] as [`DocumentCount`]; the
/// only difference is response shape — `DocumentSplitCounts`
/// returns the full `entries` list keyed by the splitting
/// property's serialized value, while `DocumentCount` returns the
/// sum.
///
/// Splitting is signalled by:
/// - An `In` where-clause on the request: the field of that clause
///   becomes the split property and each value in the array becomes
///   one entry in the result. On the **proof path**, the SDK
///   synthesizes a `count: None` entry for each In value the proof
///   was silent on (zero-count branches aren't materialized in the
///   merk tree, so absent-from-proof is cryptographically distinct
///   from `Some(0)`). The **no-proof path** emits `count: Some(0)`
///   for branches the executor confirmed are empty.
/// - A range where-clause plus `with_group_by(range_field)`: each
///   distinct value in the range becomes one entry. Zero-count
///   ranges are simply absent on both paths — the range itself is
///   unbounded so there's no caller-supplied "expected keys" list
///   to synthesize `None` entries against.
///
/// Without any grouping the response is a single entry with empty
/// `key` (i.e., the total count expressed as one-element entries
/// for shape uniformity).
impl FromProof<DocumentQuery> for DocumentSplitCounts {
    type Request = DocumentQuery;
    type Response = GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        assert_select_is_count(&request)?;

        // `has_in` controls the single-empty-key-entry guarantee on
        // the no-range prove path: Equal-only fully-covered queries
        // promise one entry with empty key (the verified count, even
        // if zero); In-on-last queries promise one entry per emitted
        // In value (zero-count branches are simply absent —
        // intentional v1 divergence from SQL; see proto docs).
        let has_in = request
            .where_clauses
            .iter()
            .any(|wc| wc.operator == drive::query::WhereOperator::In);

        let has_range = request
            .where_clauses
            .iter()
            .any(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator));

        // Range + non-empty group_by (with or without In on prefix):
        // per-distinct-value counts via a regular merk range proof
        // (no `AggregateCountOnRange` wrapper). The proof's
        // `KVCount` ops carry per-`(in_key, key)` counts that the
        // merk root commits to via `node_hash_with_count`, so
        // `verify_distinct_count_proof` runs the standard hash
        // chain check and reads the counts back as a verified
        // `Vec<SplitCountEntry>`. Only reachable when the SDK
        // builder set `.with_group_by(...)`.
        if has_range && !request.group_by.is_empty() {
            let response: Self::Response = response.into();

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
                error: "distinct range count requires a `range_countable: true` index whose \
                        last property matches the range field"
                    .to_string(),
            })?;

            let count_query = DriveDocumentCountQuery {
                document_type,
                contract_id: request.data_contract.id().to_buffer(),
                document_type_name: request.document_type_name.clone(),
                index,
                where_clauses: request.where_clauses.clone(),
            };
            // Match the prover's defaults for limit and order so
            // the verifier helper can rebuild the same path query
            // internally. Both sides anchor limit to
            // `drive::config::DEFAULT_QUERY_LIMIT` (the compile-time
            // constant) rather than the operator-tunable
            // `drive_config.default_query_limit`, so proof bytes
            // are deterministic across operators. Direction comes
            // from the first `order_by` clause; empty `order_by`
            // defaults to ascending — symmetric with the server's
            // `RangeDistinctProof` arm.
            let limit_u16 = limit_to_u16_or_default(request.limit)?;
            let left_to_right = request
                .order_by_clauses
                .first()
                .map(|c| c.ascending)
                .unwrap_or(true);

            let proof = response
                .proof()
                .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
            let mtd = response
                .metadata()
                .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;

            let entries = verify_distinct_count_proof(
                &count_query,
                proof,
                mtd,
                limit_u16,
                left_to_right,
                platform_version,
                provider,
            )?;
            return Ok((
                Some(DocumentSplitCounts::from_verified(entries)),
                mtd.clone(),
                proof.clone(),
            ));
        }

        // No range clause + `prove = true`: route through the count-
        // tree proof primitives, mirroring `DocumentCount`'s dispatch.
        // Two sub-cases:
        //
        // 1. **documents_countable + empty where**: prove the
        //    doctype's primary-key CountTree directly. Result is a
        //    single empty-key entry with the verified count.
        // 2. **Else**: require a covering countable index. Server
        //    proves the per-branch CountTree elements; SDK returns
        //    them as `Vec<SplitCountEntry>`. For Equal-only fully-
        //    covered the verifier returns one empty-key entry
        //    (re-emitted as `Some(0)` if absent — the proof
        //    committed to the empty tree). For Equal-prefix +
        //    In-on-last it returns one entry per existing In
        //    branch, then `synthesize_missing_in_entries` appends a
        //    `count: None` entry for each In value in the request
        //    that the proof was silent on.
        let response: Self::Response = response.into();
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

        // documents_countable fast path → single empty-key entry.
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
            let entries = vec![SplitCountEntry {
                in_key: None,
                key: Vec::new(),
                // `documents_countable` fast-path: the proof
                // verified the primary-key CountTree element
                // directly. The returned count IS the verified
                // value (possibly 0 for an empty doctype), so
                // emit `Some(_)` rather than `None`.
                count: Some(count),
            }];
            return Ok((
                Some(DocumentSplitCounts::from_verified(entries)),
                mtd.clone(),
                proof.clone(),
            ));
        }

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

        let mut entries =
            verify_point_lookup_count_proof(&count_query, proof, mtd, platform_version, provider)?;
        // Total-count case (Equal-only fully-covered): the proof
        // either covers a single CountTree element (entry present
        // with `Some(N)`) or doesn't materialize any element
        // because the doctype is empty. The second case still
        // counts as "verified zero" — the proof committed to the
        // empty tree — so emit `Some(0)`, not `None`. `None` is
        // reserved for "caller asked but verifier was silent on
        // this key," which is a different (and currently unused
        // on this path) signal.
        if !has_in && entries.is_empty() {
            entries.push(SplitCountEntry {
                in_key: None,
                key: Vec::new(),
                count: Some(0),
            });
        }

        // In-on-last + prove path: the proof only materializes
        // existing CountTree elements (zero-count branches aren't
        // stored in the merk tree). The caller's request lists
        // every In value they asked about; synthesize a `None`
        // entry for each In value the proof was silent on so
        // callers can tell "verified zero" (which the
        // PointLookupProof shape can't produce on its own) apart
        // from "absent from proof, unverified."
        if has_in {
            entries = synthesize_missing_in_entries(&request, entries, platform_version);
        }

        Ok((
            Some(DocumentSplitCounts::from_verified(entries)),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

impl Fetch for DocumentSplitCounts {
    type Request = DocumentQuery;
}

/// On the In-on-last + prove path, append a `count: None` entry
/// for every In value in the request that the verifier was silent
/// on. The PointLookupProof shape only materializes existing
/// CountTree elements (zero-count branches aren't stored in the
/// merk tree), so absent-from-proof on this path means either
/// "verified zero" or "proof was truncated" — distinct from
/// `Some(0)` which would mean a cryptographically committed zero.
/// Synthesizing `None` keeps callers from conflating those two.
///
/// Key serialization mirrors the prover's
/// `point_lookup_count_path_query` (which serializes each In
/// value via `document_type.serialize_value_for_key`), so the
/// synthesized keys byte-match the keys the verified entries
/// carry.
fn synthesize_missing_in_entries(
    request: &DocumentQuery,
    mut entries: Vec<SplitCountEntry>,
    platform_version: &PlatformVersion,
) -> Vec<SplitCountEntry> {
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use std::collections::HashSet;

    let Some(in_clause) = request
        .where_clauses
        .iter()
        .find(|wc| wc.operator == drive::query::WhereOperator::In)
    else {
        return entries;
    };
    let dpp::platform_value::Value::Array(in_values) = &in_clause.value else {
        return entries;
    };
    let Ok(document_type) = request
        .data_contract
        .document_type_for_name(&request.document_type_name)
    else {
        return entries;
    };

    // Serialize each requested In value to the same byte form the
    // prover used as the merk path key. `filter_map` silently drops
    // values that fail to serialize — those wouldn't have made it
    // to the merk path query either, so they'd not appear in the
    // proof regardless.
    let expected: HashSet<Vec<u8>> = in_values
        .iter()
        .filter_map(|v| {
            document_type
                .serialize_value_for_key(&in_clause.field, v, platform_version)
                .ok()
        })
        .collect();
    let present: HashSet<Vec<u8>> = entries.iter().map(|e| e.key.clone()).collect();

    for key in expected {
        if !present.contains(&key) {
            entries.push(SplitCountEntry {
                in_key: None,
                key,
                count: None,
            });
        }
    }
    entries
}
