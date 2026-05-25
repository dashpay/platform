use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 14 and above.
///
/// Identical to [`super::v3::SYSTEM_LIMITS_V3`] except for two changes:
///
/// * The daily withdrawal limit becomes relative: `daily_withdrawal_limit_percent` is set to 15,
///   so Platform pools at most 15% of the total credits it held a day ago into asset unlock
///   transactions per 24 hours — never below one maximal withdrawal and never above
///   `max_daily_withdrawal_amount`, Core's 4000 Dash unlock capacity per day — instead of the
///   flat 2000 Dash that applied from v8 (matching Core v22's `LimitAmountV22`). v13 is already
///   live on networks with the flat limit, so the change gates here.
/// * `max_time_range_overlap_factor` is set: a `timeRange` index transform may declare at most
///   24 overlapping windows per timestamp (a day-long window sliding hourly). The rule cannot
///   exist before v14 because the `timeRange` keyword itself is only admitted by the v14
///   document meta-schema.
pub const SYSTEM_LIMITS_V4: SystemLimits = SystemLimits {
    estimated_contract_max_serialized_size: 16384,
    max_field_value_size: 5120, //5 KiB
    // Use the protocol's existing data-contract schema-depth ceiling as the conservative
    // instance budget, bounding pre-schema work well above known document requirements.
    max_document_value_depth: Some(256),
    max_state_transition_size: 20480, //20 KiB
    // Load-bearing for state correctness, not just for throughput — see
    // SystemLimits::max_transitions_in_documents_batch and SYSTEM_LIMITS_V1.
    max_transitions_in_documents_batch: 1,
    withdrawal_transactions_per_block_limit: 4,
    retry_signing_expired_withdrawal_documents_per_block_limit: 1,
    max_withdrawal_amount: 50_000_000_000_000, //500 Dash
    daily_withdrawal_limit_percent: Some(15), // 15% of the total credits a day ago (replaces the flat 2000 Dash in v14)
    max_daily_withdrawal_amount: Some(400_000_000_000_000), // 4000 Dash: Core's unlock capacity per day (LimitAmountV24)
    min_withdrawal_amount: 1_000_000,                       //1000 duffs (raised from 190 in v12)
    max_contract_group_size: 256,
    max_token_redemption_cycles: 128,
    // NOTE: the Halo 2 proof grows with the action count (~2,273 B/action on
    // top of the 408 B serialized action), so a transition's on-wire size is
    // ~2,681 B per action + ~2,930 B fixed (measured: 2 actions → 8,294 B,
    // 6 → 19,018 B). The effective per-transition action bound under the
    // 20 KiB `max_state_transition_size` is therefore 6, NOT this cap — 16
    // only becomes reachable if the size limit is raised. Pinned by dpp's
    // `seed_pool_batch_fits_max_state_transition_size` signing test.
    max_shielded_transition_actions: 16,
    max_time_range_overlap_factor: Some(24),
};
