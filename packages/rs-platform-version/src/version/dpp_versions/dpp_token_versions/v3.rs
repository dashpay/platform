use crate::version::dpp_versions::dpp_token_versions::DPPTokenVersions;

/// NOTE: Not yet wired to any `PlatformVersion::PLATFORM_V*`. This constant sets
/// `distribution_function_evaluate_version: 1` (deterministic libm reward math), but
/// activation is deferred to a follow-up `PLATFORM_V13` PR. Until then it has no consumer.
pub const TOKEN_VERSIONS_V3: DPPTokenVersions = DPPTokenVersions {
    identity_token_info_default_structure_version: 0,
    identity_token_status_default_structure_version: 0,
    token_contract_info_default_structure_version: 0,
    token_config_update_action_id_version: 1,
    token_set_price_action_id_version: 1,
    distribution_function_evaluate_version: 1,
};
