use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
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
use drive::util::grove_operations::GroveDBToUse;

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

        let start_index_values = match start_index_values
            .into_iter()
            .enumerate()
            .map(|(pos, serialized_value)| {
                super::super::decode_serialized_index_value(serialized_value.as_slice(), || {
                    format!(
                        "could not convert {:?} to a value in the start index values at position {}",
                        serialized_value, pos
                    )
                })
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
                super::super::decode_serialized_index_value(serialized_value.as_slice(), || {
                    format!(
                        "could not convert {:?} to a value in the end index values at position {}",
                        serialized_value,
                        pos + start_index_values.len() + 1
                    )
                })
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
                let start = super::super::decode_serialized_index_value(
                    start_at_value_info.start_value.as_slice(),
                    || {
                        format!(
                            "could not convert {:?} to a value for start at",
                            start_at_value_info.start_value
                        )
                    },
                )?;

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

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetContestedResourcesResponseV0 {
                result: Some(get_contested_resources_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
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

            let encode_config = bincode::config::standard()
                .with_big_endian()
                .with_no_limit();

            let contested_resource_values = results
                .into_iter()
                .map(|value| bincode::encode_to_vec(value, encode_config))
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
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_contested_resources_invalid_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourcesRequestV0 {
            contract_id: vec![0; 8],
            document_type_name: "x".to_string(),
            index_name: "x".to_string(),
            start_index_values: vec![],
            end_index_values: vec![],
            start_at_value_info: None,
            count: None,
            order_ascending: true,
            prove: false,
        };

        let result = platform
            .query_contested_resources_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("contract_id must be a valid identifier")
        ));
    }

    #[test]
    fn test_query_contested_resources_contract_not_found() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourcesRequestV0 {
            contract_id: vec![0; 32],
            document_type_name: "x".to_string(),
            index_name: "x".to_string(),
            start_index_values: vec![],
            end_index_values: vec![],
            start_at_value_info: None,
            count: None,
            order_ascending: true,
            prove: false,
        };

        let result = platform
            .query_contested_resources_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DataContractNotFound(_))]
        ));
    }

    #[test]
    fn test_query_contested_resources_contract_not_found_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourcesRequestV0 {
            contract_id: vec![0; 32],
            document_type_name: "x".to_string(),
            index_name: "x".to_string(),
            start_index_values: vec![],
            end_index_values: vec![],
            start_at_value_info: None,
            count: None,
            order_ascending: true,
            prove: true,
        };

        let result = platform
            .query_contested_resources_v0(request, &state, version)
            .expect("expected query to succeed");

        // Contract lookup runs before the prove/no-prove split.
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DataContractNotFound(_))]
        ));
    }

    #[test]
    fn test_query_contested_resources_short_contract_id_rejected() {
        // Verify the InvalidArgument message for a too-short contract_id
        // includes the "contested resources query" suffix specific to this
        // endpoint.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourcesRequestV0 {
            contract_id: vec![0; 31], // off-by-one short
            document_type_name: "x".to_string(),
            index_name: "x".to_string(),
            start_index_values: vec![],
            end_index_values: vec![],
            start_at_value_info: None,
            count: None,
            order_ascending: true,
            prove: false,
        };

        let result = platform
            .query_contested_resources_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg.contains("contract_id must be a valid identifier")
                    && msg.contains("contested resources query")
        ));
    }

    /// The exact ten-byte `Value::Bytes` payload from the advisory: enum
    /// discriminant 10, variable-integer marker `0xfd`, then a big-endian
    /// `u64::MAX` declared length with no body. Under an unbounded decoder this
    /// promoted to `vec![0u8; usize::MAX]` and panicked with `capacity
    /// overflow`, cancelling the shared query/consensus token.
    const CAPACITY_OVERFLOW_PAYLOAD: [u8; 10] =
        [0x0a, 0xfd, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];

    /// Build a request that passes the public contract/document-type/contested-index
    /// gates using the genesis DPNS `domain` / `parentNameAndLabel` index, so the
    /// malicious cursor bytes actually reach the decoder.
    fn dpns_contested_request() -> GetContestedResourcesRequestV0 {
        GetContestedResourcesRequestV0 {
            contract_id: dpp::system_data_contracts::dpns_contract::ID.to_vec(),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            start_index_values: vec![],
            end_index_values: vec![],
            start_at_value_info: None,
            count: None,
            order_ascending: true,
            prove: false,
        }
    }

    #[test]
    fn test_query_contested_resources_unbounded_start_index_value_rejected() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetContestedResourcesRequestV0 {
            start_index_values: vec![CAPACITY_OVERFLOW_PAYLOAD.to_vec()],
            ..dpns_contested_request()
        };

        // Must return a graceful InvalidArgument rather than panicking (which
        // would cancel the shared cancellation token and shut down the node).
        let result = platform
            .query_contested_resources_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("start index values")
        ));
    }

    #[test]
    fn test_query_contested_resources_unbounded_end_index_value_rejected() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetContestedResourcesRequestV0 {
            end_index_values: vec![CAPACITY_OVERFLOW_PAYLOAD.to_vec()],
            ..dpns_contested_request()
        };

        let result = platform
            .query_contested_resources_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("end index values")
        ));
    }

    #[test]
    fn test_query_contested_resources_unbounded_start_at_value_rejected() {
        use dapi_grpc::platform::v0::get_contested_resources_request::get_contested_resources_request_v0::StartAtValueInfo;

        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetContestedResourcesRequestV0 {
            start_at_value_info: Some(StartAtValueInfo {
                start_value: CAPACITY_OVERFLOW_PAYLOAD.to_vec(),
                start_value_included: true,
            }),
            ..dpns_contested_request()
        };

        let result = platform
            .query_contested_resources_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("start at")
        ));
    }
}
