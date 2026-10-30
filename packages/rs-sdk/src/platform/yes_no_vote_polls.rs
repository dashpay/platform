//! Yes/no vote polls: a poll's status and tallies (`getYesNoVotePollState`) and a masternode's
//! answer on a poll (`getMasternodeYesNoVote`), both read by the poll's id. Nodes serve them
//! from protocol version 15, and the SDK sends neither before the network has proven 15.
//!
//! A query names the poll by its id, or by the whole [`YesNoVotePoll`] (see
//! [`QueriedYesNoVotePoll`]): the polls the end date listing (`getVotePollsByEndDate`)
//! returns and the poll a [`YesNoVote`](dpp::voting::votes::yes_no_vote::YesNoVote) carries
//! can be passed as they are.
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

/// The yes/no vote poll a query names: its id, or the poll itself.
///
/// A poll's id is derived with the protocol version ([`YesNoVotePoll::unique_id`]), and no
/// version before 15 derives one. For a poll, the SDK derives the id each time it builds the
/// request, with the version it speaks then. An SDK at 14 or earlier first builds the request
/// only to learn the version it needs, so it can ask the network whether it runs 15; that
/// request carries the id the newest version this build knows derives. A network SDK sends a
/// request needing 15 only once the network has proven 15, and builds it again first, with
/// the version it then speaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueriedYesNoVotePoll {
    /// The poll's id, as [`YesNoVotePoll::unique_id`] derives it.
    Id(Identifier),
    /// The poll, whose id the SDK derives when it builds the request.
    Poll(YesNoVotePoll),
}

impl QueriedYesNoVotePoll {
    /// The id the request names the poll by, under `settings`.
    fn vote_poll_id(&self, settings: &QuerySettings<'_>) -> Result<Identifier, Error> {
        match self {
            QueriedYesNoVotePoll::Id(vote_poll_id) => Ok(*vote_poll_id),
            QueriedYesNoVotePoll::Poll(vote_poll) => {
                let derives_poll_ids = settings
                    .protocol_version
                    .dpp
                    .voting_versions
                    .yes_no_vote_poll_id
                    .is_some();
                let platform_version = if derives_poll_ids {
                    settings.protocol_version
                } else {
                    PlatformVersion::latest()
                };
                Ok(vote_poll.unique_id(platform_version)?)
            }
        }
    }
}

impl From<Identifier> for QueriedYesNoVotePoll {
    fn from(vote_poll_id: Identifier) -> Self {
        QueriedYesNoVotePoll::Id(vote_poll_id)
    }
}

impl From<YesNoVotePoll> for QueriedYesNoVotePoll {
    fn from(vote_poll: YesNoVotePoll) -> Self {
        QueriedYesNoVotePoll::Poll(vote_poll)
    }
}

/// Query for the state of a yes/no vote poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YesNoVotePollStateQuery {
    /// The poll.
    pub vote_poll: QueriedYesNoVotePoll,
}

impl YesNoVotePollStateQuery {
    /// The query for the state of `vote_poll`, named by its id or given whole.
    pub fn new(vote_poll: impl Into<QueriedYesNoVotePoll>) -> Self {
        YesNoVotePollStateQuery {
            vote_poll: vote_poll.into(),
        }
    }
}

