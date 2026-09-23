use crate::version::fee::vote_resolution_fund_fees::v1::VOTE_RESOLUTION_FUND_FEES_VERSION1;
use crate::version::fee::vote_resolution_fund_fees::VoteResolutionFundFees;

/// Introduced in protocol version 14 (4.2).
pub const VOTE_RESOLUTION_FUND_FEES_VERSION2: VoteResolutionFundFees = VoteResolutionFundFees {
    contested_document_vote_resolution_fund_required_amount: 10_000_000_000, // 0.1 DASH
    // An application in a moderation election prefunds 5,000 masternode votes
    moderation_vote_resolution_fund_required_amount: 50_000_000_000, // 0.5 DASH
    ..VOTE_RESOLUTION_FUND_FEES_VERSION1
};
