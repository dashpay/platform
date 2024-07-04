use bincode::{Decode, Encode};

pub mod v1;
#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct StateTransitionMinFees {
    pub credit_transfer: u64,
    pub credit_withdrawal: u64,
    pub identity_update: u64,
    pub document_batch_sub_transition: u64,
    pub contract_create: u64,
    pub contract_update: u64,
}

impl PartialEq for StateTransitionMinFees {
    fn eq(&self, other: &Self) -> bool {
        self.credit_transfer == other.credit_transfer
            && self.credit_withdrawal == other.credit_withdrawal
            && self.identity_update == other.identity_update
            && self.document_batch_sub_transition == other.document_batch_sub_transition
            && self.contract_create == other.contract_create
            && self.contract_update == other.contract_update
    }
}
