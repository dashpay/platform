use versioned_feature_core::{FeatureVersion, OptionalFeatureVersion};

pub mod v1;
pub mod v2;
pub mod v3;

#[derive(Clone, Debug, Default)]
pub struct DPPVotingVersions {
    pub default_vote_poll_time_duration_mainnet_ms: u64,
    pub default_vote_poll_time_duration_test_network_ms: u64,
    pub contested_document_vote_poll_stored_info_version: FeatureVersion,
    /// The stored info of a yes/no poll. `None` (not active) before protocol version 15.
    pub yes_no_vote_poll_stored_info_version: OptionalFeatureVersion,
    /// How a yes/no poll's id is derived (`YesNoVotePoll::unique_id`). The id keys the poll's
    /// stored trees and its end date index entry, and names its prefunded balance. `None` (not
    /// active) before protocol version 15.
    pub yes_no_vote_poll_id: OptionalFeatureVersion,
    /// The rules a yes/no poll's parameters must meet (`YesNoVotePoll::validate_parameters`).
    /// `None` (not active) before protocol version 15.
    pub yes_no_vote_poll_parameters: OptionalFeatureVersion,
    /// How a yes/no poll is decided: its required voting power and whether it passes
    /// (`YesNoVotePoll::required_voting_power` and `YesNoVotePoll::passes`). `None` (not
    /// active) before protocol version 15.
    pub yes_no_vote_poll_decision: OptionalFeatureVersion,
}
