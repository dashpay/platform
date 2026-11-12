use crate::version::fee::vote_resolution_fund_fees::v1::VOTE_RESOLUTION_FUND_FEES_VERSION1;
use crate::version::fee::vote_resolution_fund_fees::VoteResolutionFundFees;

/// Introduced in protocol version 14 (4.2).
pub const VOTE_RESOLUTION_FUND_FEES_VERSION2: VoteResolutionFundFees = VoteResolutionFundFees {
    contested_document_vote_resolution_fund_required_amount: 10_000_000_000, // 0.1 DASH
    // A fifth of the 0.0001 DASH before: a contest's fund covers five times the votes
    contested_document_single_vote_cost: 2_000_000, // 0.00002 DASH
    // An application in a moderation election prefunds 25,000 masternode votes
    moderation_vote_resolution_fund_required_amount: 50_000_000_000, // 0.5 DASH
    // The first 250 contenders of a contest pay its fund, and then the fund doubles for every
    // 50 more: twice it for the 251st to the 300th, up to 32,768 times it for the 951st to the
    // 1,000th (3,276.8 DASH for a DPNS name)
    contested_document_contenders_before_fund_doubling: 250,
    contested_document_contenders_per_fund_doubling: 50,
    ..VOTE_RESOLUTION_FUND_FEES_VERSION1
};
