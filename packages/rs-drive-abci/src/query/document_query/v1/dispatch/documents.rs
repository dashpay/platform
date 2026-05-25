//! `RoutingDecision::Documents` — the matched-documents fetch path.

use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start as RequestV0Start;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Start as RequestV1Start;
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    result_data, Documents, ResultData,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v0, get_documents_response_v1, GetDocumentsResponseV1,
};
use dpp::version::PlatformVersion;
use drive::query::{OrderClause, WhereClause};

impl<C> Platform<C> {
    /// Forward a `select = DOCUMENTS` request through the shared
    /// `query_documents_typed` helper that v0 also dispatches into.
    /// v1 doesn't add any documents-side capability — the SQL-shaped
    /// fields (`select`, `group_by`, `having`) are all validated as
    /// documents-compatible above (empty `group_by`, empty `having`,
    /// etc.) before reaching here.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::query::document_query::v1) fn dispatch_documents_v1(
        &self,
        data_contract_id: Vec<u8>,
        document_type: String,
        where_clauses: Vec<WhereClause>,
        resolved_time_range_fields: Vec<String>,
        order_by_clauses: Vec<OrderClause>,
        limit: Option<u32>,
        start: Option<RequestV1Start>,
        prove: bool,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        let start = start.map(|s| match s {
            RequestV1Start::StartAfter(b) => RequestV0Start::StartAfter(b),
            RequestV1Start::StartAt(b) => RequestV0Start::StartAt(b),
        });
        // `limit` is `optional uint32` on v1; the typed helper takes
        // `Option<u32>` directly (`None` → server default). `Some(0)`
        // can't reach here — `validate_and_route` rejects it for
        // every SELECT mode so the v1 contract is uniform; only
        // `None` or `Some(N > 0)` survive.
        let result = self.query_documents_typed(
            data_contract_id,
            document_type,
            where_clauses,
            resolved_time_range_fields,
            order_by_clauses,
            limit,
            prove,
            start,
            platform_state,
            platform_version,
        )?;
        Ok(result.map(translate_documents_v0_to_v1))
    }
}

/// Translate a v0 `GetDocumentsResponseV0` into v1's response
/// envelope (Documents-or-Proof wrapping the v0 oneof result into
/// v1's `ResultData`-or-`Proof` shape).
fn translate_documents_v0_to_v1(
    response_v0: dapi_grpc::platform::v0::get_documents_response::GetDocumentsResponseV0,
) -> GetDocumentsResponseV1 {
    let metadata = response_v0.metadata;
    let result = match response_v0.result {
        Some(get_documents_response_v0::Result::Documents(docs)) => {
            Some(get_documents_response_v1::Result::Data(ResultData {
                variant: Some(result_data::Variant::Documents(Documents {
                    documents: docs.documents,
                })),
            }))
        }
        Some(get_documents_response_v0::Result::Proof(proof)) => {
            Some(get_documents_response_v1::Result::Proof(proof))
        }
        None => None,
    };
    GetDocumentsResponseV1 { result, metadata }
}
