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
    max_typed_array_items: 1024,
    max_references_per_document: 256,
    max_reference_operands: 4,
    max_reference_expression_depth: 4,
    max_property_constraints: 16,
    max_property_constraint_nodes: 32,
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
    core_dust_relay_fee_per_kb: None, // expired dust withdrawals fail from v14
    max_core_fee_per_byte: None,
    max_group_member_count: 256,
    max_contract_group_memberships_per_contract: 16,
    max_contract_group_admins: 16,
    max_contract_group_name_length: 64,
    max_contract_group_description_length: 256,
    max_contract_moderators: 16,
    max_contract_suspension_until: 9_007_199_254_740_991,
    max_contract_moderation_reason_length: 1024,
    max_contract_warnings_per_identity: 16,
    max_contract_moderation_reason_documents: 16,
    min_contract_moderation_election_window_seconds: 86_400, // one day
    max_contract_moderation_election_window_seconds: 2_419_200, // four weeks
    min_contract_moderation_challenge_cool_down_seconds: 1_209_600, // two weeks
    max_contract_moderation_challenge_cool_down_seconds: 94_608_000, // three years of 365 days
    contract_document_restore_window_ms: 604_800_000,        // 7 days
    max_contract_moderation_added_moderators: 15,
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
    min_time_range_ttl_drop_operations_per_write: None,
    minimum_grovedb_proof_envelope_version: 0, // V0 envelopes stay accepted until v14
};
