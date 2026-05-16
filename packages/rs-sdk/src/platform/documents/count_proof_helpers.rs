//! Shared count-proof dispatch used by [`DocumentCount`] and
//! [`DocumentSplitCounts`].
//!
//! Both consumers reduce to "give me a verified
//! `Vec<SplitCountEntry>` for this `DocumentQuery`" —
//! [`DocumentCount`] sums the entries into a single `u64`,
//! [`DocumentSplitCounts`] passes them through. Putting the
//! four-way proof dispatch behind one helper means the per-shape
//! routing (which proof primitive to use, which index to pick,
//! how to wrap the result) lives in exactly one place; the
//! consumers become thin wrappers.
//!
//! [`DocumentCount`]: drive_proof_verifier::DocumentCount
//! [`DocumentSplitCounts`]: drive_proof_verifier::DocumentSplitCounts

use crate::platform::documents::document_query::DocumentQuery;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters},
};
use drive::query::{DriveDocumentCountQuery, SelectFunction};
use drive_proof_verifier::{
    verify_aggregate_count_proof, verify_distinct_count_proof, verify_point_lookup_count_proof,
    verify_primary_key_count_tree_proof, SplitCountEntry,
};

/// Validate that the caller-built [`DocumentQuery`] actually
/// targets the count surface AND uses the `COUNT(*)` shape — the
/// only shape today's verifier can reproduce. The verifier in
/// `verify_count_query()` rebuilds a `DriveDocumentCountQuery`
/// without threading the selected `field`, so an accepted
/// `COUNT(field)` request would verify as `COUNT(*)` (different
/// result for nullable fields). Reject `COUNT(field)` upstream
/// until the verifier carries the counted field; the
/// not-yet-implemented gate already rejects it server-side, so
/// this check is the SDK-side mirror.
pub(super) fn assert_select_is_count(
    request: &DocumentQuery,
) -> Result<(), drive_proof_verifier::Error> {
    if request.select.function != SelectFunction::Count || !request.select.field.is_empty() {
        return Err(drive_proof_verifier::Error::RequestError {
            error: format!(
                "DocumentCount / DocumentSplitCounts currently require \
                 `SelectProjection::count_star()` (i.e. `COUNT(*)`); got {:?}. \
                 `COUNT(field)` is not verifiable today because the proof \
                 query doesn't carry the counted field — `COUNT(field)` \
                 against a nullable field would verify as `COUNT(*)` and \
                 return a different total. Call \
                 `.with_select(SelectProjection::count_star())` on the \
                 DocumentQuery before fetching.",
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

/// Verify a count-shape proof and return per-branch entries.
///
/// Single source of truth for the four-way count-proof dispatch:
///
/// 1. **range + non-empty `group_by`** → `RangeDistinctProof`.
///    Emits one entry per distinct value via
///    `verify_distinct_count_proof`. Path-query reconstruction
///    uses [`limit_to_u16_or_default`] anchored to the shared
///    `DEFAULT_QUERY_LIMIT` so proof bytes are deterministic
///    across operators.
/// 2. **range + empty `group_by`** → `AggregateCountOnRange`.
///    Primitive emits a single u64; wrapped here as a single
///    empty-key entry so callers see a uniform `Vec<...>` shape.
/// 3. **no range + empty `where` + `documents_countable`** →
///    primary-key CountTree fast path. `verify_primary_key_count_tree_proof`
///    returns a `u64`; wrapped here as a single empty-key entry.
/// 4. **no range + covering `countable: true` index** →
///    `PointLookupProof`. `verify_point_lookup_count_proof`
///    emits one entry per **present** queried branch. Absent
///    In values are omitted from the returned list (the current
///    path query doesn't request absence proofs); callers that
///    need to surface "queried but absent" diff their request's
///    In array against the returned entries by key. See
///    `verify_point_lookup_count_proof_v0`'s docstring for the
///    forward-compat path to per-branch `count: None`.
///
/// Wrapping (2) and (3) as single empty-key entries is the only
/// shape massage this helper does — the underlying primitives
/// genuinely emit `u64`s, and consumers ([`DocumentCount`] sums,
/// [`DocumentSplitCounts`] passes through) want a uniform
/// per-entry vec regardless.
///
/// [`DocumentCount`]: drive_proof_verifier::DocumentCount
/// [`DocumentSplitCounts`]: drive_proof_verifier::DocumentSplitCounts
pub(super) fn verify_count_query(
    request: DocumentQuery,
    response: GetDocumentsResponse,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(Option<Vec<SplitCountEntry>>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
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

    let has_range = request
        .where_clauses
        .iter()
        .any(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator));

    if has_range {
        // Range path: either RangeDistinctProof (entries) or
        // AggregateCountOnRange (single u64 wrapped as one entry).
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

        if !request.group_by.is_empty() {
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
            return Ok((Some(entries), mtd.clone(), proof.clone()));
        }

        let count =
            verify_aggregate_count_proof(&count_query, proof, mtd, platform_version, provider)?;
        return Ok((
            Some(single_empty_key_entry(count)),
            mtd.clone(),
            proof.clone(),
        ));
    }

    // No range: documents_countable fast path or covering
    // countable index.
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
        return Ok((
            Some(single_empty_key_entry(count)),
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
    let entries =
        verify_point_lookup_count_proof(&count_query, proof, mtd, platform_version, provider)?;
    Ok((Some(entries), mtd.clone(), proof.clone()))
}

/// Wrap a single `u64` from an aggregate proof primitive
/// (`AggregateCountOnRange` or `verify_primary_key_count_tree_proof`)
/// as a one-element `Vec<SplitCountEntry>` so callers see a
/// uniform shape regardless of which primitive verified the
/// proof.
fn single_empty_key_entry(count: u64) -> Vec<SplitCountEntry> {
    vec![SplitCountEntry {
        in_key: None,
        key: Vec::new(),
        count: Some(count),
    }]
}
