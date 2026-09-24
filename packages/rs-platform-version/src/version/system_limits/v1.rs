use crate::version::system_limits::SystemLimits;

pub const SYSTEM_LIMITS_V1: SystemLimits = SystemLimits {
    estimated_contract_max_serialized_size: 16384,
    max_field_value_size: 5120, //5 KiB
    max_document_value_depth: None,
    max_typed_array_items: 1024,
    max_references_per_document: 256,
    max_reference_operands: 4,
    max_reference_expression_depth: 4,
    max_property_constraints: 16,
    max_property_constraint_nodes: 32,
    max_state_transition_size: 20480, //20 KiB
    // TODO: this is currently capped at 1 because the batch state-transition
    // pipeline has known correctness issues with multi-transition batches:
    //   - It is not atomic: when one transition errors, earlier successful
    //     transitions inside the same batch are still applied to state.
    //   - Nonce-bump semantics for mixed success/failure batches are not
    //     well-defined: it is unclear whether to bump the nonce for the
    //     failed transition only, for all transitions, or for none — and the
    //     transformer/dispatch code does not consistently express any of
    //     those policies (see issue #2867).
    //   - The transitions of one batch are flattened into a single GroveDB
    //     batch whose document operations cannot see each other, so two of
    //     them that jointly empty an index group leave the group tree behind
    //     — a document-less group that still ranks, and still proves. See
    //     SystemLimits::max_transitions_in_documents_batch.
    //   - indexOnly (PV14) state probes see only pre-batch state, and the
    //     DPP duplicate check fingerprints (type, id) — while two indexOnly
    //     creates (or two delete-by-values) with different entropy-derived
    //     ids can address the very same index entries. Both would validate
    //     and then collide as duplicate operations for one qualified key
    //     inside the grove batch. Lifting the cap requires cross-sibling
    //     derived-entry tracking in batch state validation.
    // Before lifting this cap above 1, the whole batch validation +
    // transformer + nonce-bump path must be reviewed and the atomicity /
    // nonce semantics fixed. Pulling the cap higher today would expose
    // those bugs to mainnet traffic.
    max_transitions_in_documents_batch: 1,
    withdrawal_transactions_per_block_limit: 4,
    retry_signing_expired_withdrawal_documents_per_block_limit: 1,
    max_withdrawal_amount: 50_000_000_000_000, //500 Dash
    daily_withdrawal_limit_percent: None,      // relative daily withdrawal limit arrives in v14
    max_daily_withdrawal_amount: None,
    // = dpp MIN_WITHDRAWAL_AMOUNT: ASSET_UNLOCK_TX_SIZE(190) * MIN_CORE_FEE_PER_BYTE(1)
    // * CREDITS_PER_DUFF(1000) = 190_000 credits = 190 duffs.
    min_withdrawal_amount: 190_000,
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
