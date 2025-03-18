use crate::version::system_limits::SystemLimits;

pub const SYSTEM_LIMITS_V1: SystemLimits = SystemLimits {
    estimated_contract_max_serialized_size: 16384,
    max_field_value_size: 5120,       //5 KiB
    max_state_transition_size: 20480, //20 KiB
    max_transitions_in_documents_batch: 1,
    withdrawal_transactions_per_block_limit: 4,
    retry_signing_expired_withdrawal_documents_per_block_limit: 1,
    max_withdrawal_amount: 50_000_000_000_000, //500 Dash
    max_contract_group_size: 256,
    max_token_redemption_cycles: 128,
};
