#[derive(Clone, Debug, Default)]
pub struct SystemLimits {
    pub estimated_contract_max_serialized_size: u16,
    pub max_field_value_size: u32,
    pub max_state_transition_size: u64,
    pub max_transitions_in_documents_batch: u16,
    pub withdrawal_transactions_per_block_limit: u16,
    pub max_withdrawal_amount: u64,
    pub daily_withdrawal_limit: Option<u64>,
}
