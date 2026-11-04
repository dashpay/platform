use crate::version::dpp_versions::dpp_state_transition_method_versions::{
    DPPStateTransitionMethodVersions, PublicKeyInCreationMethodVersions,
};

/// V3 is protocol version 15's table. `validate_identity_public_keys_structure` 1 -> 2: a public
/// key in creation may be a Hecate signer key (`KeyType::REDPALLAS_HECATE`,
/// `Purpose::HECATE_SIGNER`, MEDIUM only), at most one per transition. Its data is left to its
/// proof of possession. V2 stays as is for replay.
pub const STATE_TRANSITION_METHOD_VERSIONS_V3: DPPStateTransitionMethodVersions =
    DPPStateTransitionMethodVersions {
        public_key_in_creation_methods: PublicKeyInCreationMethodVersions {
            from_public_key_signed_with_private_key: 0,
            from_public_key_signed_external: 0,
            hash: 0,
            duplicated_key_ids_witness: 0,
            duplicated_keys_witness: 0,
            validate_identity_public_keys_structure: 2,
        },
        verify_identity_signed_signature: 1,
    };
