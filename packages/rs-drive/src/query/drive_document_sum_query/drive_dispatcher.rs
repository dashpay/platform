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
    DocumentSumMode, DocumentSumRequest, DocumentSumResponse, RangeSumOptions, RangeSumWalkMode,
    SumMode,
};
use crate::query::{OrderClause, WhereClause};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

fn effective_no_proof_distinct_limit(
    requested_limit: Option<u32>,
    drive_config: &crate::config::DriveConfig,
) -> Result<u16, Error> {
    let effective_limit = requested_limit
        .unwrap_or(drive_config.default_query_limit as u32)
        .min(drive_config.max_query_limit as u32);

    if effective_limit == 0 {
        return Err(Error::Query(
            crate::error::query::QuerySyntaxError::InvalidLimit(
                "effective distinct SUM limit must be greater than zero".to_string(),
            ),
        ));
    }

    // Both configuration limits are u16, and the `min` above bounds every
    // caller-supplied value to `max_query_limit` before this conversion.
    Ok(effective_limit as u16)
}

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
        let resolved_time_range_fields = request.resolved_time_range_fields;
        // Same provenance-vs-shape contract as the count and joint
        // dispatchers; sum has no canonicalize step, so the guard anchors
        // here.
        crate::query::validate_resolved_time_range_clause_shapes(
            &where_clauses,
            &resolved_time_range_fields,
        )?;
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
                    &resolved_time_range_fields,
                    sum_property,
                    transaction,
                    platform_version,
                )?;
                let total = entries.first().and_then(|e| e.sum).unwrap_or(0);
                Ok(DocumentSumResponse::Aggregate(total))
            }
            DocumentSumMode::PerInValue => {
                let options = RangeSumOptions {
                    walk_mode: RangeSumWalkMode::Aggregate,
                    carrier_outer_limit: None,
                    left_to_right: order_by_ascending,
                };
                Ok(DocumentSumResponse::Entries(
                    self.execute_document_sum_per_in_value_no_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        &resolved_time_range_fields,
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
                let walk_mode = if return_distinct {
                    RangeSumWalkMode::Distinct(effective_no_proof_distinct_limit(
                        request.limit,
                        request.drive_config,
                    )?)
                } else {
                    RangeSumWalkMode::Aggregate
                };
                let options = RangeSumOptions {
                    walk_mode,
                    carrier_outer_limit: None,
                    left_to_right: order_by_ascending,
                };
                let entries = self.execute_document_sum_range_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    &resolved_time_range_fields,
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
                    &resolved_time_range_fields,
                    sum_property,
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentSumMode::RangeDistinctProof => {
                // Validate-don't-clamp limit policy on the prove path:
                // client-side proof reconstruction needs the EXACT
                // limit value the server applied to the path query
                // (the SDK rebuilds the same `SizedQuery::limit` for
                // merk-root recomputation). Silent clamping or a
                // tuned `default_query_limit` would byte-differ the
                // reconstructed path query and break verification.
                //
                // Limit fallback uses [`crate::config::DEFAULT_QUERY_LIMIT`]
                // (compile-time constant), NOT
                // `drive_config.default_query_limit` (operator-tunable
                // runtime value). `max_query_limit` still gates the
                // request as a DoS-protection knob — proofs never
                // cross the operator-set ceiling, but the ceiling
                // itself doesn't shape proof bytes; it only decides
                // whether the request gets served.
                //
                // Mirrors count's policy at
                // `drive_document_count_query::drive_dispatcher`
                // `DocumentCountMode::RangeDistinctProof`.
                let effective_limit = request
                    .limit
                    .unwrap_or(crate::config::DEFAULT_QUERY_LIMIT as u32);
                if effective_limit > request.drive_config.max_query_limit as u32 {
                    return Err(Error::Query(
                        crate::error::query::QuerySyntaxError::InvalidLimit(format!(
                            "limit {} exceeds max_query_limit {} on the prove + \
                             distinct-walk path (GROUP BY a range field, SUM); \
                             reduce the requested limit or use prove = false",
                            effective_limit, request.drive_config.max_query_limit
                        )),
                    ));
                }
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
                        &resolved_time_range_fields,
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
                    &resolved_time_range_fields,
                    sum_property,
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentSumMode::RangeAggregateCarrierProof => {
                // Validate-don't-clamp limit policy on the prove path
                // — same contract as RangeDistinctProof above. The
                // carrier proof's outer-walk cap is `SizedQuery::limit`
                // bytes-of-proof material; a silent clamp would
                // byte-differ the SDK's reconstruction and break
                // verification. Unlike the distinct arm, the carrier
                // arm passes `Option<u16>` (None = unbounded outer
                // walk), so the request's `None` stays `None` instead
                // of falling back to a default.
                let limit_u16 = request
                    .limit
                    .map(|l| {
                        if l > request.drive_config.max_query_limit as u32 {
                            return Err(Error::Query(
                                crate::error::query::QuerySyntaxError::InvalidLimit(format!(
                                    "limit {} exceeds max_query_limit {} on the prove + \
                                     carrier-aggregate path (GROUP BY In + range, SUM); \
                                     reduce the requested limit or use prove = false",
                                    l, request.drive_config.max_query_limit
                                )),
                            ));
                        }
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
                        &resolved_time_range_fields,
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
pub fn where_clauses_from_value(
    value: &Value,
    platform_version: &PlatformVersion,
) -> Result<Vec<WhereClause>, Error> {
    crate::query::drive_document_count_query::drive_dispatcher::where_clauses_from_value(
        value,
        platform_version,
    )
}

/// Parse the wire-CBOR `Value::Array` shape into structured
/// `Vec<OrderClause>`. Delegates to count's parser.
pub fn order_clauses_from_value(value: &Value) -> Result<Vec<OrderClause>, Error> {
    crate::query::drive_document_count_query::drive_dispatcher::order_clauses_from_value(value)
}

#[cfg(test)]
mod tests {
    use super::effective_no_proof_distinct_limit;
    use crate::config::DriveConfig;

    #[test]
    fn no_proof_distinct_limit_uses_the_default_and_clamps_to_the_maximum() {
        let config = DriveConfig {
            default_query_limit: 25,
            max_query_limit: 100,
            ..DriveConfig::default()
        };

        assert_eq!(
            effective_no_proof_distinct_limit(None, &config).unwrap(),
            25
        );
        assert_eq!(
            effective_no_proof_distinct_limit(Some(7), &config).unwrap(),
            7
        );
        assert_eq!(
            effective_no_proof_distinct_limit(Some(10_000), &config).unwrap(),
            100
        );

        let disabled = DriveConfig {
            max_query_limit: 0,
            ..config
        };
        assert!(effective_no_proof_distinct_limit(None, &disabled).is_err());
    }
}
