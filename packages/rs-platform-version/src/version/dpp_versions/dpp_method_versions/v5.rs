use crate::version::dpp_versions::dpp_method_versions::v4::DPP_METHOD_VERSIONS_V4;
use crate::version::dpp_versions::dpp_method_versions::DPPMethodVersions;

/// DPP method versions 5. Introduced in protocol v16: `daily_withdrawal_limit` 2 → 3 floors the
/// relative daily limit at `SystemLimits::daily_withdrawal_limit_floor` instead of at one
/// maximal withdrawal, which protocol version 16 raises to 4000 Dash. The floor stays 500 Dash,
/// so the limit is the same number as before at every total. Everything else matches V4.
pub const DPP_METHOD_VERSIONS_V5: DPPMethodVersions = DPPMethodVersions {
    daily_withdrawal_limit: 3,
    ..DPP_METHOD_VERSIONS_V4
};
