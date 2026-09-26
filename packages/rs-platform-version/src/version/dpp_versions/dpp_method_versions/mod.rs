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
    /// The preimage the outputs-only bundles of the credit pool (`Shield`, `ShieldFromIdentity`,
    /// `ShieldFromAssetLock`) bind into their Orchard sighash. `None` on versions that predate the
    /// binding: they bind nothing, which is what every shipped verifier expects. `Some(0)`: they
    /// bind `kind tag (1) || owner (32)`.
    ///
    /// The client builders and the consensus sites protocol version 14 selects read this one field
    /// through the same helpers, so a client building for a protocol version produces the preimage
    /// that version's verifier rebuilds. `validate_shielded_proof` v0 and the
    /// `ShieldFromAssetLock` transform v0 rebuild an empty preimage without reading it;
    /// `credit_pool_bundle_binding_should_agree_with_the_selected_shield_verifiers_at_every_protocol_version`
    /// holds them and this field together.
    pub credit_pool_bundle_binding: OptionalFeatureVersion,
}
