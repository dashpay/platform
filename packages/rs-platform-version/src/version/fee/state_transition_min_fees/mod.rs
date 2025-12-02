use bincode::{Decode, Encode};

pub mod v1;
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct StateTransitionMinFees {
    pub credit_transfer: u64,
    pub credit_transfer_to_addresses: u64,
    pub credit_withdrawal: u64,
    pub identity_update: u64,
    pub document_batch_sub_transition: u64,
    pub contract_create: u64,
    pub contract_update: u64,
    pub masternode_vote: u64,
}
