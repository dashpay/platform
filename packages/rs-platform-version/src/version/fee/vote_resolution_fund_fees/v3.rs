use crate::version::fee::vote_resolution_fund_fees::v2::VOTE_RESOLUTION_FUND_FEES_VERSION2;
use crate::version::fee::vote_resolution_fund_fees::VoteResolutionFundFees;

/// Introduced in protocol version 15 (5.1).
pub const VOTE_RESOLUTION_FUND_FEES_VERSION3: VoteResolutionFundFees = VoteResolutionFundFees {
    // Opening a decision proposal prefunds 25,000 masternode votes on its poll, more than every
    // masternode casting all of its votes on it
    decision_proposal_fund_required_amount: 50_000_000_000, // 0.5 DASH
    ..VOTE_RESOLUTION_FUND_FEES_VERSION2
};
