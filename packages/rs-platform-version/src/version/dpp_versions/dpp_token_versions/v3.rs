use crate::version::dpp_versions::dpp_token_versions::DPPTokenVersions;

/// Token structure versions for the 5.0 protocol version (17). Identical to
/// [`super::v2::TOKEN_VERSIONS_V2`]; the lifecycle record's structure version is 0 in every
/// table because the record is new here, and the field is listed so the table names every
/// structure the version builds.
pub const TOKEN_VERSIONS_V3: DPPTokenVersions = DPPTokenVersions {
    identity_token_info_default_structure_version: 0,
    identity_token_status_default_structure_version: 0,
    token_contract_info_default_structure_version: 0,
    token_config_update_action_id_version: 1,
    token_set_price_action_id_version: 1,
    contract_token_lifecycle_default_structure_version: 0,
};
