use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_split_count_request::GetDocumentsSplitCountRequestV0;
use dapi_grpc::platform::v0::get_documents_split_count_response::{
    get_documents_split_count_response_v0, GetDocumentsSplitCountResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::DriveDocumentQuery;
use drive::util::grove_operations::GroveDBToUse;
use std::collections::BTreeMap;

impl<C> Platform<C> {
    pub(super) fn query_documents_split_count_v0(
        &self,
        GetDocumentsSplitCountRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            split_count_by_index_property,
            prove,
        }: GetDocumentsSplitCountRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsSplitCountResponseV0>, Error> {
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

        // Validate the split property exists in the document type
        if split_count_by_index_property.is_empty() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(
                    "split_count_by_index_property must not be empty".to_string(),
                ),
            ));
        }

        // Check that the property exists in the document type schema
        if document_type
            .properties()
            .get(split_count_by_index_property.as_str())
            .is_none()
        {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "property {} not found in document type {}",
                    split_count_by_index_property, document_type_name
                )),
            ));
        }

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

            GetDocumentsSplitCountResponseV0 {
                result: Some(get_documents_split_count_response_v0::Result::Proof(proof)),
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

            // Deserialize documents and split count by the specified property
            let mut counts_by_key: BTreeMap<Vec<u8>, u64> = BTreeMap::new();

            for raw_document in &results {
                let document =
                    check_validation_result_with_data!(dpp::document::Document::from_bytes(
                        raw_document.as_slice(),
                        document_type,
                        platform_version,
                    )
                    .map_err(|e| QueryError::InvalidArgument(format!(
                        "failed to deserialize document: {}",
                        e
                    ))));

                let key = if let Some(value) =
                    document.properties().get(&split_count_by_index_property)
                {
                    // Serialize the property value to CBOR bytes for the key
                    value.to_cbor_buffer().unwrap_or_default()
                } else {
                    // Null / missing key
                    Vec::new()
                };

                *counts_by_key.entry(key).or_insert(0) += 1;
            }

            let entries = counts_by_key
                .into_iter()
                .map(
                    |(key, count)| get_documents_split_count_response_v0::SplitCountEntry {
                        key,
                        count,
                    },
                )
                .collect();

            GetDocumentsSplitCountResponseV0 {
                result: Some(get_documents_split_count_response_v0::Result::SplitCounts(
                    get_documents_split_count_response_v0::SplitCounts { entries },
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
