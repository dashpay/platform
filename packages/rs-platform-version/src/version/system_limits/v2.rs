use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 12.
///
/// Identical to [`super::v1::SYSTEM_LIMITS_V1`] except that `min_withdrawal_amount` is raised
/// from 190,000 credits (190 duffs) to 1,000,000 credits (1000 duffs): the previous floor was
/// the bare asset-unlock transaction fee and too low a minimum for a Core `TxOut`.
pub const SYSTEM_LIMITS_V2: SystemLimits = SystemLimits {
    estimated_contract_max_serialized_size: 16384,
    max_field_value_size: 5120, //5 KiB
    // v12 is already active on live networks; the depth limit activates in v13 (see v3).
    max_document_value_depth: None,
    max_state_transition_size: 20480, //20 KiB
    // Load-bearing for state correctness, not just for throughput — see
    // SystemLimits::max_transitions_in_documents_batch and SYSTEM_LIMITS_V1.
    max_transitions_in_documents_batch: 1,
    withdrawal_transactions_per_block_limit: 4,
    retry_signing_expired_withdrawal_documents_per_block_limit: 1,
    max_withdrawal_amount: 50_000_000_000_000, //500 Dash
    daily_withdrawal_limit_percent: None,      // relative daily withdrawal limit arrives in v14
    max_daily_withdrawal_amount: None,
    min_withdrawal_amount: 1_000_000, //1000 duffs (raised from 190 in v12)
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
    max_time_range_overlap_factor: None,
    max_time_range_ttl_seconds: None,
    max_time_range_ttl_drop_operations_per_write: None,
};
