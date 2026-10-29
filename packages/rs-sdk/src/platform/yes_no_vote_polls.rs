//! Yes/no vote polls: a poll's status and tallies (`getYesNoVotePollState`) and a masternode's
//! answer on a poll (`getMasternodeYesNoVote`), both read by the poll's id. Nodes serve them
//! from protocol version 15, and the SDK sends neither before the network has proven 15.
//!
//! A query takes the poll's id, or the whole [`YesNoVotePoll`], whose id the SDK derives with
//! its protocol version ([`YesNoVotePoll::unique_id`]): the polls the end date listing
//! (`getVotePollsByEndDate`) returns and the poll a [`YesNoVote`](dpp::voting::votes::yes_no_vote::YesNoVote)
//! carries can be passed as they are.
//!
//! [`YesNoVotePollState::fetch`] answers `None` for a poll never opened. While the poll is open
//! its stored info is started and the tallies are the voting power cast so far; once it
//! finished, the stored info records the result and the tallies are zero, as the votes are
//! removed. [`MasternodeYesNoVote::fetch`] answers `None` when the masternode has not voted on
//! the poll, and once the poll finished.
//!
//! Both types also implement [`FetchUnproved`] for the unverified fast path.

use crate::platform::{Fetch, FetchUnproved, Identifier, Query, QuerySettings};
use crate::Error;
use dapi_grpc::platform::v0::get_masternode_yes_no_vote_request::GetMasternodeYesNoVoteRequestV0;
use dapi_grpc::platform::v0::get_yes_no_vote_poll_state_request::GetYesNoVotePollStateRequestV0;
use dapi_grpc::platform::v0::{
    get_masternode_yes_no_vote_request, get_yes_no_vote_poll_state_request,
    GetMasternodeYesNoVoteRequest, GetYesNoVotePollStateRequest,
};
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
pub use drive_proof_verifier::types::yes_no_vote_polls::{MasternodeYesNoVote, YesNoVotePollState};

/// Query for the state of a yes/no vote poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YesNoVotePollStateQuery {
    /// The poll's id, as [`YesNoVotePoll::unique_id`] derives it.
    pub vote_poll_id: Identifier,
}

impl YesNoVotePollStateQuery {
    /// The query for the state of `vote_poll`, its id derived as `platform_version` selects.
    pub fn for_vote_poll(
        vote_poll: &YesNoVotePoll,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        Ok(YesNoVotePollStateQuery {
            vote_poll_id: vote_poll.unique_id(platform_version)?,
        })
    }
}

