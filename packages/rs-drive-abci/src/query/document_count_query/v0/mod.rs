use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_count_request::GetDocumentsCountRequestV0;
use dapi_grpc::platform::v0::get_documents_count_response::{
    get_documents_count_response_v0, GetDocumentsCountResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::DriveDocumentQuery;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_documents_count_v0(
        &self,
        GetDocumentsCountRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            prove,
        }: GetDocumentsCountRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsCountResponseV0>, Error> {
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

        let drive_query =
            check_validation_result_with_data!(DriveDocumentQuery::from_decomposed_values(
                where_clause,
                None,
                Some(self.config.drive.default_query_limit),
                None,
                true,
                None,
                contract_ref,
                document_type,
                &self.config.drive,
            ));

        let response = if prove {
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

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
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

            let count = results.len() as u64;

            GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Count(count)),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
