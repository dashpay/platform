pub mod v1;
#[derive(Clone, Debug, Default)]
pub struct VoteResolutionFundFees {
    /// This is the amount that will be deducted from an identity and used to pay for voting
    pub conflicting_vote_resolution_fund_required_amount: u64,
}
