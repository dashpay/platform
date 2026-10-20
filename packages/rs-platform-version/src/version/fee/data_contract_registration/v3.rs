use crate::version::fee::data_contract_registration::v2::FEE_DATA_CONTRACT_REGISTRATION_VERSION2;
use crate::version::fee::data_contract_registration::FeeDataContractRegistrationVersion;

/// Introduced in protocol version 14 (4.2): a token with a once-per-identity distribution pays
/// the same surcharge as one with a perpetual or pre-programmed distribution.
pub const FEE_DATA_CONTRACT_REGISTRATION_VERSION3: FeeDataContractRegistrationVersion =
    FeeDataContractRegistrationVersion {
        token_uses_once_per_identity_distribution_fee: 10_000_000_000, // 0.1 Dash
        ..FEE_DATA_CONTRACT_REGISTRATION_VERSION2
    };
