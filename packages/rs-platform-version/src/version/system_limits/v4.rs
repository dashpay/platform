use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 14 and above.
///
/// Identical to [`super::v3::SYSTEM_LIMITS_V3`] except that `daily_withdrawal_limit` is raised
/// from 2000 Dash to 4000 Dash, matching Core's doubled credit-pool unlock limit
/// (`LimitAmountV24`, DIP-0165, dashpay/dash#6662). v13 is already live on networks with the
/// 2000 Dash limit, so the change gates here.
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
    max_withdrawal_amount: 50_000_000_000_000,   //500 Dash
    daily_withdrawal_limit: 400_000_000_000_000, //4000 Dash (Core v24 limit; raised from 2000 in v14)
    min_withdrawal_amount: 1_000_000,            //1000 duffs (raised from 190 in v12)
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
};
