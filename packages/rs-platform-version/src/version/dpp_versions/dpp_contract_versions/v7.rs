use super::v6::CONTRACT_VERSIONS_V6;
use super::{DPPContractVersions, TokenVersions};
use versioned_feature_core::FeatureVersionBounds;

/// Protocol 15 admits token configuration format 1, retaining all protocol 14 contract rules.
pub const CONTRACT_VERSIONS_V7: DPPContractVersions = DPPContractVersions {
    token_versions: TokenVersions {
        token_configuration_format: FeatureVersionBounds {
            min_version: 0,
            max_version: 1,
            default_current_version: 0,
        },
        ..CONTRACT_VERSIONS_V6.token_versions
    },
    ..CONTRACT_VERSIONS_V6
};
