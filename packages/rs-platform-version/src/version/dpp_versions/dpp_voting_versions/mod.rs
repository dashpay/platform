use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DPPVotingVersions {
    pub default_vote_poll_time_duration_mainnet_ms: u64,
    pub default_vote_poll_time_duration_test_network_ms: u64,
    pub contested_document_vote_poll_stored_info_version: FeatureVersion,
}
