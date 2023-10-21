use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::Start;
use dapi_grpc::platform::v0::{
    get_documents_response, GetDocumentsRequest, GetDocumentsResponse, Proof,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::DriveQuery;
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_documents_v0(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.quorum_type() as u32;
        let GetDocumentsRequest {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            order_by,
            limit,
            prove,
            start,
        } = check_validation_result_with_data!(GetDocumentsRequest::decode(query_data));
        let contract_id: Identifier = check_validation_result_with_data!(data_contract_id
            .try_into()
            .map_err(|_| QueryError::InvalidArgument(
                "id must be a valid identifier (32 bytes long)".to_string()
            )));
        let (_, contract) =
            check_validation_result_with_data!(self.drive.get_contract_with_fetch_info_and_fee(
                contract_id.to_buffer(),
                None,
                true,
                None,
                platform_version,
            ));
        let contract = check_validation_result_with_data!(contract.ok_or(QueryError::Query(
            QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info",
            )
        )));
        let contract_ref = &contract.contract;
        let document_type = check_validation_result_with_data!(
            contract_ref.document_type_for_name(document_type_name.as_str())
        );

        let where_clause = if r#where.is_empty() {
            Value::Null
        } else {
            check_validation_result_with_data!(ciborium::de::from_reader(r#where.as_slice())
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode 'where' query from cbor".to_string(),
                    ))
                }))
        };

        // TODO: fix?
        //   Fails with "query syntax error: deserialization error: unable to decode 'order_by' query from cbor"
        //   cbor deserialization fails if order_by is empty
        let order_by = if !order_by.is_empty() {
            check_validation_result_with_data!(ciborium::de::from_reader(order_by.as_slice())
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode 'order_by' query from cbor".to_string(),
                    ))
                }))
        } else {
            None
        };

        let (start_at_included, start_at) = if let Some(start) = start {
            match start {
                Start::StartAfter(after) => (
                    false,
                    Some(check_validation_result_with_data!(after
                        .try_into()
                        .map_err(|_| QueryError::Query(
                            QuerySyntaxError::InvalidStartsWithClause(
                                "start after should be a 32 byte identifier",
                            )
                        )))),
                ),
                Start::StartAt(at) => (
                    true,
                    Some(check_validation_result_with_data!(at.try_into().map_err(
                        |_| QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(
                            "start at should be a 32 byte identifier",
                        ))
                    ))),
                ),
            }
        } else {
            (true, None)
        };

        if limit > u16::MAX as u32 {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!("limit {} out of bounds", limit)),
            )));
        }

        let drive_query = check_validation_result_with_data!(DriveQuery::from_decomposed_values(
            where_clause,
            order_by,
            Some(if limit == 0 {
                self.config.drive.default_query_limit
            } else {
                limit as u16
            }),
            start_at,
            start_at_included,
            None,
            contract_ref,
            document_type,
            &self.config.drive,
        ));
        let response_data =
            if prove {
                let (proof, _) = check_validation_result_with_data!(
                    drive_query.execute_with_proof(&self.drive, None, None, platform_version)
                );
                GetDocumentsResponse {
                    result: Some(get_documents_response::Result::Proof(Proof {
                        grovedb_proof: proof,
                        quorum_hash: state.last_quorum_hash().to_vec(),
                        quorum_type,
                        block_id_hash: state.last_block_id_hash().to_vec(),
                        signature: state.last_block_signature().to_vec(),
                        round: state.last_block_round(),
                    })),
                    metadata: Some(metadata),
                }
                .encode_to_vec()
            } else {
                let results = check_validation_result_with_data!(drive_query
                    .execute_raw_results_no_proof(&self.drive, None, None, platform_version))
                .0;
                GetDocumentsResponse {
                    result: Some(get_documents_response::Result::Documents(
                        get_documents_response::Documents { documents: results },
                    )),
                    metadata: Some(metadata),
                }
                .encode_to_vec()
            };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
