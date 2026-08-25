//! `RoutingDecision::HavingRange` — the boolean-HAVING range
//! surface (PV14).

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
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::{
    DocumentHavingRequest, DocumentHavingResponse, HavingClause, OrderClause, SelectProjection,
    WhereClause,
};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Dispatch a boolean-`HAVING` range request
    /// (`GROUP BY p HAVING <agg> <op> <value> LIMIT n`) to
    /// [`Drive::execute_document_having_request`] and map the response
    /// onto the wire.
    ///
    /// Parallels [`Self::dispatch_ranked_v1`] line-for-line — same
    /// contract/doctype resolution, same error → typed-rejection
    /// mapping, same prove split — because the two surfaces read the
    /// same indexed tree. The response reuses the `RankedEntries`
    /// message (a having page is the same "group key + aggregate value"
    /// entry list), with one deliberate difference: `skipped` is left
    /// unset. Its published contract is "the page's starting rank", and
    /// a value-bounded page has no rank base — the entries are simply
    /// every matching group in axis order, cut at `limit`.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::query::document_query::v1) fn dispatch_having_v1(
        &self,
        data_contract_id: Vec<u8>,
        document_type_name: String,
        select: SelectProjection,
        group_by: Vec<String>,
        having: Vec<HavingClause>,
        where_clauses: Vec<WhereClause>,
        order_clauses: Vec<OrderClause>,
        limit: Option<u32>,
        offset: Option<u32>,
        start: Option<RequestV1Start>,
        prove: bool,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
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

        let drive_request = DocumentHavingRequest {
            contract: contract_ref,
            document_type,
            group_by: &group_by,
            select,
            having: &having,
            order_by: &order_clauses,
            where_clauses: &where_clauses,
            limit,
            offset,
            has_start_at: start.is_some(),
            prove,
        };

        let drive_response =
            match self
                .drive
                .execute_document_having_request(drive_request, None, platform_version)
            {
                Ok(r) => r,
                Err(drive::error::Error::Query(qe)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)));
                }
                // Same empty-tree mapping as the ranked path — and
                // genuinely reachable here: the range prover has no
                // empty-range shape for a completely empty axis
                // secondary (pinned by
                // `an_empty_match_set_reads_empty_and_proves_empty` in
                // rs-drive's having suite), so a proved HAVING request
                // against an index with no documents yet surfaces this
                // merk-level failure.
                Err(e) => match empty_ranking_proof_rejection(&e) {
                    Some(rejection) => {
                        return Ok(QueryValidationResult::new_with_error(rejection));
                    }
                    None => return Err(e.into()),
                },
            };

        let response = match drive_response {
            DocumentHavingResponse::Entries(entries) => GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Data(ResultData {
                    // Always a list, never an aggregate collapse — same
                    // rationale as ranked: even one matching group is
                    // an entry, because the caller needs to know which
                    // group matched, not only that one did.
                    variant: Some(result_data::Variant::Ranked(RankedEntries {
                        // Order preserved verbatim: axis order in the
                        // walk direction, and drive already asserted
                        // the list is no longer than the limit.
                        entries: entries.into_iter().map(into_v1_ranked_entry).collect(),
                        // Deliberately unset. `skipped`'s published
                        // contract is rank-based ("entry i is the
                        // group at rank skipped + i"), and a
                        // value-bounded page has no rank base — there
                        // is nothing the field could truthfully say.
                        skipped: None,
                    })),
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
            DocumentHavingResponse::Proof(proof_bytes) => {
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
