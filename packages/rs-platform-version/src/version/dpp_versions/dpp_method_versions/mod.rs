use versioned_feature_core::{FeatureVersion, OptionalFeatureVersion};

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

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
    /// What the token bundle of `TokenShieldedTransferWithShieldedFee`,
    /// `TokenUnshieldWithShieldedFee` and `TokenPurchaseFromShieldedPool` binds into its Orchard
    /// sighash after its own layout. `None` on versions where it binds no fee bundle: only its
    /// type, its token and the transition's transparent fields, which is what
    /// `validate_shielded_proof` v1 rebuilds. `Some(0)`: it also binds the Orchard commitment of
    /// the transition's fee bundle, which binds a digest of the token bundle's actions in turn,
    /// so neither bundle can be paired with a bundle of different effects.
    ///
    /// The client builders read this field through
    /// `dpp::shielded::token_pool_paid_fee_binding_extra_sighash_data`; `validate_shielded_proof`
    /// v2 verifies version 0 as a constant of its generation, and
    /// `token_pool_paid_fee_binding_should_agree_with_the_selected_verifier_at_every_protocol_version`
    /// holds the field and the selected verifier together.
    pub token_pool_paid_fee_binding: OptionalFeatureVersion,
}
