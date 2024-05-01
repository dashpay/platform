pub mod v1;
#[derive(Clone, Debug, Default)]
pub struct StateTransitionMinFees {
    pub credit_transfer: u64,
    pub credit_withdrawal: u64,
    pub identity_update: u64,
    pub document_batch_sub_transition: u64,
    pub contract_create: u64,
    pub contract_update: u64,
}
