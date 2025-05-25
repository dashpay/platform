pub mod v1;

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct SystemLimits {
    pub estimated_contract_max_serialized_size: u16,
    pub max_field_value_size: u32,
    /// Max size of a state transition in bytes.
    ///
    /// NOTE: This must be equal to the `max-tx-bytes` in the Tenderdash config
    pub max_state_transition_size: u64,
    pub max_transitions_in_documents_batch: u16,
    pub withdrawal_transactions_per_block_limit: u16,
    pub retry_signing_expired_withdrawal_documents_per_block_limit: u16,
    pub max_withdrawal_amount: u64,
    pub max_contract_group_size: u16,
    // This the max redemption cycles we can process if we don't use a constant distribution
    // For a constant perpetual distribution this is very cheap since it's just a multiplication
    // For other distributions we much calculate at each cycle the rewards, so we don't want to
    // do this that much
    pub max_token_redemption_cycles: u32,
}
