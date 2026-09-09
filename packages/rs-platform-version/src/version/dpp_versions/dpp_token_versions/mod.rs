pub mod v1;
pub mod v2;
pub mod v3;

use versioned_feature_core::FeatureVersion;

#[derive(Clone, Debug, Default)]
pub struct DPPTokenVersions {
    pub identity_token_info_default_structure_version: FeatureVersion,
    pub identity_token_status_default_structure_version: FeatureVersion,
    pub token_contract_info_default_structure_version: FeatureVersion,
    /// Version for the token config update action_id calculation.
    /// v0: uses only the u8 discriminant of the config change item (vulnerable to value swap)
    /// v1: includes the full serialized config change item in the hash
    pub token_config_update_action_id_version: FeatureVersion,
    /// Version for the set-price-for-direct-purchase action_id calculation.
    /// v0: uses only minimum_purchase_amount_and_price().1 (vulnerable to schedule swap)
    /// v1: includes the full serialized TokenPricingSchedule in the hash
    pub token_set_price_action_id_version: FeatureVersion,
    /// Version for the transcendental math (`ln`, `exp`, `pow`) in `DistributionFunction::evaluate`.
    /// v0: std `f64` methods, which link to the platform libm and differ by 1 ulp between
    ///     aarch64-musl and x86_64-musl (musl's `__FP_FAST_FMA` branch); a claim amount can land
    ///     on either side of a `floor` boundary and split the app hash.
    /// v1: the pinned pure-Rust `libm` crate, bit-identical on every target.
    pub distribution_function_evaluate_version: FeatureVersion,
}
