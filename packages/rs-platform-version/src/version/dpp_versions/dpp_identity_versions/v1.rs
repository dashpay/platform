use crate::version::dpp_versions::dpp_identity_versions::{
    DPPIdentityVersions, IdentityKeyTypeMethodVersions,
};

pub const IDENTITY_VERSIONS_V1: DPPIdentityVersions = DPPIdentityVersions {
    identity_structure_version: 0,
    identity_key_structure_version: 0,
    identity_key_type_method_versions: IdentityKeyTypeMethodVersions {
        random_public_key_data: 0,
        random_public_and_private_key_data: 0,
    },
};
