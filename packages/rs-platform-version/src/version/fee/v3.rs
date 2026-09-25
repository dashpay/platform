use crate::version::fee::data_contract_registration::v3::FEE_DATA_CONTRACT_REGISTRATION_VERSION3;
use crate::version::fee::document_ttl::v1::FEE_DOCUMENT_TTL_VERSION1;
use crate::version::fee::v2::FEE_VERSION2;
use crate::version::fee::vote_resolution_fund_fees::v2::VOTE_RESOLUTION_FUND_FEES_VERSION2;
use crate::version::fee::FeeVersion;

/// Introduced in protocol version 14 (4.2).
pub const FEE_VERSION3: FeeVersion = FeeVersion {
    // Contested contributions use the active protocol, so unchanged storage
    // rates retain the historical fee_version_number inherited from FEE_VERSION2.
    vote_resolution_fund_fees: VOTE_RESOLUTION_FUND_FEES_VERSION2,
    // Tokens with a once-per-identity distribution pay a registration surcharge.
    data_contract_registration: FEE_DATA_CONTRACT_REGISTRATION_VERSION3,
    // Documents of a type declaring a `ttl` (new in protocol version 14) pay the
    // tiered lifetime price and prepay their deletion; FEE_VERSION2 carries the same
    // group, unread before 14, so the table is spelled out here where it takes effect.
    document_ttl: FEE_DOCUMENT_TTL_VERSION1,
    ..FEE_VERSION2
};
