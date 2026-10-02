use versioned_feature_core::{FeatureVersion, OptionalFeatureVersion};

pub mod v1;
pub mod v2;
pub mod v3;

#[derive(Clone, Debug, Default)]
pub struct DPPMethodVersions {
    pub epoch_core_reward_credits_for_distribution: FeatureVersion,
    pub daily_withdrawal_limit: FeatureVersion,
    pub deduct_fee_from_outputs_or_remaining_balance_of_inputs: FeatureVersion,
    pub compute_minimum_shielded_fee: FeatureVersion,
    pub shielded_extra_sighash_data: FeatureVersion,
    /// The Core-anchored withdrawal limit: how much Core's credit pool may still drop given its
    /// balance now and at the window start (`SystemLimits::core_credit_pool_unlock_limit_percent`
    /// and `core_credit_pool_unlock_limit_floor`). Exists from protocol version 14.
    pub core_credit_pool_unlock_limit: OptionalFeatureVersion,
}
