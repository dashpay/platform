use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::{
    get_contested_resource_vote_state_response_v0, GetContestedResourceVoteStateResponseV0,
};
use dapi_grpc::platform::v0::get_contested_vote_polls_by_end_date_request::GetContestedVotePollsByEndDateRequestV0;
use dapi_grpc::platform::v0::get_contested_vote_polls_by_end_date_response::{get_contested_vote_polls_by_end_date_response_v0, GetContestedVotePollsByEndDateResponseV0};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::version::PlatformVersion;

use drive::error::query::QuerySyntaxError;
use drive::query::vote_poll_vote_state_query::{ContenderWithSerializedDocument, ContestedDocumentVotePollDriveQuery};
use drive::query::VotePollsByEndDateDriveQuery;

impl<C> Platform<C> {
    pub(super) fn query_contested_vote_polls_by_end_date_query_v0(
        &self,
        GetContestedVotePollsByEndDateRequestV0 {
            start_time_info, end_time_info, limit, ascending, prove
        }: GetContestedVotePollsByEndDateRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContestedVotePollsByEndDateResponseV0>, Error> {
        let config = &self.config.drive;
        
        let start_time = start_time_info.map(|start_time_info| (start_time_info.start_time_ms, start_time_info.start_time_included));

        let end_time = end_time_info.map(|end_time_info| (end_time_info.end_time_ms, end_time_info.end_time_included));
        
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

        let query = VotePollsByEndDateDriveQuery {
            start_time,
            limit: Some(limit),
            order_ascending: ascending,
            end_time,
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

            GetContestedVotePollsByEndDateResponseV0 {
                result: Some(
                    get_contested_vote_polls_by_end_date_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof),
                    ),
                ),
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

            let contenders = results
                .contenders
                .into_iter()
                .map(
                    |ContenderWithSerializedDocument {
                         identity_id,
                         serialized_document,
                         vote_tally,
                     }| {
                        get_contested_resource_vote_state_response_v0::Contender {
                            identifier: identity_id.to_vec(),
                            vote_count: vote_tally,
                            document: serialized_document,
                        }
                    },
                )
                .collect();

            GetContestedVotePollsByEndDateResponseV0 {
                result: Some(
                    get_contested_vote_polls_by_end_date_response_v0::Result::ContestedVotePollsByTimestamps(
                        get_contested_vote_polls_by_end_date_response_v0::ContestedVotePollsByTimestamps {
                            contested_vote_polls_by_timestamps: vec![],
                            finished_results: false,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
