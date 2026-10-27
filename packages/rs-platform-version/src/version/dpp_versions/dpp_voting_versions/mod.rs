use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DPPVotingVersions {
    pub default_vote_poll_time_duration_mainnet_ms: u64,
    pub default_vote_poll_time_duration_test_network_ms: u64,
    pub contested_document_vote_poll_stored_info_version: FeatureVersion,
    /// The stored info of a yes/no poll (protocol version 15).
    pub yes_no_vote_poll_stored_info_version: FeatureVersion,
    /// The rules a yes/no poll's parameters must meet (`YesNoVotePoll::validate_parameters`,
    /// protocol version 15).
    pub yes_no_vote_poll_parameters: FeatureVersion,
    /// How a yes/no poll is decided: its required voting power and whether it passes
    /// (`YesNoVotePoll::required_voting_power` and `YesNoVotePoll::passes`, protocol version 15).
    pub yes_no_vote_poll_decision: FeatureVersion,
}
