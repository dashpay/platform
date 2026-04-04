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

        let mut drive_query =
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

        // Remove the limit so we count ALL matching documents, not just up to the
        // default query limit. A count query needs to return the total number of
        // documents matching the where clause.
        drive_query.limit = None;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_documents_count_no_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        store_data_contract(&platform, &data_contract, version);

        let data_contract_id = data_contract.id();
        let document_type_name = "person";
        let document_type = data_contract
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(500);
        for _ in 0..5 {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                &data_contract,
                document_type,
                &random_document,
                platform_version,
            );
        }

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Count(count)),
                metadata: Some(_),
            }) => {
                assert_eq!(count, 5, "expected count of 5 documents");
            }
            other => panic!("expected count result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_count_empty_result() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        store_data_contract(&platform, &data_contract, version);

        let data_contract_id = data_contract.id();
        let document_type_name = "person";

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Count(count)),
                metadata: Some(_),
            }) => {
                assert_eq!(count, 0, "expected count of 0 documents");
            }
            other => panic!("expected count result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_count_with_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        store_data_contract(&platform, &data_contract, version);

        let data_contract_id = data_contract.id();
        let document_type_name = "person";
        let document_type = data_contract
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(500);
        for _ in 0..3 {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                &data_contract,
                document_type,
                &random_document,
                platform_version,
            );
        }

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            prove: true,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        assert!(matches!(
            result.data,
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
