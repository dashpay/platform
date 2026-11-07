use crate::version::fee::private_document::v1::FEE_PRIVATE_DOCUMENT_VERSION1;
use crate::version::fee::signature::v2::FEE_SIGNATURE_VERSION2;
use crate::version::fee::v3::FEE_VERSION3;
use crate::version::fee::vote_resolution_fund_fees::v3::VOTE_RESOLUTION_FUND_FEES_VERSION3;
use crate::version::fee::FeeVersion;

/// Introduced in protocol version 15 (5.1).
pub const FEE_VERSION4: FeeVersion = FeeVersion {
    // Decision proposals (new in protocol version 15) prefund their poll's votes. Storage rates
    // are unchanged, so the fee_version_number inherited from FEE_VERSION3 stays.
    vote_resolution_fund_fees: VOTE_RESOLUTION_FUND_FEES_VERSION3,
    // Each private document entry (new in protocol version 15) pays its storage and a flat
    // 10,000,000 credits; the earlier schedules carry the same group, unread before 15, so the
    // table is spelled out here where it takes effect.
    private_document: FEE_PRIVATE_DOCUMENT_VERSION1,
    // REDPALLAS_HECATE keys (new in protocol version 15) are priced to verify. The number stays
    // too: the fee history serves no RedPallas cost (no `KnownCostItem` reads it, and verification
    // reads the active schedule), which dpp's
    // `should_serve_every_known_cost_of_a_protocol_version_from_its_fee_version_number` checks.
    signature: FEE_SIGNATURE_VERSION2,
    ..FEE_VERSION3
};
