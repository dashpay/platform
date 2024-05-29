pub mod v1;
#[derive(Clone, Debug, Default)]
pub struct VoteResolutionFundFees {
    /// This is the amount that will be deducted from an identity and used to pay for voting
    pub contested_document_vote_resolution_fund_required_amount: u64,
    /// This is the amount that a single vote will cost
    pub contested_document_single_vote_cost: u64,
}
