use crate::version::FeatureVersion;

#[derive(Clone, Debug, Default)]
pub struct SystemDataContractVersions {
    pub withdrawals: FeatureVersion,
    pub dpns: FeatureVersion,
    pub dashpay: FeatureVersion,
    pub masternode_reward_shares: FeatureVersion,
    pub feature_flags: FeatureVersion,
}
