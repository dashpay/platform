use crate::version::fee::vote_resolution_fund_fees::VoteResolutionFundFees;

pub const VOTE_RESOLUTION_FUND_FEES_VERSION1: VoteResolutionFundFees = VoteResolutionFundFees {
    contested_document_vote_resolution_fund_required_amount: 20000000000, // 0.2 Dash
    contested_document_vote_resolution_unlock_fund_required_amount: 400000000000, // 4 Dash
    contested_document_single_vote_cost: 10000000,                        // 0.0001 Dash
};
