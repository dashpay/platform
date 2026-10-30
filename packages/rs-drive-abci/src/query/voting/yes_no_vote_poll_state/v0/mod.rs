use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_yes_no_vote_poll_state_request::GetYesNoVotePollStateRequestV0;
use dapi_grpc::platform::v0::get_yes_no_vote_poll_state_response::get_yes_no_vote_poll_state_response_v0::{
    self, yes_no_vote_poll_state, FinishedYesNoVotePoll, YesNoVotePollBlock,
    YesNoVotePollState as YesNoVotePollStateProto,
};
use dapi_grpc::platform::v0::get_yes_no_vote_poll_state_response::GetYesNoVotePollStateResponseV0;
use dpp::block::block_info::BlockInfo;
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStatus;
use drive::query::yes_no_vote_poll_state_query::{
    YesNoVotePollState, YesNoVotePollStateDriveQuery,
};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_yes_no_vote_poll_state_v0(
        &self,
        GetYesNoVotePollStateRequestV0 {
            vote_poll_id,
            prove,
        }: GetYesNoVotePollStateRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetYesNoVotePollStateResponseV0>, Error> {
        let vote_poll_id: Identifier =
            check_validation_result_with_data!(vote_poll_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "vote_poll_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));
        let query = YesNoVotePollStateDriveQuery { vote_poll_id };

        // Before protocol version 15 the Drive read is not active, which every node at this
        // height shares, so it is refused as a query error rather than an internal one.
        let result = if prove {
            let proof = check_validation_result_with_data!(query.execute_with_proof(
                &self.drive,
                None,
                &mut vec![],
                platform_version,
            ));
            get_yes_no_vote_poll_state_response_v0::Result::Proof(
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                    .map(|(_, proof)| proof)?,
            )
        } else {
            let state = check_validation_result_with_data!(query.execute_no_proof(
                &self.drive,
                None,
                &mut vec![],
                platform_version,
            ));
            get_yes_no_vote_poll_state_response_v0::Result::State(state_to_proto(state))
        };

        Ok(QueryValidationResult::new_with_data(
            GetYesNoVotePollStateResponseV0 {
                result: Some(result),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
        ))
    }
}

fn block_to_proto(block: &BlockInfo) -> YesNoVotePollBlock {
    YesNoVotePollBlock {
        height: block.height,
        core_height: block.core_height,
        time_ms: block.time_ms,
        epoch: block.epoch.index as u32,
    }
}

fn state_to_proto(state: YesNoVotePollState) -> YesNoVotePollStateProto {
    let status = state
        .stored_info
        .map(|stored_info| match stored_info.status() {
            YesNoVotePollStatus::Started(start_block) => {
                yes_no_vote_poll_state::Status::Started(block_to_proto(start_block))
            }
            YesNoVotePollStatus::Finished(result) => {
                yes_no_vote_poll_state::Status::Finished(FinishedYesNoVotePoll {
                    passed: result.passed,
                    yes_voting_power: result.yes_voting_power,
                    no_voting_power: result.no_voting_power,
                    abstain_voting_power: result.abstain_voting_power,
                    required_voting_power: result.required_voting_power,
                    start_block: Some(block_to_proto(&result.start_block)),
                    finalization_block: Some(block_to_proto(&result.finalization_block)),
                })
            }
        });
    YesNoVotePollStateProto {
        status,
        yes_voting_power: state.yes_voting_power,
        no_voting_power: state.no_voting_power,
        abstain_voting_power: state.abstain_voting_power,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;
    use drive::error::drive::DriveError as DriveInternalError;
    use drive::error::Error as DriveError;

    fn request(vote_poll_id: Vec<u8>, prove: bool) -> GetYesNoVotePollStateRequestV0 {
        GetYesNoVotePollStateRequestV0 {
            vote_poll_id,
            prove,
        }
    }

    #[test]
    fn should_refuse_a_poll_id_that_is_not_32_bytes() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let result = platform
            .query_yes_no_vote_poll_state_v0(request(vec![1; 8], false), &state, version)
            .expect("expected the query to run");
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(message)] if message.contains("vote_poll_id")
        ));
    }

    #[test]
    fn should_refuse_the_query_before_protocol_version_15() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, Some(14));
        for prove in [false, true] {
            let result = platform
                .query_yes_no_vote_poll_state_v0(request(vec![1; 32], prove), &state, version)
                .expect("expected the query to run");
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [QueryError::Drive(DriveError::Drive(
                        DriveInternalError::VersionNotActive { .. }
                    ))]
                ),
                "prove={prove}: {:?}",
                result.errors
            );
        }
    }

    #[test]
    fn should_answer_an_empty_state_for_a_poll_never_opened() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let response = platform
            .query_yes_no_vote_poll_state_v0(request(vec![1; 32], false), &state, version)
            .expect("expected the query to run")
            .into_data()
            .expect("expected a state");
        assert_eq!(
            response.result,
            Some(get_yes_no_vote_poll_state_response_v0::Result::State(
                YesNoVotePollStateProto::default()
            ))
        );
    }
}
