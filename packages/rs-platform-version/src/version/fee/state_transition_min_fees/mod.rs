use bincode::{Decode, Encode};

pub mod v1;
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct StateTransitionMinFees {
    pub credit_transfer: u64,
    pub credit_withdrawal: u64,
    pub identity_update: u64,
    pub document_batch_sub_transition: u64,
    pub contract_create: u64,
    pub contract_update: u64,
    pub masternode_vote: u64,
}

#[cfg(test)]
mod tests {
    use super::StateTransitionMinFees;

    #[test]
    // If this test failed, then a new field was added in StateTransitionMinFees. And the corresponding eq needs to be updated as well
    fn test_fee_state_transition_min_fees_version_equality() {
        let version1 = StateTransitionMinFees {
            credit_transfer: 1,
            credit_withdrawal: 2,
            identity_update: 3,
            document_batch_sub_transition: 4,
            contract_create: 5,
            contract_update: 6,
            masternode_vote: 7,
        };

        let version2 = StateTransitionMinFees {
            credit_transfer: 1,
            credit_withdrawal: 2,
            identity_update: 3,
            document_batch_sub_transition: 4,
            contract_create: 5,
            contract_update: 6,
            masternode_vote: 7,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "StateTransitionMinFees equality test failed. If a field was added or removed, update the Eq implementation.");
    }
}
