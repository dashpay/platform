//! `RoutingDecision::Average` — the grouped average surface.

use super::super::not_yet_implemented;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Start as RequestV1Start;
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    average_results, result_data, AverageAggregate, AverageEntries, AverageEntry, AverageResults,
    ResultData,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v1, GetDocumentsResponseV1,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::{
    AverageEntry as DriveAverageEntry, AverageMode, CountMode, DocumentAverageRequest,
    DocumentAverageResponse, OrderClause, WhereClause,
};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Dispatch a `select = AVG(field)` request to
    /// [`Drive::execute_document_average_request`] and map the response
    /// into a `GetDocumentsResponseV1` carrying an `AverageResults`
    /// payload (or a `Proof` payload when prove=true).
    ///
    /// Parallels [`Self::dispatch_sum_v1`] line-by-line — same request
    /// construction, same error → typed-rejection mapping, same prove
    /// vs no-prove split. The response shape mapping differs:
    /// `DocumentAverageResponse::Aggregate { count, sum }` →
    /// `AverageResults::aggregate_average`,
    /// `DocumentAverageResponse::Entries(_)` → `AverageResults::entries`,
    /// `DocumentAverageResponse::Proof(_)` → outer `result.proof`.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::query::document_query::v1) fn dispatch_average_v1(
        &self,
        data_contract_id: Vec<u8>,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        resolved_time_range_fields: Vec<String>,
        order_clauses: Vec<OrderClause>,
        limit: Option<u32>,
        start: Option<RequestV1Start>,
        prove: bool,
        sum_property: String,
        mode: CountMode,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        if start.is_some() {
            return Ok(QueryValidationResult::new_with_error(not_yet_implemented(
                "start_after / start_at with SELECT AVG (paginate by narrowing the \
                 range clause itself)",
            )));
        }

        let contract_id: Identifier =
            check_validation_result_with_data!(data_contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let (_, contract_fetch_info) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;
        let contract_fetch_info = check_validation_result_with_data!(contract_fetch_info.ok_or(
            QueryError::Query(QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info",
            ))
        ));
        let contract_ref = &contract_fetch_info.contract;
        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                document_type_name, contract_id
            ))));

        // `AverageMode` mirrors `CountMode` 1:1 — map across.
        let avg_mode = match mode {
            CountMode::Aggregate => AverageMode::Aggregate,
            CountMode::GroupByIn => AverageMode::GroupByIn,
            CountMode::GroupByRange => AverageMode::GroupByRange,
            CountMode::GroupByCompound => AverageMode::GroupByCompound,
        };

        let drive_request = DocumentAverageRequest {
            contract: contract_ref,
            document_type,
            sum_property,
            where_clauses,
            resolved_time_range_fields,
            order_clauses,
            mode: avg_mode,
            limit,
            prove,
            drive_config: &self.config.drive,
        };
        let drive_response =
            match self
                .drive
                .execute_document_average_request(drive_request, None, platform_version)
            {
                Ok(r) => r,
                Err(drive::error::Error::Query(qe)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)));
                }
                Err(e) => return Err(e.into()),
            };

        let response = match drive_response {
            DocumentAverageResponse::Aggregate { count, sum } => GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Data(ResultData {
                    variant: Some(result_data::Variant::Averages(AverageResults {
                        variant: Some(average_results::Variant::AggregateAverage(
                            AverageAggregate { count, sum },
                        )),
                    })),
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
            DocumentAverageResponse::Entries(entries) => {
                if avg_mode == AverageMode::Aggregate {
                    // Mirror sum-side's fold for the `select=AVG,
                    // group_by=[]` + PerInValue executor combo. Fold
                    // both count and sum across In branches. Either
                    // axis overflowing is surfaced as a typed
                    // `QuerySyntaxError::Unsupported` so the client
                    // doesn't get a silently-saturated answer to
                    // divide against (which would also misreport the
                    // average).
                    let mut total_count: u64 = 0;
                    let mut total_sum: i64 = 0;
                    let mut overflow_axis: Option<&'static str> = None;
                    for e in &entries {
                        match total_count.checked_add(e.count.unwrap_or(0)) {
                            Some(c) => total_count = c,
                            None => {
                                overflow_axis = Some("count");
                                break;
                            }
                        }
                        match total_sum.checked_add(e.sum.unwrap_or(0)) {
                            Some(s) => total_sum = s,
                            None => {
                                overflow_axis = Some("sum");
                                break;
                            }
                        }
                    }
                    if let Some(axis) = overflow_axis {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            QuerySyntaxError::Unsupported(format!(
                                "aggregate AVG across In branches overflows {axis} \
                                 ({} axis range); narrow the In set or query branches \
                                 individually",
                                if axis == "count" { "u64" } else { "i64" },
                            )),
                        )));
                    }
                    GetDocumentsResponseV1 {
                        result: Some(get_documents_response_v1::Result::Data(ResultData {
                            variant: Some(result_data::Variant::Averages(AverageResults {
                                variant: Some(average_results::Variant::AggregateAverage(
                                    AverageAggregate {
                                        count: total_count,
                                        sum: total_sum,
                                    },
                                )),
                            })),
                        })),
                        metadata: Some(
                            self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                        ),
                    }
                } else {
                    GetDocumentsResponseV1 {
                        result: Some(get_documents_response_v1::Result::Data(ResultData {
                            variant: Some(result_data::Variant::Averages(AverageResults {
                                variant: Some(average_results::Variant::Entries(AverageEntries {
                                    entries: entries
                                        .into_iter()
                                        .map(into_v1_average_entry)
                                        .collect(),
                                })),
                            })),
                        })),
                        metadata: Some(
                            self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                        ),
                    }
                }
            }
            DocumentAverageResponse::Proof(proof_bytes) => {
                let (grovedb_used, proof) =
                    self.response_proof_v0(platform_state, proof_bytes, GroveDBToUse::Current)?;
                GetDocumentsResponseV1 {
                    result: Some(get_documents_response_v1::Result::Proof(proof)),
                    metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
                }
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

/// Translate an rs-drive `AverageEntry` into the wire `AverageEntry`.
/// Mirror of [`into_v1_entry`] + [`into_v1_sum_entry`] for the average
/// surface (carries both count and sum so the client can divide).
///
/// `zip_entries` in `drive_document_average_query::drive_dispatcher`
/// performs a strict two-pointer merge that errors out as
/// `CorruptedCodeExecution` on any per-`(in_key, key)` divergence
/// between the count and sum streams. So by the time an entry reaches
/// this mapper, both axes have already been asserted to agree on
/// `Some`-vs-`None` for the same key — meaning the dangerous
/// `(count: None, sum: Some(V))` bucket that could let a client
/// divide V by 0 cannot exist. The `unwrap_or(0)` below is therefore
/// defense-in-depth (same as [`into_v1_entry`] / [`into_v1_sum_entry`]
/// for individual count / sum entries) rather than load-bearing.
fn into_v1_average_entry(e: DriveAverageEntry) -> AverageEntry {
    AverageEntry {
        in_key: e.in_key,
        key: e.key,
        count: e.count.unwrap_or(0),
        sum: e.sum.unwrap_or(0),
    }
}