impl Query<GetYesNoVotePollStateRequest> for YesNoVotePollStateQuery {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetYesNoVotePollStateRequest, Error> {
        Ok(GetYesNoVotePollStateRequest {
            version: Some(get_yes_no_vote_poll_state_request::Version::V0(
                GetYesNoVotePollStateRequestV0 {
                    vote_poll_id: self.vote_poll_id.to_vec(),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Query<GetYesNoVotePollStateRequest> for Identifier {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetYesNoVotePollStateRequest, Error> {
        YesNoVotePollStateQuery {
            vote_poll_id: *self,
        }
        .query(settings)
    }
}

impl Query<GetYesNoVotePollStateRequest> for YesNoVotePoll {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetYesNoVotePollStateRequest, Error> {
        YesNoVotePollStateQuery::for_vote_poll(self, settings.protocol_version)?.query(settings)
    }
}

impl Fetch for YesNoVotePollState {
    type Query = GetYesNoVotePollStateRequest;
    type Request = GetYesNoVotePollStateRequest;
}

impl FetchUnproved for YesNoVotePollState {
    type Request = GetYesNoVotePollStateRequest;
}

/// Query for a masternode's answer on a yes/no vote poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasternodeYesNoVoteQuery {
    /// The masternode's pro tx hash.
    pub pro_tx_hash: Identifier,
    /// The poll's id, as [`YesNoVotePoll::unique_id`] derives it.
    pub vote_poll_id: Identifier,
}

impl MasternodeYesNoVoteQuery {
    /// The query for the masternode's answer on `vote_poll`, the poll's id derived as
    /// `platform_version` selects.
    pub fn for_vote_poll(
        pro_tx_hash: Identifier,
        vote_poll: &YesNoVotePoll,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        Ok(MasternodeYesNoVoteQuery {
            pro_tx_hash,
            vote_poll_id: vote_poll.unique_id(platform_version)?,
        })
    }
}

impl Query<GetMasternodeYesNoVoteRequest> for MasternodeYesNoVoteQuery {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetMasternodeYesNoVoteRequest, Error> {
        Ok(GetMasternodeYesNoVoteRequest {
            version: Some(get_masternode_yes_no_vote_request::Version::V0(
                GetMasternodeYesNoVoteRequestV0 {
                    pro_tx_hash: self.pro_tx_hash.to_vec(),
                    vote_poll_id: self.vote_poll_id.to_vec(),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Fetch for MasternodeYesNoVote {
    type Query = GetMasternodeYesNoVoteRequest;
    type Request = GetMasternodeYesNoVoteRequest;
}

impl FetchUnproved for MasternodeYesNoVote {
    type Request = GetMasternodeYesNoVoteRequest;
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::{Sdk, SdkBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::platform_value::BinaryData;
    use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
    use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::v0::YesNoVotePollStoredInfoV0;
    use dpp::voting::vote_polls::yes_no_vote_poll::YesNoMinimumVotingPower;

    fn mock_sdk() -> Sdk {
        SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("expected a mock sdk")
    }

    fn poll() -> YesNoVotePoll {
        YesNoVotePoll {
            resource_path: vec![BinaryData::new(vec![0xc1; 32])],
            supermajority_numerator: 2,
            supermajority_denominator: 3,
            minimum_voting_power: YesNoMinimumVotingPower::Absolute { voting_power: 4 },
        }
    }

    #[test]
    fn should_name_the_poll_by_the_id_the_platform_version_derives() {
        let sdk = mock_sdk();
        let settings = sdk.query_settings();
        let vote_poll_id = poll()
            .unique_id(PlatformVersion::latest())
            .expect("poll id");
        let by_poll: GetYesNoVotePollStateRequest = poll().query(&settings).expect("request");
        let by_id: GetYesNoVotePollStateRequest = vote_poll_id.query(&settings).expect("request");
        assert_eq!(by_poll, by_id);
        let Some(get_yes_no_vote_poll_state_request::Version::V0(v0)) = by_poll.version else {
            panic!("expected a version 0 request");
        };
        assert_eq!(v0.vote_poll_id, vote_poll_id.to_vec());
        assert_eq!(v0.prove, settings.prove);
    }

    #[test]
    fn should_refuse_to_derive_a_poll_id_before_protocol_version_15() {
        let protocol_14 = PlatformVersion::get(14).expect("protocol 14");
        assert!(YesNoVotePollStateQuery::for_vote_poll(&poll(), protocol_14).is_err());
        assert!(MasternodeYesNoVoteQuery::for_vote_poll(
            Identifier::new([1; 32]),
            &poll(),
            protocol_14
        )
        .is_err());
    }

    #[tokio::test]
    async fn should_fetch_a_poll_state_and_a_masternode_answer_from_mock_expectations() {
        let mut sdk = mock_sdk();
        let state = YesNoVotePollState {
            stored_info: Some(
                YesNoVotePollStoredInfoV0::new(BlockInfo {
                    time_ms: 1_000,
                    height: 7,
                    core_height: 42,
                    epoch: Default::default(),
                })
                .into(),
            ),
            yes_voting_power: 5,
            no_voting_power: 1,
            abstain_voting_power: 4,
        };
        let vote = MasternodeYesNoVote {
            vote_choice: YesNoAbstainVoteChoice::No,
            times_voted: 2,
        };
        let masternode_query = MasternodeYesNoVoteQuery::for_vote_poll(
            Identifier::new([3; 32]),
            &poll(),
            PlatformVersion::latest(),
        )
        .expect("query");
        sdk.mock()
            .expect_fetch(poll(), Some(state.clone()))
            .await
            .expect("expected to register the state")
            .expect_fetch(masternode_query, Some(vote))
            .await
            .expect("expected to register the vote");

        assert_eq!(
            YesNoVotePollState::fetch(&sdk, poll())
                .await
                .expect("expected the state"),
            Some(state)
        );
        assert_eq!(
            MasternodeYesNoVote::fetch(&sdk, masternode_query)
                .await
                .expect("expected the vote"),
            Some(vote)
        );
    }
}
