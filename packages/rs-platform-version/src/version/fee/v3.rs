use crate::version::fee::v2::FEE_VERSION2;
use crate::version::fee::vote_resolution_fund_fees::v2::VOTE_RESOLUTION_FUND_FEES_VERSION2;
use crate::version::fee::FeeVersion;

/// Introduced in protocol version 14 (4.2).
pub const FEE_VERSION3: FeeVersion = FeeVersion {
    // Contested contributions use the active protocol, so unchanged storage
    // rates retain the historical fee_version_number inherited from FEE_VERSION2.
    vote_resolution_fund_fees: VOTE_RESOLUTION_FUND_FEES_VERSION2,
    ..FEE_VERSION2
};
