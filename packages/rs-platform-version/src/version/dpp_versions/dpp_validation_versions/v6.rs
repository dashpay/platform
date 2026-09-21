use super::v5::DPP_VALIDATION_VERSIONS_V5;
use super::{DPPValidationVersions, DataContractValidationVersions};

/// Protocol 15 makes the token shielded pool opt-in immutable after creation.
pub const DPP_VALIDATION_VERSIONS_V6: DPPValidationVersions = DPPValidationVersions {
    data_contract: DataContractValidationVersions {
        validate_token_config_update: 1,
        ..DPP_VALIDATION_VERSIONS_V5.data_contract
    },
    ..DPP_VALIDATION_VERSIONS_V5
};
