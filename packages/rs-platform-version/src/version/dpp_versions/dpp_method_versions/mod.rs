use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DPPMethodVersions {
    pub epoch_core_reward_credits_for_distribution: FeatureVersion,
    pub daily_withdrawal_limit: FeatureVersion,
    pub modify_inputs_and_outputs_for_fee: FeatureVersion,
}