impl Query<GetYesNoVotePollStateRequest> for YesNoVotePollStateQuery {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetYesNoVotePollStateRequest, Error> {
        Ok(GetYesNoVotePollStateRequest {
            version: Some(get_yes_no_vote_poll_state_request::Version::V0(
                GetYesNoVotePollStateRequestV0 {
                    vote_poll_id: self.vote_poll.vote_poll_id(settings)?.to_vec(),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Query<GetYesNoVotePollStateRequest> for Identifier {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetYesNoVotePollStateRequest, Error> {
        YesNoVotePollStateQuery::new(*self).query(settings)
    }
}

impl Query<GetYesNoVotePollStateRequest> for YesNoVotePoll {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetYesNoVotePollStateRequest, Error> {
        YesNoVotePollStateQuery::new(self.clone()).query(settings)
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasternodeYesNoVoteQuery {
    /// The masternode's pro tx hash.
    pub pro_tx_hash: Identifier,
    /// The poll.
    pub vote_poll: QueriedYesNoVotePoll,
}

impl MasternodeYesNoVoteQuery {
    /// The query for the masternode's answer on `vote_poll`, named by its id or given whole.
    pub fn new(pro_tx_hash: Identifier, vote_poll: impl Into<QueriedYesNoVotePoll>) -> Self {
        MasternodeYesNoVoteQuery {
            pro_tx_hash,
            vote_poll: vote_poll.into(),
        }
    }
}

impl Query<GetMasternodeYesNoVoteRequest> for MasternodeYesNoVoteQuery {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetMasternodeYesNoVoteRequest, Error> {
        Ok(GetMasternodeYesNoVoteRequest {
            version: Some(get_masternode_yes_no_vote_request::Version::V0(
                GetMasternodeYesNoVoteRequestV0 {
                    pro_tx_hash: self.pro_tx_hash.to_vec(),
                    vote_poll_id: self.vote_poll.vote_poll_id(settings)?.to_vec(),
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
    use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
    use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::v0::YesNoVotePollStoredInfoV0;
    use dpp::voting::vote_polls::yes_no_vote_poll::{
        YesNoMinimumVotingPower, YesNoVotePollPurpose,
    };

    fn mock_sdk() -> Sdk {
        SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("expected a mock sdk")
    }

    fn poll() -> YesNoVotePoll {
        YesNoVotePoll {
            purpose: YesNoVotePollPurpose::DecisionProposal {
                document_id: Identifier::new([0xc1; 32]),
            },
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

    /// No version before 15 derives a poll id, so an SDK at 14 builds the request (which only
    /// tells it the version to ask the network for) with the id the newest version derives.
    #[test]
    fn should_build_a_poll_request_at_protocol_version_14_with_the_newest_versions_id() {
        let protocol_14 = PlatformVersion::get(14).expect("protocol 14");
        assert!(poll().unique_id(protocol_14).is_err());
        let newest_id = poll()
            .unique_id(PlatformVersion::latest())
            .expect("poll id");
        let sdk = mock_sdk();
        let settings = QuerySettings {
            protocol_version: protocol_14,
            ..sdk.query_settings()
        };

        let state: GetYesNoVotePollStateRequest = poll().query(&settings).expect("request");
        let Some(get_yes_no_vote_poll_state_request::Version::V0(state)) = state.version else {
            panic!("expected a version 0 request");
        };
        assert_eq!(state.vote_poll_id, newest_id.to_vec());

        let pro_tx_hash = Identifier::new([1; 32]);
        let vote: GetMasternodeYesNoVoteRequest =
            MasternodeYesNoVoteQuery::new(pro_tx_hash, poll())
                .query(&settings)
                .expect("request");
        let by_id: GetMasternodeYesNoVoteRequest =
            MasternodeYesNoVoteQuery::new(pro_tx_hash, newest_id)
                .query(&settings)
                .expect("request");
        assert_eq!(vote, by_id);
    }

    #[tokio::test]
    async fn should_fetch_a_poll_state_and_a_masternode_answer_from_mock_expectations() {
        let mut sdk = mock_sdk();
        let state = YesNoVotePollState {
            stored_info: Some(
                YesNoVotePollStoredInfoV0::new(
                    poll(),
                    BlockInfo {
                        time_ms: 1_000,
                        height: 7,
                        core_height: 42,
                        epoch: Default::default(),
                    },
                )
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
        let masternode_query = MasternodeYesNoVoteQuery::new(Identifier::new([3; 32]), poll());
        sdk.mock()
            .expect_fetch(poll(), Some(state.clone()))
            .await
            .expect("expected to register the state")
            .expect_fetch(masternode_query.clone(), Some(vote))
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
