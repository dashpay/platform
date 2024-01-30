use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV0;
use dapi_grpc::platform::v0::get_documents_response::GetDocumentsResponseV0;
use dapi_grpc::platform::v0::{get_documents_response, GetDocumentsResponse, Proof};
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
        request: GetDocumentsRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.validator_set_quorum_type() as u32;
        let GetDocumentsRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            order_by,
            limit,
            prove,
            start,
        } = request;
        let contract_id: Identifier = check_validation_result_with_data!(data_contract_id
            .try_into()
            .map_err(|_| QueryError::InvalidArgument(
                "id must be a valid identifier (32 bytes long)".to_string()
            )));
        let (_, contract) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;
        let contract = check_validation_result_with_data!(contract.ok_or(QueryError::Query(
            QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info",
            )
        )));
        let contract_ref = &contract.contract;
        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                document_type_name, contract_id
            ))));

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
        let response_data = if prove {
            let proof =
                match drive_query.execute_with_proof(&self.drive, None, None, platform_version) {
                    Ok(result) => result.0,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };
            GetDocumentsResponse {
                version: Some(get_documents_response::Version::V0(
                    GetDocumentsResponseV0 {
                        result: Some(
                            get_documents_response::get_documents_response_v0::Result::Proof(
                                Proof {
                                    grovedb_proof: proof,
                                    quorum_hash: state.last_committed_quorum_hash().to_vec(),
                                    quorum_type,
                                    block_id_hash: state.last_committed_block_id_hash().to_vec(),
                                    signature: state.last_committed_block_signature().to_vec(),
                                    round: state.last_committed_block_round(),
                                },
                            ),
                        ),
                        metadata: Some(metadata),
                    },
                )),
            }
            .encode_to_vec()
        } else {
            let results = match drive_query.execute_raw_results_no_proof(
                &self.drive,
                None,
                None,
                platform_version,
            ) {
                Ok(result) => result.0,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(e) => return Err(e.into()),
            };

            GetDocumentsResponse {
                version: Some(get_documents_response::Version::V0(
                    GetDocumentsResponseV0 {
                        result: Some(
                            get_documents_response::get_documents_response_v0::Result::Documents(
                                get_documents_response::get_documents_response_v0::Documents {
                                    documents: results,
                                },
                            ),
                        ),
                        metadata: Some(metadata),
                    },
                )),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
