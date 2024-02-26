use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV0;
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v0, GetDocumentsResponseV0,
};
use dapi_grpc::platform::v0::{get_documents_response, GetDocumentsResponse};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::DriveQuery;

impl<C> Platform<C> {
    pub(super) fn query_documents_v0(
        &self,
        GetDocumentsRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            order_by,
            limit,
            prove,
            start,
        }: GetDocumentsRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV0>, Error> {
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

            let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

            GetDocumentsResponseV0 {
                result: Some(get_documents_response_v0::Result::Proof(proof)),
                metadata: Some(metadata),
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

            GetDocumentsResponseV0 {
                result: Some(get_documents_response_v0::Result::Documents(
                    get_documents_response_v0::Documents { documents: results },
                )),
                metadata: Some(self.response_metadata_v0()),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform, store_data_contract};
    use dpp::tests::fixtures::get_data_contract_fixture;

    #[test]
    fn test_invalid_document_id() {
        let (platform, version) = setup_platform();

        let request = GetDocumentsRequestV0 {
            data_contract_id: vec![0; 8],
            document_type: "niceDocument".to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_data_contract_not_found_in_documents_request() {
        let (platform, version) = setup_platform();

        let data_contract_id = vec![0; 32];

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.clone(),
            document_type: "niceDocument".to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DataContractNotFound(msg))] if msg == &"contract not found when querying from value with contract info"
        ));
    }

    #[test]
    fn test_absent_document_type() {
        let (platform, version) = setup_platform();

        let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "fakeDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains(format!(
            "document type {} not found for contract",
            document_type,
        ).as_str())))
    }

    #[test]
    fn test_invalid_where_clause() {
        let (platform, version) = setup_platform();

        let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![0x9F], // Incomplete CBOR array
            limit: 0,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DeserializationError(msg))] if msg == "unable to decode 'where' query from cbor"
        ))
    }

    #[test]
    fn test_invalid_order_by_clause() {
        let (platform, version) = setup_platform();

        let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![0x9F], // Incomplete CBOR array
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DeserializationError(msg))] if msg == "unable to decode 'order_by' query from cbor"
        ));
    }

    #[test]
    fn test_invalid_start_at_clause() {
        let (platform, version) = setup_platform();

        let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: Some(Start::StartAt(vec![0; 8])),
        };

        let result = platform
            .query_documents_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(msg))] if msg == &"start at should be a 32 byte identifier"
        ))
    }

    #[test]
    fn test_invalid_start_after_clause() {
        let (platform, version) = setup_platform();

        let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: Some(Start::StartAfter(vec![0; 8])),
        };

        let result = platform
            .query_documents_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(msg))] if msg == &"start after should be a 32 byte identifier"
        ));
    }

    #[test]
    fn test_invalid_limit() {
        let (platform, version) = setup_platform();

        let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";
        let limit = u32::MAX;

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidLimit(msg))] if msg == &format!("limit {} out of bounds", limit)
        ))
    }

    #[test]
    fn test_documents_not_found() {
        let (platform, version) = setup_platform();

        let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetDocumentsResponseV0 {
                result: Some(get_documents_response_v0::Result::Documents(documents)),
                metadata: Some(_),
            }) if documents.documents.is_empty()
        ));
    }

    #[test]
    fn test_documents_absence_proof() {
        let (platform, version) = setup_platform();

        let created_data_contract = get_data_contract_fixture(None, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: true,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetDocumentsResponseV0 {
                result: Some(get_documents_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
