use crate::version::fee::state_transition_min_fees::v1::STATE_TRANSITION_MIN_FEES_VERSION1;
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

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct StateTransitionMinFeesBeforeProtocolVersion11 {
    pub credit_transfer: u64,
    pub credit_withdrawal: u64,
    pub identity_update: u64,
    pub document_batch_sub_transition: u64,
    pub contract_create: u64,
    pub contract_update: u64,
    pub masternode_vote: u64,
}

impl From<StateTransitionMinFeesBeforeProtocolVersion11> for StateTransitionMinFees {
    fn from(value: StateTransitionMinFeesBeforeProtocolVersion11) -> Self {
        StateTransitionMinFees {
            credit_transfer: value.credit_transfer,
            credit_transfer_to_addresses: STATE_TRANSITION_MIN_FEES_VERSION1
                .credit_transfer_to_addresses, // new field in v4
            credit_withdrawal: value.credit_withdrawal,
            identity_update: value.identity_update,
            document_batch_sub_transition: value.document_batch_sub_transition,
            contract_create: value.contract_create,
            contract_update: value.contract_update,
            masternode_vote: value.masternode_vote,
        }
    }
}
