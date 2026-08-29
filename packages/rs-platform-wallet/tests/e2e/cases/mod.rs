//! End-to-end test cases. Each submodule hosts
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`].
//!
//! Hosts the platform-address (PA), identity (ID), asset-lock (AL/CR),
//! DPNS, token (TK), shielded (SH) and Found-bug-pin cases; see
//! `TEST_SPEC.md` for the priority matrix.

// Asset-lock manager cases (Wave AL — see TEST_SPEC.md ### Asset Lock (AL))
pub mod al_001_concurrent_asset_lock_builds;
pub mod cr_001_spv_mn_list_sync_readiness;
pub mod cr_003_asset_lock_funded_registration;
pub mod cr_004_legacy_bip32_utxo_update_after_spend;
pub mod dpns_001_register_name;
// Found-bug pins (see TEST_SPEC.md ### Found bugs)
pub mod found_004_fund_from_asset_lock_silent_fallback;
pub mod found_012_account_type_tunnel_vision;
pub mod found_013_recover_asset_lock_silent_failure;
pub mod found_017_register_wallet_store_error_lost;
pub mod found_017_register_wallet_store_ok_persists;
pub mod found_021_instant_lock_dropped_on_context_promotion;
pub mod found_022_asset_lock_builder_consumes_change_index_on_failure;
pub mod found_025_address_sync_silent_discard;
pub mod found_coinjoin_gap_limit_sync;
pub mod id_001_register_identity_from_addresses;
pub mod id_002_top_up_identity;
pub mod id_002b_asset_lock_top_up;
pub mod id_003_identity_to_identity_transfer;
pub mod id_005_identity_to_addresses_transfer;
pub mod id_007_identity_auth_addresses_not_monitored;
pub mod id_sweep_recovers_identity_credits;
pub mod pa_001_multi_output;
pub mod pa_001b_change_address_branch;
pub mod pa_001c_zero_credit_output;
pub mod pa_002_partial_fund;
pub mod pa_002b_zero_change;
pub mod pa_003_fee_scaling;
pub mod pa_004_sweep_back;
pub mod pa_004b_sweep_dust_boundary;
pub mod pa_004c_sweep_zero_balance;
pub mod pa_005_address_rotation;
pub mod pa_005b_gap_limit_triplet;
pub mod pa_005c_receive_address_reservation_lifecycle;
pub mod pa_006_replay_safety;
pub mod pa_006b_concurrent_broadcast;
pub mod pa_007_sync_watermark;
pub mod pa_007b_concurrent_sync;
pub mod pa_008_concurrent_funding;
pub mod pa_008b_cross_wallet_funding;
pub mod pa_008c_funding_mutex_observable;
pub mod pa_009_min_input_amount;
pub mod pa_3040_bug_pin;
pub mod print_bank_address;
pub mod print_bank_address_offline;
// Shielded (Orchard) cases (Wave H — see TEST_SPEC.md ### Shielded (SH))
#[cfg(feature = "shielded")]
pub mod sh_001_shield_from_account;
#[cfg(feature = "shielded")]
pub mod sh_002_shield_unshield_round_trip;
#[cfg(feature = "shielded")]
pub mod sh_003_shielded_transfer;
#[cfg(feature = "shielded")]
pub mod sh_004_balance_after_sync;
#[cfg(feature = "shielded")]
pub mod sh_005_inmemory_witness_split;
#[cfg(feature = "shielded")]
pub mod sh_006_add_account_never_syncs;
#[cfg(feature = "shielded")]
pub mod sh_007_pre_bind_note_witnessable;
#[cfg(feature = "shielded")]
pub mod sh_008_unshield_insufficient_balance;
#[cfg(feature = "shielded")]
pub mod sh_009_zero_amount_rejected;
#[cfg(feature = "shielded")]
pub mod sh_010_double_spend_reservation;
#[cfg(feature = "shielded")]
pub mod sh_011_note_selection_convergence;
#[cfg(feature = "shielded")]
pub mod sh_012_sync_watermark_idempotency;
#[cfg(feature = "shielded")]
pub mod sh_013_bind_empty_accounts;
#[cfg(feature = "shielded")]
pub mod sh_014_spend_before_bind;
#[cfg(feature = "shielded")]
pub mod sh_018_shield_from_asset_lock;
#[cfg(feature = "shielded")]
pub mod sh_019_shielded_withdraw_l1;
// Shielded adversarial / abuse cases (Wave H follow-up — SH-020..SH-035)
#[cfg(feature = "shielded")]
pub mod sh_020_double_spend_two_transitions;
#[cfg(feature = "shielded")]
pub mod sh_021_nullifier_replay_after_restart;
#[cfg(feature = "shielded")]
pub mod sh_022_value_not_conserved;
#[cfg(feature = "shielded")]
pub mod sh_023_fee_underpayment;
#[cfg(feature = "shielded")]
pub mod sh_024_value_boundary_overflow;
#[cfg(feature = "shielded")]
pub mod sh_025_forged_proof;
#[cfg(feature = "shielded")]
pub mod sh_026_anchor_mismatch;
#[cfg(feature = "shielded")]
pub mod sh_027_malformed_note_serde;
// SH-028 / SH-029 BLOCKED — no injectable sync-source seam (see TEST_SPEC.md).
#[cfg(feature = "shielded")]
pub mod sh_030_cross_network_recipient;
#[cfg(feature = "shielded")]
pub mod sh_031_rebind_different_seed;
#[cfg(feature = "shielded")]
pub mod sh_032_exact_change_boundary;
#[cfg(feature = "shielded")]
pub mod sh_033_duplicate_nullifier_in_bundle;
#[cfg(feature = "shielded")]
pub mod sh_034_tampered_binding_signature;
#[cfg(feature = "shielded")]
pub mod sh_035_replayed_asset_lock_proof;
#[cfg(feature = "shielded")]
pub mod sh_036_identity_create_from_shielded_pool;
// Token tests (Wave 2 — per TEST_SPEC.md ### Tokens (TK))
pub mod tk_001_token_transfer;
pub mod tk_001b_token_transfer_zero;
pub mod tk_001c_token_transfer_after_reissue;
pub mod tk_002_token_claim_perpetual;
pub mod tk_003_register_token_contract;
pub mod tk_004_token_transfer_round_trip;
pub mod tk_005_token_mint;
pub mod tk_005b_token_mint_to_other;
pub mod tk_006_token_burn;
pub mod tk_007_token_freeze;
pub mod tk_008_token_unfreeze;
pub mod tk_009_token_destroy_frozen;
pub mod tk_010_token_pause_resume;
pub mod tk_011_token_price_purchase;
pub mod tk_012_token_update_config;
pub mod tk_013_token_claim_pre_programmed;
pub mod tk_014_token_group_action;
