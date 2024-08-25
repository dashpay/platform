use crate::version::fee::state_transition_min_fees::StateTransitionMinFees;

pub const STATE_TRANSITION_MIN_FEES_VERSION1: StateTransitionMinFees = StateTransitionMinFees {
    credit_transfer: 100000,
    credit_withdrawal: 100000,
    identity_update: 100000,
    document_batch_sub_transition: 100000,
    contract_create: 100000,
    contract_update: 100000,
    masternode_vote: 100000,
};
