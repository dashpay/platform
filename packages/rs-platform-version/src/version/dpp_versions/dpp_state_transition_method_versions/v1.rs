use crate::version::dpp_versions::dpp_state_transition_method_versions::{
    DPPStateTransitionMethodVersions, PublicKeyInCreationMethodVersions,
};

pub const STATE_TRANSITION_METHOD_VERSIONS_V1: DPPStateTransitionMethodVersions =
    DPPStateTransitionMethodVersions {
        public_key_in_creation_methods: PublicKeyInCreationMethodVersions {
            from_public_key_signed_with_private_key: 0,
            from_public_key_signed_external: 0,
            hash: 0,
            duplicated_key_ids_witness: 0,
            duplicated_keys_witness: 0,
            validate_identity_public_keys_structure: 0,
        },
    };
