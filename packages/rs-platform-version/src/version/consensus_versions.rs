use versioned_feature_core::FeatureVersion;

#[derive(Clone, Debug, Default)]
pub struct ConsensusVersions {
    pub tenderdash_consensus_version: FeatureVersion,
}
