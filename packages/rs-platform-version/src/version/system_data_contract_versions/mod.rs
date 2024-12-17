pub mod v1;

use crate::version::protocol_version::FeatureVersion;

#[derive(Clone, Debug, Default)]
#[ferment_macro::export]
pub struct SystemDataContractVersions {
    pub withdrawals: FeatureVersion,
    pub dpns: FeatureVersion,
    pub dashpay: FeatureVersion,
    pub masternode_reward_shares: FeatureVersion,
    pub feature_flags: FeatureVersion,
    pub wallet: FeatureVersion,
}
