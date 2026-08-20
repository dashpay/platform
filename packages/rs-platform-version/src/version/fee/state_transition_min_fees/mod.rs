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
    // Address-based state transitions
    pub address_credit_withdrawal: u64,
    pub address_funds_transfer_input_cost: u64,
    pub address_funds_transfer_output_cost: u64,
    pub identity_create_base_cost: u64,
    pub identity_topup_base_cost: u64,
    pub identity_key_in_creation_cost: u64,
    /// Expected (display) fee components for
    /// `AddressFundingFromAssetLockTransition` — a client-side estimate of
    /// the GroveDB-metered fee actually charged at execution, NOT a
    /// consensus value (no consensus path reads these; the consensus floor
    /// is `calculate_min_required_fee`). Calibrated against `apply = true`
    /// execution in the drive-abci calibration test with headroom for
    /// state-tree growth.
    pub address_funding_expected_base_fee: u64,
    pub address_funding_expected_fee_per_input: u64,
    pub address_funding_expected_fee_per_output: u64,
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
                .credit_transfer_to_addresses,
            credit_withdrawal: value.credit_withdrawal,
            identity_update: value.identity_update,
            document_batch_sub_transition: value.document_batch_sub_transition,
            contract_create: value.contract_create,
            contract_update: value.contract_update,
            masternode_vote: value.masternode_vote,
            // Address-based state transitions (new)
            address_credit_withdrawal: STATE_TRANSITION_MIN_FEES_VERSION1.address_credit_withdrawal,
            address_funds_transfer_input_cost: STATE_TRANSITION_MIN_FEES_VERSION1
                .address_funds_transfer_input_cost,
            address_funds_transfer_output_cost: STATE_TRANSITION_MIN_FEES_VERSION1
                .address_funds_transfer_output_cost,
            identity_create_base_cost: STATE_TRANSITION_MIN_FEES_VERSION1.identity_create_base_cost,
            identity_topup_base_cost: STATE_TRANSITION_MIN_FEES_VERSION1.identity_topup_base_cost,
            identity_key_in_creation_cost: STATE_TRANSITION_MIN_FEES_VERSION1
                .identity_key_in_creation_cost,
            address_funding_expected_base_fee: STATE_TRANSITION_MIN_FEES_VERSION1
                .address_funding_expected_base_fee,
            address_funding_expected_fee_per_input: STATE_TRANSITION_MIN_FEES_VERSION1
                .address_funding_expected_fee_per_input,
            address_funding_expected_fee_per_output: STATE_TRANSITION_MIN_FEES_VERSION1
                .address_funding_expected_fee_per_output,
        }
    }
}
