use crate::version::dpp_versions::dpp_state_transition_method_versions::{
    DPPStateTransitionMethodVersions, PublicKeyInCreationMethodVersions,
};

/// V2 is protocol version 14's table. `validate_identity_public_keys_structure` 0 -> 1: a public
/// key in creation may carry a budget or an expiry, which v1 only admits on AUTHENTICATION keys
/// below the MASTER security level and with a non-zero budget. V1 stays as is for replay.
pub const STATE_TRANSITION_METHOD_VERSIONS_V2: DPPStateTransitionMethodVersions =
    DPPStateTransitionMethodVersions {
        public_key_in_creation_methods: PublicKeyInCreationMethodVersions {
            from_public_key_signed_with_private_key: 0,
            from_public_key_signed_external: 0,
            hash: 0,
            duplicated_key_ids_witness: 0,
            duplicated_keys_witness: 0,
            validate_identity_public_keys_structure: 1,
        },
    };
