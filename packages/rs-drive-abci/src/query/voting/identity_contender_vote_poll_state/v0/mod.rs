use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_contender_vote_poll_state_request::GetIdentityContenderVotePollStateRequestV0;
use dapi_grpc::platform::v0::get_identity_contender_vote_poll_state_response::get_identity_contender_vote_poll_state_response_v0::{
    Contender, FinishedVoteInfo, IdentityContenderVotePollState, PollInfo, Result as ResultV0, Status,
};
use dapi_grpc::platform::v0::get_identity_contender_vote_poll_state_response::GetIdentityContenderVotePollStateResponseV0;
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::{
    IdentityContenderVotePollStatus, IdentityContenderVotePollStoredInfoV0Getters,
};
use drive::query::identity_contender_vote_poll_state_query::IdentityContenderVotePollStateQuery;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_identity_contender_vote_poll_state_v0(
        &self,
        GetIdentityContenderVotePollStateRequestV0 {
            vote_poll_id,
            start_at_identifier_info,
            count,
            prove,
        }: GetIdentityContenderVotePollStateRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityContenderVotePollStateResponseV0>, Error> {
        // The limit the proof verifier assumes when the request names none, so proofs verify
        // whatever this node's default query limit is
        let max_limit = platform_version.drive_abci.query.max_returned_elements;
        let vote_poll_id: Identifier =
            check_validation_result_with_data!(vote_poll_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "vote_poll_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));
        let limit = check_validation_result_with_data!(count.map_or(Ok(max_limit), |limit| {
            let limit = u16::try_from(limit)
                .map_err(|_| QueryError::InvalidArgument("limit out of bounds".to_string()))?;
            if limit == 0 || limit > max_limit {
                Err(QueryError::InvalidArgument(format!(
                    "limit {} out of bounds of [1, {}]",
                    limit, max_limit
                )))
            } else {
                Ok(limit)
            }
        }));
        let start_at = check_validation_result_with_data!(start_at_identifier_info
            .map(|start_at_identifier_info| {
                Identifier::from_vec(start_at_identifier_info.start_identifier)
                    .map(|identifier| {
                        (
                            identifier.to_buffer(),
                            start_at_identifier_info.start_identifier_included,
                        )
                    })
                    .map_err(|_| {
                        QueryError::InvalidArgument(
                            "start_identifier must be a valid identifier (32 bytes long)"
                                .to_string(),
                        )
                    })
            })
            .transpose());

        let query = IdentityContenderVotePollStateQuery {
            vote_poll_id,
            limit: Some(limit),
            start_at,
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
            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;
            GetIdentityContenderVotePollStateResponseV0 {
                result: Some(ResultV0::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let state =
                match query.execute_no_proof(&self.drive, None, &mut vec![], platform_version) {
                    Ok(state) => state,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };
            let info = state.stored_info.map(|stored_info| PollInfo {
                status: match stored_info.status() {
                    IdentityContenderVotePollStatus::Joining => Status::Joining as i32,
                    IdentityContenderVotePollStatus::Voting => Status::Voting as i32,
                    IdentityContenderVotePollStatus::Resolved => Status::Resolved as i32,
                },
                join_end_time_ms: stored_info.join_end_time_ms(),
                vote_end_time_ms: stored_info.vote_end_time_ms(),
                finished_vote_info: stored_info.result().map(|result| FinishedVoteInfo {
                    winner_id: result.winner.map(|winner| winner.to_vec()),
                    vote_phase_held: result.vote_phase_held,
                    finished_at_block_height: result.finalization_block.height,
                    finished_at_core_block_height: result.finalization_block.core_height,
                    finished_at_block_time_ms: result.finalization_block.time_ms,
                    finished_at_epoch: result.finalization_block.epoch.index as u32,
                }),
            });
            GetIdentityContenderVotePollStateResponseV0 {
                result: Some(ResultV0::State(IdentityContenderVotePollState {
                    info,
                    contenders: state
                        .contenders
                        .into_iter()
                        .map(|contender| Contender {
                            identity_id: contender.identity_id.to_vec(),
                            vote_tally: contender.vote_tally,
                            joined_at_block_height: contender.info.joined_at().height,
                            joined_at_block_time_ms: contender.info.joined_at().time_ms,
                            reference_id: contender.info.reference_id().to_vec(),
                        })
                        .collect(),
                    abstain_vote_tally: state.abstain_vote_tally,
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };
        Ok(QueryValidationResult::new_with_data(response))
    }
}
