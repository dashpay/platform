//! Shared AVG-proof dispatch used by [`DocumentAverage`] and
//! [`DocumentSplitAverages`].
//!
//! Mirror of [`super::count_proof_helpers::verify_count_query`] for
//! the average surface. Both consumers reduce to "give me verified
//! `Vec<AverageEntry>` for this `DocumentQuery`" —
//! [`DocumentAverage`] sums into a single `(count, sum)` pair,
//! [`DocumentSplitAverages`] passes the entries through.
//!
//! [`DocumentAverage`]: drive_proof_verifier::DocumentAverage
//! [`DocumentSplitAverages`]: drive_proof_verifier::DocumentSplitAverages

use crate::platform::documents::document_query::DocumentQuery;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters},
};
use drive::query::drive_document_sum_query::index_picker::find_range_summable_index_for_where_clauses;
use drive::query::drive_document_sum_query::index_picker::find_summable_index_for_where_clauses;
use drive::query::drive_document_sum_query::{is_range_operator, DriveDocumentSumQuery};
use drive::query::{SelectFunction, WhereOperator};
use drive_proof_verifier::{
    verify_aggregate_count_and_sum_proof, verify_carrier_aggregate_count_and_sum_proof,
    verify_distinct_count_and_sum_proof, verify_point_lookup_count_and_sum_proof,
    verify_primary_key_count_sum_tree_proof, AverageEntry,
};

/// Validate that the caller-built [`DocumentQuery`] targets the
/// average surface. AVG always needs a non-empty `field` naming
/// the integer property to average — `AVG()` with no field is a
/// wire-shape error (the server-side `not_yet_implemented` gate
/// rejects it too, so this is the SDK-side mirror).
pub(super) fn assert_select_is_avg(
    request: &DocumentQuery,
) -> Result<(), drive_proof_verifier::Error> {
    if request.select.function != SelectFunction::Avg || request.select.field.is_empty() {
        return Err(drive_proof_verifier::Error::RequestError {
            error: format!(
                "DocumentAverage / DocumentSplitAverages require \
                 `SelectProjection::avg(\"<field>\")`; got {:?}. \
                 The named field must match the doctype-level \
                 `documentsSummable` (or `documentsAverageable`) \
                 OR a `summable: \"<field>\"` index covering the \
                 where-clause shape — averages reuse sum-tree \
                 indexes, no separate `averageable` flag is needed.",
                request.select
            ),
        });
    }
    Ok(())
}

