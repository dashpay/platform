use bincode::{Decode, Encode};

pub mod v1;
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct VoteResolutionFundFees {
    /// This is the amount that will be deducted from an identity and used to pay for voting
    pub contested_document_vote_resolution_fund_required_amount: u64,
    /// This is the amount that will be deducted from an identity and used to pay for voting if we are currently locked
    pub contested_document_vote_resolution_unlock_fund_required_amount: u64,
    /// This is the amount that a single vote will cost
    pub contested_document_single_vote_cost: u64,
}

#[cfg(test)]
mod tests {
    use super::VoteResolutionFundFees;

    #[test]
    // If this test failed, then a new field was added in VoteResolutionFundFees. And the corresponding eq needs to be updated as well
    fn test_fee_storage_version_equality() {
        let version1 = VoteResolutionFundFees {
            contested_document_vote_resolution_fund_required_amount: 1,
            contested_document_vote_resolution_unlock_fund_required_amount: 2,
            contested_document_single_vote_cost: 3,
        };

        let version2 = VoteResolutionFundFees {
            contested_document_vote_resolution_fund_required_amount: 1,
            contested_document_vote_resolution_unlock_fund_required_amount: 2,
            contested_document_single_vote_cost: 3,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "VoteResolutionFundFees equality test failed. If a field was added or removed, update the Eq implementation.");
    }
}
