//! Sum-query dispatcher entry point.
//!
//! Parallels [`crate::query::drive_document_count_query::drive_dispatcher`]
//! for the sum surface. Routes a parsed [`DocumentSumRequest`] to one of
//! the per-mode executors based on the (where × mode × prove) triple,
//! exactly the way count's dispatcher does.
//!
//! `where_clauses_from_value` / `order_clauses_from_value` are wire-shape
//! adapters that the bench and the gRPC handler both use to convert the
//! CBOR-decoded `Value::Array` input into structured `Vec<WhereClause>` /
//! `Vec<OrderClause>`. Identical input contract to count.

use crate::drive::Drive;
use crate::error::Error;
use crate::query::drive_document_sum_query::{
    DocumentSumMode, DocumentSumRequest, DocumentSumResponse, RangeSumOptions, SumMode,
};
use crate::query::{OrderClause, WhereClause};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

#[cfg(feature = "server")]
impl Drive {
    /// Server-side entry point for the sum surface. Routes a
    /// [`DocumentSumRequest`] to the appropriate executor based on the
    /// where-shape, requested mode, and `prove` flag.
    ///
    /// Mirrors [`Drive::execute_document_count_request`].
    pub fn execute_document_sum_request(
        &self,
        request: DocumentSumRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentSumResponse, Error> {
        let resolved_mode = super::mode_detection::detect_sum_mode(&request, platform_version)?;

        let contract_id = request.contract.id().to_buffer();
        let document_type_name = request.document_type.name().to_string();
        let where_clauses = request.where_clauses;
        let sum_property = request.sum_property;
        // Default direction is ascending; the first order clause's
        // direction (if any) wins. Mirrors count's analog.
        let order_by_ascending = request
            .order_clauses
            .first()
            .map(|c| c.ascending)
            .unwrap_or(true);

        match resolved_mode {
            DocumentSumMode::Total => {
                let entries = self.execute_document_sum_total_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    sum_property,
                    transaction,
                    platform_version,
                )?;
                let total = entries.first().and_then(|e| e.sum).unwrap_or(0);
                Ok(DocumentSumResponse::Aggregate(total))
            }
            DocumentSumMode::PerInValue => {
                let options = RangeSumOptions {
                    return_distinct_sums_in_range: false,
                    carrier_outer_limit: None,
                    left_to_right: order_by_ascending,
                };
                Ok(DocumentSumResponse::Entries(
                    self.execute_document_sum_per_in_value_no_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        sum_property,
                        options,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
            DocumentSumMode::RangeNoProof => {
                let return_distinct = matches!(
                    request.mode,
                    SumMode::GroupByRange | SumMode::GroupByCompound
                );
                let options = RangeSumOptions {
                    return_distinct_sums_in_range: return_distinct,
                    carrier_outer_limit: None,
                    left_to_right: order_by_ascending,
                };
                let entries = self.execute_document_sum_range_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    sum_property,
                    options,
                    transaction,
                    platform_version,
                )?;
                if matches!(request.mode, SumMode::Aggregate) {
                    let total = entries.first().and_then(|e| e.sum).unwrap_or(0);
                    Ok(DocumentSumResponse::Aggregate(total))
                } else {
                    Ok(DocumentSumResponse::Entries(entries))
                }
            }
            DocumentSumMode::RangeProof => Ok(DocumentSumResponse::Proof(
                self.execute_document_sum_range_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    sum_property,
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentSumMode::RangeDistinctProof => {
                // Clamp the caller-supplied limit against the drive config's
                // max before narrowing to u16. The previous `as u16` cast
                // silently truncated values >= 65 536 and bypassed the
                // max-limit policy from `drive_config`.
                let effective_limit = request
                    .limit
                    .unwrap_or(request.drive_config.default_query_limit as u32)
                    .min(request.drive_config.max_query_limit as u32);
                let limit_u16 = u16::try_from(effective_limit).map_err(|_| {
                    Error::Query(crate::error::query::QuerySyntaxError::Unsupported(format!(
                        "limit {} exceeds u16::MAX for range-distinct sum proof",
                        effective_limit
                    )))
                })?;
                Ok(DocumentSumResponse::Proof(
                    self.execute_document_sum_range_distinct_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        sum_property,
                        limit_u16,
                        order_by_ascending,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
            DocumentSumMode::PointLookupProof => Ok(DocumentSumResponse::Proof(
                self.execute_document_sum_point_lookup_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    sum_property,
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentSumMode::RangeAggregateCarrierProof => {
                // Same clamp-then-try_into pattern as RangeDistinctProof
                // above. Carrier proofs commit `(outer_key, sum)` pairs;
                // a truncated outer-walk cap would silently change which
                // pairs end up in the proof.
                let limit_u16 = request
                    .limit
                    .map(|l| l.min(request.drive_config.max_query_limit as u32))
                    .map(|l| {
                        u16::try_from(l).map_err(|_| {
                            Error::Query(crate::error::query::QuerySyntaxError::Unsupported(
                                format!(
                                    "limit {} exceeds u16::MAX for carrier-aggregate sum proof",
                                    l
                                ),
                            ))
                        })
                    })
                    .transpose()?;
                Ok(DocumentSumResponse::Proof(
                    self.execute_document_sum_range_aggregate_carrier_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        sum_property,
                        limit_u16,
                        order_by_ascending,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
        }
    }
}

// `detect_sum_mode` lives in the versioned
// [`mode_detection`](super::mode_detection) module — the routing
// table is consensus-relevant on the query surface and protocol
// versions that change it must do so behind a method-version bump.

/// Parse the wire-CBOR `Value::Array` shape into structured
/// `Vec<WhereClause>`. Delegates to count's parser.
pub fn where_clauses_from_value(value: &Value) -> Result<Vec<WhereClause>, Error> {
    crate::query::drive_document_count_query::drive_dispatcher::where_clauses_from_value(value)
}

/// Parse the wire-CBOR `Value::Array` shape into structured
/// `Vec<OrderClause>`. Delegates to count's parser.
pub fn order_clauses_from_value(value: &Value) -> Result<Vec<OrderClause>, Error> {
    crate::query::drive_document_count_query::drive_dispatcher::order_clauses_from_value(value)
}