/// Verify an AVG-shape proof and return per-branch `AverageEntry`s.
///
/// Single source of truth for the AVG-proof dispatch. Mirrors the
/// server-side prove path in
/// [`drive::Drive::execute_document_average_request`].
///
/// Supported prove shapes:
/// - **Empty where + `documentsCountable` + matching
///   `documentsSummable`** → primary-key count-sum tree direct
///   read via
///   [`verify_primary_key_count_sum_tree_proof`]. One entry,
///   `key = []`, `count = Some(n)`, `sum = Some(v)`.
/// - **Range + `rangeAverageable` index (rangeCountable +
///   rangeSummable)** → PCPS aggregate-count-and-sum proof via
///   [`verify_aggregate_count_and_sum_proof`]. One entry with
///   `key = []`, both metrics committed from the same in-range
///   traversal.
/// - **`In` + range + `rangeAverageable` index** (group_by =
///   `[in_field]`) → carrier-PCPS proof via
///   [`verify_carrier_aggregate_count_and_sum_proof`]. One entry
///   per present In branch, `in_key = <serialized In value>`.
/// - **`GroupByRange` / `GroupByCompound` + range +
///   `rangeAverageable` index** → per-distinct-key AVG proof via
///   [`verify_distinct_count_and_sum_proof`]. One entry per
///   distinct in-range value (or per `(in_key, key)` for compound).
/// - **Equal/`In` + no range on a summable + countable index** →
///   point-lookup count-and-sum proof via
///   [`verify_point_lookup_count_and_sum_proof`].
pub(super) fn verify_average_query(
    request: DocumentQuery,
    response: GetDocumentsResponse,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(Option<Vec<AverageEntry>>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
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
    let contract_id = request.data_contract.id().to_buffer();
    let sum_property = request.select.field.clone();

    // Empty-where AVG fast path: primary-key count-sum-bearing
    // element direct read. Doctype must declare both
    // `documentsCountable` and a matching `documentsSummable` —
    // mirror of the server-side fast-path gate in
    // `execute_document_average_prove`.
    if request.where_clauses.is_empty()
        && document_type.documents_countable()
        && document_type
            .documents_summable()
            .map(|p| p == sum_property)
            .unwrap_or(false)
    {
        let (count, sum) = verify_primary_key_count_sum_tree_proof(
            contract_id,
            &request.document_type_name,
            proof,
            mtd,
            platform_version,
            provider,
        )?;
        return Ok((
            Some(single_empty_key_entry(count, sum)),
            mtd.clone(),
            proof.clone(),
        ));
    }

    let has_range = request
        .where_clauses
        .iter()
        .any(|wc| is_range_operator(wc.operator));
    let has_in = request
        .where_clauses
        .iter()
        .any(|wc| wc.operator == WhereOperator::In);

    // Range AVG paths — three modes:
    // - Aggregate (group_by=[]) → single (count, sum) per proof
    // - GroupByIn (group_by=[in_field]) → carrier per-In (count, sum)
    // - GroupByRange / GroupByCompound (group_by=[range_field] or
    //   [in_field, range_field]) → per-distinct-key (count, sum)
    // All three need a `rangeAverageable`-eligible index.
    if has_range {
        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            &request.where_clauses,
            &sum_property,
        )
        .filter(|idx| idx.range_countable)
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "prove AVG requires an index that declares BOTH `rangeCountable: \
                    true` AND `rangeSummable: true` (a `rangeAverageable: true` \
                    index is the shorthand) whose last property matches the range \
                    field and whose summable property matches the request's \
                    select `field`"
                .to_string(),
        })?;
        let sum_query = DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name: request.document_type_name.clone(),
            index,
            where_clauses: request.where_clauses.clone(),
            sum_property,
        };

        // Distinct mode discrimination: SQL `group_by` has a
        // range-field first (with or without an In on prefix
        // before it). Mirrors the SUM dispatcher's logic.
        let group_by_first = request.group_by.first().map(String::as_str);
        let distinct_mode = match group_by_first {
            Some(field) => !is_in_field(&request.where_clauses, field),
            None => false,
        };

        if distinct_mode {
            let limit_u16 = if request.limit == 0 {
                drive::config::DEFAULT_QUERY_LIMIT
            } else {
                u16::try_from(request.limit).map_err(|_| {
                    drive_proof_verifier::Error::RequestError {
                        error: format!(
                            "limit {} exceeds u16::MAX for distinct AVG proof",
                            request.limit
                        ),
                    }
                })?
            };
            let left_to_right = request
                .order_by_clauses
                .first()
                .map(|c| c.ascending)
                .unwrap_or(true);
            let entries = verify_distinct_count_and_sum_proof(
                &sum_query,
                proof,
                mtd,
                limit_u16,
                left_to_right,
                platform_version,
                provider,
            )?;
            return Ok((Some(entries), mtd.clone(), proof.clone()));
        }

        if !has_in {
            // Aggregate (group_by=[]) — single (count, sum) per
            // proof; one entry with empty `key`.
            let (count, sum) = verify_aggregate_count_and_sum_proof(
                &sum_query,
                proof,
                mtd,
                platform_version,
                provider,
            )?;
            return Ok((
                Some(single_empty_key_entry(count, sum)),
                mtd.clone(),
                proof.clone(),
            ));
        } else {
            // Carrier-PCPS — one (count, sum) per resolved In
            // branch.
            let limit_u16 = if request.limit == 0 {
                None
            } else {
                Some(u16::try_from(request.limit).map_err(|_| {
                    drive_proof_verifier::Error::RequestError {
                        error: format!(
                            "limit {} exceeds u16::MAX for carrier-aggregate AVG proof",
                            request.limit
                        ),
                    }
                })?)
            };
            let left_to_right = request
                .order_by_clauses
                .first()
                .map(|c| c.ascending)
                .unwrap_or(true);
            let entries = verify_carrier_aggregate_count_and_sum_proof(
                &sum_query,
                proof,
                mtd,
                limit_u16,
                left_to_right,
                platform_version,
                provider,
            )?;
            return Ok((Some(entries), mtd.clone(), proof.clone()));
        }
    }

    // Point-lookup AVG: `Aggregate` + Equal/In + no range against
    // a count+sum index (summable + countable terminator). The
    // empty-where primary-key fast path is already handled above.
    if matches!(request.select.function, SelectFunction::Avg)
        && request.group_by.is_empty()
        && !has_range
    {
        let index = find_summable_index_for_where_clauses(
            document_type.indexes(),
            &request.where_clauses,
            &sum_property,
        )
        .filter(|idx| idx.countable.is_countable())
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "prove point-lookup AVG requires an index that declares BOTH \
                    `summable: \"<prop>\"` AND a countable terminator (`countable: \
                    \"countable\"` or `\"countableAllowingOffset\"`) whose properties \
                    exactly match the where clause fields"
                .to_string(),
        })?;
        let sum_query = DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name: request.document_type_name.clone(),
            index,
            where_clauses: request.where_clauses.clone(),
            sum_property,
        };
        let entries = verify_point_lookup_count_and_sum_proof(
            &sum_query,
            proof,
            mtd,
            platform_version,
            provider,
        )?;
        return Ok((Some(entries), mtd.clone(), proof.clone()));
    }

    Err(drive_proof_verifier::Error::RequestError {
        error: format!(
            "prove AVG: the (has_range = {}, has_in = {}, where_clauses.len() = {}) \
             combination is not yet supported on the prove path. Currently supported \
             shapes: empty-where on `documentsCountable + documentsSummable` doctype \
             (primary-key direct read); range AVG on a `rangeAverageable` index \
             (PCPS aggregate); In + range AVG on a `rangeAverageable` index (PCPS \
             carrier). Use prove=false on the wire to get the composed count + sum \
             path which covers every where-shape today.",
            has_range,
            has_in,
            request.where_clauses.len(),
        ),
    })
}

/// Wrap a single `(count, sum)` from a per-key-less aggregate
/// primitive (primary-key fast path / PCPS aggregate range) as a
/// one-element `Vec<AverageEntry>` so call sites see a uniform
/// shape across aggregate and carrier variants.
fn single_empty_key_entry(count: u64, sum: i64) -> Vec<AverageEntry> {
    vec![AverageEntry {
        in_key: None,
        key: Vec::new(),
        count: Some(count),
        sum: Some(sum),
    }]
}

/// Whether the request's where clauses contain an `In` clause on
/// the named field. Used to discriminate distinct vs carrier modes
/// when the SQL-shape `group_by` is non-empty — distinct mode is
/// "group_by on a range field" (no In on that field).
fn is_in_field(where_clauses: &[drive::query::WhereClause], field: &str) -> bool {
    where_clauses
        .iter()
        .any(|wc| wc.operator == WhereOperator::In && wc.field == field)
}
