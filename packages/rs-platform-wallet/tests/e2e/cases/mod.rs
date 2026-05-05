//! End-to-end test cases. Each submodule hosts
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`].
//!
//! P0 platform-address (PA) cases land here first; the remaining
//! TEST_SPEC.md priorities (P1, P2, ID-, DP-, DPNS-, TK-, …) follow
//! in subsequent PRs.

pub mod cr_003_asset_lock_funded_registration;
pub mod dpns_001_register_name;
pub mod id_001_register_identity_from_addresses;
pub mod id_002_top_up_identity;
pub mod id_003_identity_to_identity_transfer;
pub mod id_005_identity_to_addresses_transfer;
pub mod id_007_identity_auth_addresses_monitored;
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
pub mod pa_006_replay_safety;
pub mod pa_006b_concurrent_broadcast;
pub mod pa_007_sync_watermark;
pub mod pa_007b_concurrent_sync;
pub mod pa_008_concurrent_funding;
pub mod pa_008b_cross_wallet_funding;
pub mod pa_008c_funding_mutex_observable;
pub mod pa_009_min_input_amount;
pub mod pa_010_bank_starvation;
pub mod pa_3040_bug_pin;
pub mod print_bank_address;
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
