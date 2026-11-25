use crate::version::dpp_versions::dpp_method_versions::v3::DPP_METHOD_VERSIONS_V3;
use crate::version::dpp_versions::dpp_method_versions::DPPMethodVersions;

/// DPP method versions 4. Introduced in protocol v15: `token_pool_paid_fee_binding` `None` →
/// `Some(0)` binds the fee bundle's commitment into the token bundle of the identity-less token
/// pool transitions. Everything else matches V3.
pub const DPP_METHOD_VERSIONS_V4: DPPMethodVersions = DPPMethodVersions {
    token_pool_paid_fee_binding: Some(0),
    ..DPP_METHOD_VERSIONS_V3
};
