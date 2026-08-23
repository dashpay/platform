use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 13.
///
/// Identical to [`super::v2::SYSTEM_LIMITS_V2`] except that `max_document_value_depth` is set:
/// document property values are bounded to 256 nested containers, enforced by the wire decoder
/// and mirrored by document validation. The rule cannot activate in v12 because v12 is already
/// live on networks without it.
pub const SYSTEM_LIMITS_V3: SystemLimits = SystemLimits {
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
    daily_withdrawal_limit: 200_000_000_000_000, //2000 Dash (Core v22 limit)
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
