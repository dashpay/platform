use crate::version::dpp_versions::dpp_token_versions::DPPTokenVersions;

/// Activates deterministic (libm) distribution function reward math. Wired to
/// `PLATFORM_V14`; `v14.rs` pins that v13 stays on version 0.
pub const TOKEN_VERSIONS_V3: DPPTokenVersions = DPPTokenVersions {
    identity_token_info_default_structure_version: 0,
    identity_token_status_default_structure_version: 0,
    token_contract_info_default_structure_version: 0,
    token_config_update_action_id_version: 1,
    token_set_price_action_id_version: 1,
    distribution_function_evaluate_version: 1,
};
