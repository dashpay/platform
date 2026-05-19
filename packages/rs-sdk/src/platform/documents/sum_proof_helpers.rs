//! Shared SUM-proof dispatch used by [`DocumentSum`] and
//! [`DocumentSplitSums`].
//!
//! Mirror of [`super::count_proof_helpers::verify_count_query`] for
//! the sum surface. Both consumers reduce to "give me verified
//! `Vec<SumEntry>` for this `DocumentQuery`" — [`DocumentSum`] sums
//! the entries into a single `i64`, [`DocumentSplitSums`] passes
//! them through.
//!
//! [`DocumentSum`]: drive_proof_verifier::DocumentSum
//! [`DocumentSplitSums`]: drive_proof_verifier::DocumentSplitSums

use crate::platform::documents::document_query::DocumentQuery;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters},
};
use drive::query::drive_document_sum_query::index_picker::{
    find_range_summable_index_for_where_clauses, find_summable_index_for_where_clauses,
};
use drive::query::drive_document_sum_query::{is_range_operator, DriveDocumentSumQuery};
use drive::query::{SelectFunction, WhereOperator};
use drive_proof_verifier::{
    verify_aggregate_sum_proof, verify_carrier_aggregate_sum_proof, verify_point_lookup_sum_proof,
    verify_primary_key_sum_tree_proof, SumEntry,
};

/// Validate that the caller-built [`DocumentQuery`] targets the
/// sum surface. SUM always needs a non-empty `field` naming the
/// integer property to aggregate.
pub(super) fn assert_select_is_sum(
    request: &DocumentQuery,
) -> Result<(), drive_proof_verifier::Error> {
    if request.select.function != SelectFunction::Sum || request.select.field.is_empty() {
        return Err(drive_proof_verifier::Error::RequestError {
            error: format!(
                "DocumentSum / DocumentSplitSums require \
                 `SelectProjection::sum(\"<field>\")`; got {:?}. \
                 The named field must match the doctype-level \
                 `documentsSummable` OR a `summable: \"<field>\"` \
                 index covering the where-clause shape.",
                request.select
            ),
        });
    }
    Ok(())
}

/// Verify a SUM-shape proof and return per-branch `SumEntry`s.
///
/// Mirrors the server-side `detect_sum_mode` routing in
/// [`drive::query::drive_document_sum_query::drive_dispatcher`].
/// Supported prove shapes:
/// - **Empty where + matching `documentsSummable` doctype** →
///   primary-key SumTree direct read.
/// - **Range, no `In`, summable index** → aggregate-sum proof.
/// - **Equal/`In`, no range, summable index** → point-lookup
///   sum proof (one entry per resolved key).
/// - **`In` + range, `rangeSummable` index** → carrier-aggregate
///   sum proof (one aggregate per In branch).
///
/// Other shapes (range-distinct on a `rangeSummable` index) are
/// rejected as `RequestError` until the server-side
/// `distinct_sum_path_query` is unstubbed and a matching
/// verifier is added.
pub(super) fn verify_sum_query(
    request: DocumentQuery,
    response: GetDocumentsResponse,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(Option<Vec<SumEntry>>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
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

    // Empty-where SUM fast path: primary-key SumTree element direct
    // read. Mirror of the server-side fast path in
    // `execute_document_sum_point_lookup_proof`.
    if request.where_clauses.is_empty()
        && document_type
            .documents_summable()
            .map(|p| p == sum_property)
            .unwrap_or(false)
    {
        let sum = verify_primary_key_sum_tree_proof(
            contract_id,
            &request.document_type_name,
            proof,
            mtd,
            platform_version,
            provider,
        )?;
        return Ok((
            Some(single_empty_key_entry(sum)),
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

    // Range SUM: aggregate (no In) or carrier (In on prefix +
    // range on terminator). Both need a `rangeSummable: true`
    // index; carrier additionally needs the In to land on a
    // prefix property of that index.
    if has_range {
        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            &request.where_clauses,
            &sum_property,
        )
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "prove range SUM requires a `rangeSummable: true` index whose last \
                    property matches the range field and whose summable property \
                    matches the request's select `field`"
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

        if !has_in {
            let sum =
                verify_aggregate_sum_proof(&sum_query, proof, mtd, platform_version, provider)?;
            return Ok((
                Some(single_empty_key_entry(sum)),
                mtd.clone(),
                proof.clone(),
            ));
        } else {
            let limit_u16 = if request.limit == 0 {
                None
            } else {
                Some(u16::try_from(request.limit).map_err(|_| {
                    drive_proof_verifier::Error::RequestError {
                        error: format!(
                            "limit {} exceeds u16::MAX for carrier-aggregate SUM proof",
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
            let entries = verify_carrier_aggregate_sum_proof(
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

    // Point-lookup SUM (Equal-only or In on terminator).
    let index = find_summable_index_for_where_clauses(
        document_type.indexes(),
        &request.where_clauses,
        &sum_property,
    )
    .ok_or_else(|| drive_proof_verifier::Error::RequestError {
        error: "prove SUM requires a `summable: \"<prop>\"` index whose properties \
                exactly match the where clause fields and whose summed property \
                matches the request's select `field`, or `documentsSummable: \
                \"<prop>\"` on the document type for unfiltered total sums"
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
    let entries =
        verify_point_lookup_sum_proof(&sum_query, proof, mtd, platform_version, provider)?;
    Ok((Some(entries), mtd.clone(), proof.clone()))
}

/// Wrap a single `i64` from an aggregate primitive (range-aggregate
/// or primary-key direct read) as a one-element `Vec<SumEntry>` so
/// call sites see a uniform shape.
fn single_empty_key_entry(sum: i64) -> Vec<SumEntry> {
    vec![SumEntry {
        in_key: None,
        key: Vec::new(),
        sum: Some(sum),
    }]
}
