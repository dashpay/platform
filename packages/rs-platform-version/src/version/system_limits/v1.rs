use crate::version::system_limits::SystemLimits;

pub const SYSTEM_LIMITS_V1: SystemLimits = SystemLimits {
    estimated_contract_max_serialized_size: 16384,
    max_field_value_size: 5120,       //5 KiB
    max_state_transition_size: 20480, //20 KiB
    // TODO: this is currently capped at 1 because the batch state-transition
    // pipeline has known correctness issues with multi-transition batches:
    //   - It is not atomic: when one transition errors, earlier successful
    //     transitions inside the same batch are still applied to state.
    //   - Nonce-bump semantics for mixed success/failure batches are not
    //     well-defined: it is unclear whether to bump the nonce for the
    //     failed transition only, for all transitions, or for none — and the
    //     transformer/dispatch code does not consistently express any of
    //     those policies (see issue #2867 and PR #3608).
    // Before lifting this cap above 1, the whole batch validation +
    // transformer + nonce-bump path must be reviewed and the atomicity /
    // nonce semantics fixed. Pulling the cap higher today would expose
    // those bugs to mainnet traffic.
    max_transitions_in_documents_batch: 1,
    withdrawal_transactions_per_block_limit: 4,
    retry_signing_expired_withdrawal_documents_per_block_limit: 1,
    max_withdrawal_amount: 50_000_000_000_000, //500 Dash
    max_contract_group_size: 256,
    max_token_redemption_cycles: 128,
    // 16 actions x 408 bytes + ~5,305 bytes overhead = ~11,833 bytes (within 20 KiB max_state_transition_size)
    max_shielded_transition_actions: 16,
};
