use crate::version::dpp_versions::dpp_method_versions::DPPMethodVersions;

/// DPP method versions 3. Introduced in protocol v14: `daily_withdrawal_limit` 1 → 2 replaces the
/// flat daily withdrawal limit with a percentage of the total credits Platform held a day ago
/// (`SystemLimits::daily_withdrawal_limit_percent`). Everything else matches V2.
pub const DPP_METHOD_VERSIONS_V3: DPPMethodVersions = DPPMethodVersions {
    epoch_core_reward_credits_for_distribution: 0,
    daily_withdrawal_limit: 2,
    deduct_fee_from_outputs_or_remaining_balance_of_inputs: 0,
    compute_minimum_shielded_fee: 0,
    shielded_extra_sighash_data: 0,
};
