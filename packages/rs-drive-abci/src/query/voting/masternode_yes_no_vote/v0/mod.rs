use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_masternode_yes_no_vote_request::GetMasternodeYesNoVoteRequestV0;
use dapi_grpc::platform::v0::get_masternode_yes_no_vote_response::get_masternode_yes_no_vote_response_v0::{
    self, masternode_yes_no_vote, MasternodeYesNoVote as MasternodeYesNoVoteProto,
    MasternodeYesNoVoteEntry,
};
use dapi_grpc::platform::v0::get_masternode_yes_no_vote_response::GetMasternodeYesNoVoteResponseV0;
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use drive::query::masternode_yes_no_vote_query::MasternodeYesNoVoteDriveQuery;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_masternode_yes_no_vote_v0(
        &self,
        GetMasternodeYesNoVoteRequestV0 {
            pro_tx_hash,
            vote_poll_id,
            prove,
        }: GetMasternodeYesNoVoteRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetMasternodeYesNoVoteResponseV0>, Error> {
        let pro_tx_hash: Identifier =
            check_validation_result_with_data!(pro_tx_hash.try_into().map_err(|_| {
                QueryError::InvalidArgument("pro_tx_hash must be 32 bytes long".to_string())
            }));
        let vote_poll_id: Identifier =
            check_validation_result_with_data!(vote_poll_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "vote_poll_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));
        let query = MasternodeYesNoVoteDriveQuery {
            pro_tx_hash,
            vote_poll_id,
        };

        // Before protocol version 15 the Drive reads are not active, which every node at this
        // height shares, so they are refused as a query error rather than an internal one.
        let result = if prove {
            let proof = check_validation_result_with_data!(query.execute_with_proof(
                &self.drive,
                None,
                &mut vec![],
                platform_version,
            ));
            get_masternode_yes_no_vote_response_v0::Result::Proof(
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                    .map(|(_, proof)| proof)?,
            )
        } else {
            let vote = check_validation_result_with_data!(query.execute_no_proof(
                &self.drive,
                None,
                &mut vec![],
                platform_version,
            ));
            get_masternode_yes_no_vote_response_v0::Result::Entry(MasternodeYesNoVoteEntry {
                vote: vote.map(|vote| MasternodeYesNoVoteProto {
                    choice: choice_to_proto(vote.vote_choice) as i32,
                    times_voted: vote.times_voted as u32,
                }),
            })
        };

        Ok(QueryValidationResult::new_with_data(
            GetMasternodeYesNoVoteResponseV0 {
                result: Some(result),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
        ))
    }
}

fn choice_to_proto(vote_choice: YesNoAbstainVoteChoice) -> masternode_yes_no_vote::Choice {
    match vote_choice {
        YesNoAbstainVoteChoice::Yes => masternode_yes_no_vote::Choice::Yes,
        YesNoAbstainVoteChoice::No => masternode_yes_no_vote::Choice::No,
        YesNoAbstainVoteChoice::Abstain => masternode_yes_no_vote::Choice::Abstain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;
    use drive::error::drive::DriveError as DriveInternalError;
    use drive::error::Error as DriveError;

    fn request(
        pro_tx_hash: Vec<u8>,
        vote_poll_id: Vec<u8>,
        prove: bool,
    ) -> GetMasternodeYesNoVoteRequestV0 {
        GetMasternodeYesNoVoteRequestV0 {
            pro_tx_hash,
            vote_poll_id,
            prove,
        }
    }

    #[test]
    fn should_refuse_ids_that_are_not_32_bytes() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        for (pro_tx_hash, vote_poll_id, field) in [
            (vec![1; 8], vec![1; 32], "pro_tx_hash"),
            (vec![1; 32], vec![1; 31], "vote_poll_id"),
        ] {
            let result = platform
                .query_masternode_yes_no_vote_v0(
                    request(pro_tx_hash, vote_poll_id, false),
                    &state,
                    version,
                )
                .expect("expected the query to run");
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [QueryError::InvalidArgument(message)] if message.contains(field)
                ),
                "{field}: {:?}",
                result.errors
            );
        }
    }

    #[test]
    fn should_refuse_the_query_before_protocol_version_15() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, Some(14));
        for prove in [false, true] {
            let result = platform
                .query_masternode_yes_no_vote_v0(
                    request(vec![1; 32], vec![2; 32], prove),
                    &state,
                    version,
                )
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
    fn should_answer_no_vote_for_a_masternode_that_never_voted() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let response = platform
            .query_masternode_yes_no_vote_v0(
                request(vec![1; 32], vec![2; 32], false),
                &state,
                version,
            )
            .expect("expected the query to run")
            .into_data()
            .expect("expected an entry");
        assert_eq!(
            response.result,
            Some(get_masternode_yes_no_vote_response_v0::Result::Entry(
                MasternodeYesNoVoteEntry { vote: None }
            ))
        );
    }
}
