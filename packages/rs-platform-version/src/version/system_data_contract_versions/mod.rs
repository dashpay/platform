pub mod v1;
pub mod v2;
pub mod v3;

use crate::version::FeatureVersion;

#[derive(Clone, Debug, Default)]
pub struct SystemDataContractVersions {
    pub withdrawals: FeatureVersion,
    pub dpns: FeatureVersion,
    pub dashpay: FeatureVersion,
    pub masternode_reward_shares: FeatureVersion,
    pub wallet: FeatureVersion,
    pub token_history: FeatureVersion,
    pub keyword_search: FeatureVersion,
    pub document_history: FeatureVersion,
}
