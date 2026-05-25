//! `RoutingDecision::Sum` — the grouped sum surface.

use super::super::not_yet_implemented;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Start as RequestV1Start;
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    result_data, sum_results, ResultData, SumEntries, SumEntry, SumResults,
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
    CountMode, DocumentSumRequest, DocumentSumResponse, OrderClause, SumEntry as DriveSumEntry,
    SumMode, WhereClause,
};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Dispatch a `select = SUM(field)` request to
    /// [`Drive::execute_document_sum_request`] and map the response
    /// into a `GetDocumentsResponseV1` carrying a `SumResults` payload
    /// (or a `Proof` payload when prove=true).
    ///
    /// Parallels [`Self::dispatch_count_v1`] line-by-line — same
    /// request construction, same error → typed-rejection mapping,
    /// same prove vs no-prove split. Only the response shape mapping
    /// differs: `DocumentSumResponse::Aggregate(i64)` →
    /// `SumResults::aggregate_sum`, `Entries(Vec<SumEntry>)` →
    /// `SumResults::entries`, `Proof(bytes)` → outer `result.proof`.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::query::document_query::v1) fn dispatch_sum_v1(
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
                "start_after / start_at with SELECT SUM (paginate by narrowing the \
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

        // `SumMode` mirrors `CountMode` 1:1 — same four variants
        // computed via the same `compute_aggregate_mode_and_check_limit`
        // helper. Map across the isomorphism.
        let sum_mode = match mode {
            CountMode::Aggregate => SumMode::Aggregate,
            CountMode::GroupByIn => SumMode::GroupByIn,
            CountMode::GroupByRange => SumMode::GroupByRange,
            CountMode::GroupByCompound => SumMode::GroupByCompound,
        };

        let drive_request = DocumentSumRequest {
            contract: contract_ref,
            document_type,
            sum_property,
            where_clauses,
            resolved_time_range_fields,
            order_clauses,
            mode: sum_mode,
            limit,
            prove,
            drive_config: &self.config.drive,
        };
        let drive_response =
            match self
                .drive
                .execute_document_sum_request(drive_request, None, platform_version)
            {
                Ok(r) => r,
                Err(drive::error::Error::Query(qe)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)));
                }
                Err(e) => return Err(e.into()),
            };

        let response = match drive_response {
            DocumentSumResponse::Aggregate(sum) => GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Data(ResultData {
                    variant: Some(result_data::Variant::Sums(SumResults {
                        variant: Some(sum_results::Variant::AggregateSum(sum)),
                    })),
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
            DocumentSumResponse::Entries(entries) => {
                if sum_mode == SumMode::Aggregate {
                    // Mirror of count's same-arm: `select=SUM,
                    // group_by=[]` whose executor routed through a
                    // PerInValue path (In + no range + no prove)
                    // returns one entry per In branch. Fold them into
                    // a single aggregate. `checked_add` surfaces the
                    // narrow case where per-branch sums truly add to
                    // more than i64::MAX as a typed
                    // `QuerySyntaxError::Unsupported` rather than
                    // silently saturating at i64::MAX (which produces
                    // a deterministic-but-misleading answer).
                    let mut total: i64 = 0;
                    let mut overflow = false;
                    for e in &entries {
                        match total.checked_add(e.sum.unwrap_or(0)) {
                            Some(t) => total = t,
                            None => {
                                overflow = true;
                                break;
                            }
                        }
                    }
                    if overflow {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            QuerySyntaxError::Unsupported(
                                "aggregate SUM across In branches overflows i64 — \
                                 the In-fold cannot be represented; narrow the In set \
                                 or query branches individually"
                                    .to_string(),
                            ),
                        )));
                    }
                    GetDocumentsResponseV1 {
                        result: Some(get_documents_response_v1::Result::Data(ResultData {
                            variant: Some(result_data::Variant::Sums(SumResults {
                                variant: Some(sum_results::Variant::AggregateSum(total)),
                            })),
                        })),
                        metadata: Some(
                            self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                        ),
                    }
                } else {
                    GetDocumentsResponseV1 {
                        result: Some(get_documents_response_v1::Result::Data(ResultData {
                            variant: Some(result_data::Variant::Sums(SumResults {
                                variant: Some(sum_results::Variant::Entries(SumEntries {
                                    entries: entries.into_iter().map(into_v1_sum_entry).collect(),
                                })),
                            })),
                        })),
                        metadata: Some(
                            self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                        ),
                    }
                }
            }
            DocumentSumResponse::Proof(proof_bytes) => {
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

/// Translate an rs-drive `SumEntry` into the wire `SumEntry`. Mirror
/// of [`into_v1_entry`] for the sum surface.
fn into_v1_sum_entry(e: DriveSumEntry) -> SumEntry {
    SumEntry {
        in_key: e.in_key,
        key: e.key,
        // `sum` is `sint64` on the wire — same `None`-rounds-to-0
        // contract as `into_v1_entry`.
        sum: e.sum.unwrap_or(0),
    }
}
