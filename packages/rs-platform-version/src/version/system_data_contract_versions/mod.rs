pub mod v1;

use versioned_feature_core::FeatureVersion;

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct SystemDataContractVersions {
    pub withdrawals: FeatureVersion,
    pub dpns: FeatureVersion,
    pub dashpay: FeatureVersion,
    pub masternode_reward_shares: FeatureVersion,
    pub feature_flags: FeatureVersion,
    pub wallet: FeatureVersion,
    pub token_history: FeatureVersion,
    pub keyword_search: FeatureVersion,
}
