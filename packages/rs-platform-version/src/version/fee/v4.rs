use crate::version::fee::v3::FEE_VERSION3;
use crate::version::fee::vote_resolution_fund_fees::v3::VOTE_RESOLUTION_FUND_FEES_VERSION3;
use crate::version::fee::FeeVersion;

/// Introduced in protocol version 15 (5.1).
pub const FEE_VERSION4: FeeVersion = FeeVersion {
    // Decision proposals (new in protocol version 15) prefund their poll's votes. Storage rates
    // are unchanged, so the fee_version_number inherited from FEE_VERSION3 stays.
    vote_resolution_fund_fees: VOTE_RESOLUTION_FUND_FEES_VERSION3,
    ..FEE_VERSION3
};
