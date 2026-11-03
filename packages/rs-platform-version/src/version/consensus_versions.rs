use versioned_feature_core::FeatureVersion;

#[derive(Clone, Debug, Default)]
pub struct ConsensusVersions {
    pub tenderdash_consensus_version: FeatureVersion,
    /// Full evidence override; `None` leaves the chain's evidence parameters unchanged.
    pub evidence: Option<ConsensusEvidenceParams>,
}

/// Evidence limits installed together because Tenderdash replaces the full evidence message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsensusEvidenceParams {
    /// Maximum serialized evidence bytes per block; zero prevents inclusion.
    pub max_bytes: i64,
    /// Maximum evidence age in blocks; must be positive even when inclusion is disabled.
    pub max_age_num_blocks: i64,
    /// Maximum evidence age in seconds; must be positive even when inclusion is disabled.
    pub max_age_duration_seconds: i64,
}
