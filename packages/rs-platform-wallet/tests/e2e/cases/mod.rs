//! End-to-end test cases. Each submodule hosts
//! `#[tokio_shared_rt::test(shared)]` entries that share the
//! process-wide [`super::framework::E2eContext`].
//!
//! P0 platform-address (PA) cases land here first; the remaining
//! TEST_SPEC.md priorities (P1, P2, ID-, DP-, DPNS-, TK-, …) follow
//! in subsequent PRs.

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
