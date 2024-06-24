use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contested_resource_voters_for_identity_request::GetContestedResourceVotersForIdentityRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_voters_for_identity_response::{
    get_contested_resource_voters_for_identity_response_v0,
    GetContestedResourceVotersForIdentityResponseV0,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::{check_validation_result_with_data, platform_value};
use drive::error::query::QuerySyntaxError;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;

impl<C> Platform<C> {
    pub(super) fn query_contested_resource_voters_for_identity_v0(
        &self,
        GetContestedResourceVotersForIdentityRequestV0 {
            contract_id,
            document_type_name,
            index_name,
            index_values,
            contestant_id,
            start_at_identifier_info,
            count,
            order_ascending,
            prove,
        }: GetContestedResourceVotersForIdentityRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContestedResourceVotersForIdentityResponseV0>, Error> {
        let config = &self.config.drive;
        let contract_id: Identifier =
            check_validation_result_with_data!(contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "contract_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let contestant_id: Identifier =
            check_validation_result_with_data!(contestant_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "voter_id must be a valid identifier (32 bytes long)".to_string(),
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

        let index = check_validation_result_with_data!(document_type.find_contested_index().ok_or(
            QueryError::InvalidArgument(format!(
                "document type {} does not have a contested index",
                document_type_name
            ))
        ));

        if &index.name != &index_name {
            return Ok(QueryValidationResult::new_with_error(QueryError::InvalidArgument(format!(
                "index with name {} is not the contested index on the document type {}, {} is the name of the only contested index",
                index_name, document_type_name, index.name
            ))));
        }

        let index_values = match index_values
            .into_iter()
            .enumerate()
            .map(|(pos, serialized_value)| {
                Ok(bincode::decode_from_slice(
                    serialized_value.as_slice(),
                    bincode::config::standard()
                        .with_big_endian()
                        .with_no_limit(),
                )
                .map_err(|_| {
                    QueryError::InvalidArgument(format!(
                        "could not convert {:?} to a value in the index values at position {}",
                        serialized_value, pos
                    ))
                })?
                .0)
            })
            .collect::<Result<Vec<_>, QueryError>>()
        {
            Ok(index_values) => index_values,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name,
            index_name,
            index_values,
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

        let response = if prove {
            let query = ContestedDocumentVotePollVotesDriveQuery {
                vote_poll,
                contestant_id,
                offset: None,
                limit: Some(limit),
                start_at: start_at_identifier_info
                    .map(|start_at_identifier_info| {
                        Ok::<([u8; 32], bool), platform_value::Error>((
                            Identifier::from_vec(start_at_identifier_info.start_identifier)?
                                .to_buffer(),
                            start_at_identifier_info.start_identifier_included,
                        ))
                    })
                    .transpose()?,
                order_ascending,
            };

            let proof = match query.execute_with_proof(&self.drive, None, None, platform_version) {
                Ok(result) => result.0,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(e) => return Err(e.into()),
            };

            GetContestedResourceVotersForIdentityResponseV0 {
                result: Some(
                    get_contested_resource_voters_for_identity_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof),
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let query = ContestedDocumentVotePollVotesDriveQuery {
                vote_poll: vote_poll.clone(),
                contestant_id,
                offset: None,
                limit: Some(limit),
                start_at: start_at_identifier_info
                    .clone()
                    .map(|start_at_identifier_info| {
                        Ok::<([u8; 32], bool), platform_value::Error>((
                            Identifier::from_vec(start_at_identifier_info.start_identifier)?
                                .to_buffer(),
                            start_at_identifier_info.start_identifier_included,
                        ))
                    })
                    .transpose()?,
                order_ascending,
            };

            let voters =
                match query.execute_no_proof(&self.drive, None, &mut vec![], platform_version) {
                    Ok(result) => result,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };

            let finished_results = if voters.len() == limit as usize && limit > 0 {
                let extra_query = ContestedDocumentVotePollVotesDriveQuery {
                    vote_poll,
                    contestant_id,
                    offset: None,
                    limit: Some(1),
                    start_at: Some((voters.last().unwrap().to_buffer(), false)),
                    order_ascending,
                };
                let another_result = match extra_query.execute_no_proof(
                    &self.drive,
                    None,
                    &mut vec![],
                    platform_version,
                ) {
                    Ok(result) => result,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };
                another_result.is_empty()
            } else {
                true
            };

            let voters = voters.into_iter().map(|voter| voter.to_vec()).collect();

            GetContestedResourceVotersForIdentityResponseV0 {
                result: Some(
                    get_contested_resource_voters_for_identity_response_v0::Result::ContestedResourceVoters(
                        get_contested_resource_voters_for_identity_response_v0::ContestedResourceVoters {
                            voters,
                            finished_results,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
