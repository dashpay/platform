use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::dpp_voting_versions::DPPVotingVersions;

/// Voting versions 3. Introduced in protocol version 15 with the yes/no poll kind: its stored
/// info, id, parameter rules and decision rules, and the decision proposal that opens one, are
/// `Some(0)` (`None`, not active, in every earlier table). Everything else matches
/// `VOTING_VERSION_V2`.
pub const VOTING_VERSION_V3: DPPVotingVersions = DPPVotingVersions {
    yes_no_vote_poll_stored_info_version: Some(0),
    yes_no_vote_poll_id: Some(0),
    yes_no_vote_poll_parameters: Some(0),
    yes_no_vote_poll_decision: Some(0),
    decision_proposal_vote_poll: Some(0),
    ..VOTING_VERSION_V2
};
