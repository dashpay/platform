//! `RoutingDecision::Ranked` — the ranked top-k surface (PV14).

use super::super::PrefetchedContract;
use super::{empty_ranking_proof_rejection, into_v1_ranked_entry};
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Start as RequestV1Start;
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    result_data, RankedEntries, ResultData,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v1, GetDocumentsResponseV1,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::query::ResolvedTimeRange;
use drive::query::{
    DocumentRankedRequest, DocumentRankedResponse, HavingClause, OrderClause, SelectProjection,
    WhereClause,
};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Dispatch a ranked request — a `COUNT` / `SUM` / `AVG` select
    /// with a `GROUP BY` whose single `ORDER BY` clause names the
    /// selected aggregate — to
    /// [`Drive::execute_document_ranked_request`], and map the response
    /// into a `GetDocumentsResponseV1` carrying a `RankedEntries`
    /// payload (or a `Proof` payload when prove=true).
    ///
    /// Structurally parallel to [`Self::dispatch_count_v1`] — same
    /// contract fetch, same `Error::Query` → typed-rejection mapping,
    /// same prove vs no-prove split — with two differences worth
    /// naming:
    ///
    /// 1. **The request is forwarded whole.** `where_clauses`,
    ///    `having`, `order_by`, `limit`, `offset` and `start` all go
    ///    down, including the ones a ranked request must leave empty,
    ///    because drive owns those rejections: the SDK's client-side
    ///    helpers call drive's validator with no abci in the path, so
    ///    re-checking here would create a second, driftable copy of
    ///    the grammar. The rejections come back as `Error::Query(...)`
    ///    and are surfaced to the caller as query errors, not internal
    ///    ones. `order_by` in particular is no longer refused here —
    ///    it is the ranking.
    /// 2. **Proving an empty ranking is mapped, not propagated.** See
    ///    [`empty_ranking_proof_rejection`].
    #[allow(clippy::too_many_arguments)]
    pub(in crate::query::document_query::v1) fn dispatch_ranked_v1(
        &self,
        prefetched_contract: PrefetchedContract,
        data_contract_id: Vec<u8>,
        document_type_name: String,
        select: SelectProjection,
        group_by: Vec<String>,
        having: Vec<HavingClause>,
        where_clauses: Vec<WhereClause>,
        resolved_time_ranges: Vec<ResolvedTimeRange>,
        order_clauses: Vec<OrderClause>,
        limit: Option<u32>,
        offset: Option<u32>,
        start: Option<RequestV1Start>,
        prove: bool,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        // The request's single contract fetch: reuse the time-range
        // resolution's when it ran, fetch now otherwise — after the cheap
        // shape guards above, so their rejections keep precedence.
        let (contract_id, contract_fetch_info) = check_validation_result_with_data!(self
            .contract_for_aggregate_dispatch(
                prefetched_contract,
                data_contract_id,
                platform_version
            )?);
        let contract_ref = &contract_fetch_info.contract;
        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                document_type_name, contract_id
            ))));

        let drive_request = DocumentRankedRequest {
            contract: contract_ref,
            document_type,
            group_by: &group_by,
            select,
            having: &having,
            order_by: &order_clauses,
            where_clauses: &where_clauses,
            resolved_time_ranges: &resolved_time_ranges,
            limit,
            offset,
            has_start_at: start.is_some(),
            prove,
        };

        let drive_response =
            match self
                .drive
                .execute_document_ranked_request(drive_request, None, platform_version)
            {
                Ok(r) => r,
                Err(drive::error::Error::Query(qe)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)));
                }
                Err(e) => match empty_ranking_proof_rejection(&e) {
                    Some(rejection) => {
                        return Ok(QueryValidationResult::new_with_error(rejection));
                    }
                    None => return Err(e.into()),
                },
            };

        let response = match drive_response {
            DocumentRankedResponse::Entries(page) => GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Data(ResultData {
                    // No aggregate-collapse arm here, unlike count /
                    // sum / average: a ranked result is always a list
                    // of groups. Even `LIMIT 1` returns one *entry*,
                    // because the caller needs to know which group
                    // won, not only the winning value.
                    variant: Some(result_data::Variant::Ranked(RankedEntries {
                        // Order is preserved verbatim: entry order is
                        // the ranking order, and drive already
                        // asserted the list is no longer than `k`.
                        entries: page.entries.into_iter().map(into_v1_ranked_entry).collect(),
                        // The page's starting rank, so entry `i` is
                        // identifiable as the group at rank
                        // `skipped + i` rather than as "one of the
                        // top few". This is the *unproven* path, so
                        // the number is only as good as the node —
                        // which is exactly why a proving client
                        // ignores it and re-derives the attested
                        // value from the proof bytes instead (see
                        // `RankedPage::skipped`). Sent as `Some`
                        // unconditionally, including the `0` an
                        // offset-less query produces: the proto field
                        // is `optional` to keep "this node predates
                        // the field" distinguishable from "this page
                        // starts at rank 0", and collapsing 0 to
                        // `None` would throw that distinction away.
                        skipped: Some(page.skipped),
                    })),
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
            DocumentRankedResponse::Proof(proof_bytes) => {
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
