use crate::version::dpp_versions::dpp_state_transition_method_versions::{
    DPPStateTransitionMethodVersions, PublicKeyInCreationMethodVersions,
};

// PROTOCOL_VERSION_14: validate_identity_public_keys_structure v1 accepts the
// DIP-33 PAYMENT_SCAN / PAYMENT_SPEND key purposes (ECDSA_SECP256K1 only, at
// most one of each per transition). v1 of this struct (method version 0)
// remains for PROTOCOL_VERSION_13 chain replay, where those purposes are
// rejected.
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
