use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::GetContestedResourceIdentityVotesRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_identity_votes_response::{
    get_contested_resource_identity_votes_response_v0, GetContestedResourceIdentityVotesResponseV0,
};
use dapi_grpc::platform::v0::get_contested_resource_voters_for_identity_response::{
    get_contested_resource_voters_for_identity_response_v0,
    GetContestedResourceVotersForIdentityResponseV0,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::{check_validation_result_with_data, platform_value, ProtocolError};
use drive::error::query::QuerySyntaxError;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;

impl<C> Platform<C> {
    pub(super) fn query_contested_resource_identity_votes_v0(
        &self,
        GetContestedResourceIdentityVotesRequestV0 {
            identity_id,
            limit,
            offset,
            order_ascending,
            start_at_vote_poll_id_info,
            prove,
        }: GetContestedResourceIdentityVotesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContestedResourceIdentityVotesResponseV0>, Error> {
        let config = &self.config.drive;
        let identity_id: Identifier =
            check_validation_result_with_data!(identity_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "identity_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let limit = limit
            .map_or(Some(config.default_query_limit), |limit_value| {
                if limit_value == 0
                    || limit_value > u16::MAX as u32
                    || limit_value as u16 > config.default_query_limit
                {
                    None
                } else {
                    Some(limit_value as u16)
                }
            })
            .ok_or(drive::error::Error::Query(QuerySyntaxError::InvalidLimit(
                format!("limit greater than max limit {}", config.max_query_limit),
            )))?;

        let offset = check_validation_result_with_data!(offset
            .map(|offset| {
                u16::try_from(offset)
                    .map_err(|_| QueryError::InvalidArgument("offset out of bounds".to_string()))
            })
            .transpose());

        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset,
            limit: Some(limit),
            start_at: start_at_vote_poll_id_info
                .map(|start_at_vote_poll_id_info| {
                    Ok::<([u8; 32], bool), platform_value::Error>((
                        Identifier::from_vec(start_at_vote_poll_id_info.start_at_poll_identifier)?
                            .to_buffer(),
                        start_at_vote_poll_id_info.start_poll_identifier_included,
                    ))
                })
                .transpose()?,
            order_ascending,
        };

        let response = if prove {
            let proof = match query.execute_with_proof(&self.drive, None, None, platform_version) {
                Ok(result) => result.0,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(e) => return Err(e.into()),
            };

            GetContestedResourceIdentityVotesResponseV0 {
                result: Some(
                    get_contested_resource_identity_votes_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof),
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let votes =
                match query.execute_no_proof(&self.drive, None, &mut vec![], platform_version) {
                    Ok(result) => result,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };

            let finished_results = if votes.len() == limit as usize && limit > 0 {
                let start_at = if order_ascending {
                    votes
                        .iter()
                        .next_back()
                        .map(|(id, _)| (id.to_buffer(), false))
                } else {
                    votes.iter().next().map(|(id, _)| (id.to_buffer(), false))
                };
                let extra_query = ContestedResourceVotesGivenByIdentityQuery {
                    identity_id,
                    offset: None,
                    limit: Some(1),
                    start_at,
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

            let votes = votes
                .into_values()
                .map(|resource_vote| {
                    Ok(get_contested_resource_identity_votes_response_v0::Vote {
                        contested_resource_serialized_vote: resource_vote
                            .serialize_consume_to_bytes()?,
                    })
                })
                .collect::<Result<
                    Vec<get_contested_resource_identity_votes_response_v0::Vote>,
                    ProtocolError,
                >>()?;

            GetContestedResourceIdentityVotesResponseV0 {
                result: Some(
                    get_contested_resource_identity_votes_response_v0::Result::Votes(
                        get_contested_resource_identity_votes_response_v0::Votes {
                            votes,
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
