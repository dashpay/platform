use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use bincode::error::EncodeError;
use dapi_grpc::platform::v0::get_contested_resources_request::GetContestedResourcesRequestV0;
use dapi_grpc::platform::v0::get_contested_resources_response::get_contested_resources_response_v0;
use dapi_grpc::platform::v0::get_contested_resources_response::GetContestedResourcesResponseV0;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};
use drive::error::query::QuerySyntaxError;
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;

impl<C> Platform<C> {
    pub(super) fn query_contested_resources_v0(
        &self,
        GetContestedResourcesRequestV0 {
            contract_id,
            document_type_name,
            index_name,
            start_index_values,
            end_index_values,
            start_at_value_info,
            count,
            order_ascending,
            prove,
        }: GetContestedResourcesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContestedResourcesResponseV0>, Error> {
        let config = &self.config.drive;
        let contract_id: Identifier =
            check_validation_result_with_data!(contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "contract_id must be a valid identifier (32 bytes long) (contested resources query)".to_string(),
                )
            }));

        let (_, contract) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;

        let contract = check_validation_result_with_data!(contract.ok_or(QueryError::Query(
            QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info (contested resources query)",
            )
        )));

        let contract_ref = &contract.contract;

        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {} (contested resources query)",
                document_type_name, contract_id
            ))));

        let index = check_validation_result_with_data!(document_type.find_contested_index().ok_or(
            QueryError::InvalidArgument(format!(
                "document type {} does not have a contested index (contested resources query)",
                document_type_name
            ))
        ));

        if index.name != index_name {
            return Ok(QueryValidationResult::new_with_error(QueryError::InvalidArgument(format!(
                "index with name {} is not the contested index on the document type {}, {} is the name of the only contested index (contested resources query)",
                index_name, document_type_name, index.name
            ))));
        }

        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let start_index_values = match start_index_values
            .into_iter()
            .enumerate()
            .map(|(pos, serialized_value)| {
                Ok(bincode::decode_from_slice(
                    serialized_value.as_slice(),
                    bincode_config,
                )
                    .map_err(|_| {
                        QueryError::InvalidArgument(format!(
                            "could not convert {:?} to a value in the start index values at position {}",
                            serialized_value, pos
                        ))
                    })?
                    .0)
            })
            .collect::<Result<Vec<Value>, QueryError>>()
        {
            Ok(index_values) => index_values,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        let end_index_values = match end_index_values
            .into_iter()
            .enumerate()
            .map(|(pos, serialized_value)| {
                Ok(bincode::decode_from_slice(
                    serialized_value.as_slice(),
                    bincode_config,
                )
                    .map_err(|_| {
                        QueryError::InvalidArgument(format!(
                            "could not convert {:?} to a value in the end index values at position {}",
                            serialized_value, pos + start_index_values.len() + 1
                        ))
                    })?
                    .0)
            })
            .collect::<Result<Vec<Value>, QueryError>>()
        {
            Ok(index_values) => index_values,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        let limit = check_validation_result_with_data!(count.map_or(
            Ok(config.default_query_limit),
            |limit| {
                let limit = u16::try_from(limit)
                    .map_err(|_| QueryError::InvalidArgument("limit out of bounds".to_string()))?;
                if limit == 0 || limit > config.default_query_limit {
                    Err(QueryError::InvalidArgument(format!(
                        "limit {} out of bounds of [1, {}]",
                        limit, config.default_query_limit
                    )))
                } else {
                    Ok(limit)
                }
            }
        ));

        let result = start_at_value_info
            .map(|start_at_value_info| {
                let start = bincode::decode_from_slice(
                    start_at_value_info.start_value.as_slice(),
                    bincode_config,
                )
                .map_err(|_| {
                    QueryError::InvalidArgument(format!(
                        "could not convert {:?} to a value for start at",
                        start_at_value_info.start_value
                    ))
                })?
                .0;

                Ok::<(dpp::platform_value::Value, bool), QueryError>((
                    start,
                    start_at_value_info.start_value_included,
                ))
            })
            .transpose();

        let start_at_value_info = match result {
            Ok(start_at_value_info) => start_at_value_info,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        let query = VotePollsByDocumentTypeQuery {
            contract_id,
            document_type_name,
            index_name,
            start_index_values,
            end_index_values,
            start_at_value: start_at_value_info,
            limit: Some(limit),
            order_ascending,
        };

        let response = if prove {
            let proof =
                match query.execute_with_proof(&self.drive, None, &mut vec![], platform_version) {
                    Ok(result) => result,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };

            GetContestedResourcesResponseV0 {
                result: Some(get_contested_resources_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let results =
                match query.execute_no_proof(&self.drive, None, &mut vec![], platform_version) {
                    Ok(result) => result,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };

            let contested_resource_values = results
                .into_iter()
                .map(|value| bincode::encode_to_vec(value, bincode_config))
                .collect::<Result<Vec<Vec<u8>>, EncodeError>>()
                .map_err(|e| {
                    Error::Protocol(ProtocolError::CorruptedSerialization(e.to_string()))
                })?;

            GetContestedResourcesResponseV0 {
                result: Some(
                    get_contested_resources_response_v0::Result::ContestedResourceValues(
                        get_contested_resources_response_v0::ContestedResourceValues {
                            contested_resource_values,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
