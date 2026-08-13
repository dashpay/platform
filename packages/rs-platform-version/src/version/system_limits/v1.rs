use crate::version::system_limits::SystemLimits;

pub const SYSTEM_LIMITS_V1: SystemLimits = SystemLimits {
    estimated_contract_max_serialized_size: 16384,
    max_field_value_size: 5120, //5 KiB
    max_document_value_depth: None,
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
    // Before lifting this cap above 1, the whole batch validation +
    // transformer + nonce-bump path must be reviewed and the atomicity /
    // nonce semantics fixed. Pulling the cap higher today would expose
    // those bugs to mainnet traffic.
    max_transitions_in_documents_batch: 1,
    withdrawal_transactions_per_block_limit: 4,
    retry_signing_expired_withdrawal_documents_per_block_limit: 1,
    max_withdrawal_amount: 50_000_000_000_000, //500 Dash
    // = dpp MIN_WITHDRAWAL_AMOUNT: ASSET_UNLOCK_TX_SIZE(190) * MIN_CORE_FEE_PER_BYTE(1)
    // * CREDITS_PER_DUFF(1000) = 190_000 credits = 190 duffs.
    min_withdrawal_amount: 190_000,
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
