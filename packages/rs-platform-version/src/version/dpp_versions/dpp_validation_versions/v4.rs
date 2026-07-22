use crate::version::dpp_versions::dpp_validation_versions::{
    DPPValidationVersions, DataContractValidationVersions,
};

use super::v3::DPP_VALIDATION_VERSIONS_V3;

/// Protocol v13 validation versions.
///
/// v1 token-group validation covers every change-control rule family rather
/// than relying on the legacy hand-maintained subset.
pub const DPP_VALIDATION_VERSIONS_V4: DPPValidationVersions = DPPValidationVersions {
    data_contract: DataContractValidationVersions {
        validate_token_config_groups_exist: 1,
        ..DPP_VALIDATION_VERSIONS_V3.data_contract
    },
    ..DPP_VALIDATION_VERSIONS_V3
};
