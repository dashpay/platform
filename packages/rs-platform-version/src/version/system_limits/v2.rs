use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 12 and above.
///
/// Identical to [`super::v1::SYSTEM_LIMITS_V1`] except that `min_withdrawal_amount` is raised
/// from 190,000 credits (190 duffs) to 1,000,000 credits (1000 duffs): the previous floor was
/// the bare asset-unlock transaction fee and too low a minimum for a Core `TxOut`.
pub const SYSTEM_LIMITS_V2: SystemLimits = SystemLimits {
    estimated_contract_max_serialized_size: 16384,
    max_field_value_size: 5120,       //5 KiB
    max_state_transition_size: 20480, //20 KiB
    max_transitions_in_documents_batch: 1,
    withdrawal_transactions_per_block_limit: 4,
    retry_signing_expired_withdrawal_documents_per_block_limit: 1,
    max_withdrawal_amount: 50_000_000_000_000, //500 Dash
    min_withdrawal_amount: 1_000_000,          //1000 duffs (raised from 190 in v12)
    max_contract_group_size: 256,
    max_token_redemption_cycles: 128,
    // 16 actions x 408 bytes + ~5,305 bytes overhead = ~11,833 bytes (within 20 KiB max_state_transition_size)
    max_shielded_transition_actions: 16,
};
