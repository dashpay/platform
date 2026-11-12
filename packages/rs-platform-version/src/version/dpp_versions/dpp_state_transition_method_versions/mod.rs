use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;
pub mod v3;

#[derive(Clone, Debug, Default)]
pub struct DPPStateTransitionMethodVersions {
    pub public_key_in_creation_methods: PublicKeyInCreationMethodVersions,
    /// `StateTransition::verify_identity_signed_signature`: 0 accepts a BLS12_381 signature
    /// that is well formed but does not verify, 1 refuses it.
    pub verify_identity_signed_signature: FeatureVersion,
    /// `dpp::util::ecdsa::verify_hash_signature`, the check of a compact recoverable
    /// ECDSA signature by recovering its public key and comparing the key's HASH160 (an
    /// ECDSA_HASH160 identity key, a P2PKH address witness, an asset lock's one-time key): 0
    /// accepts a signature whose S is in the upper half of the curve order, which any third party
    /// can derive from a valid one, 1 refuses it.
    pub verify_hash_signature: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct PublicKeyInCreationMethodVersions {
    pub from_public_key_signed_with_private_key: FeatureVersion,
    pub from_public_key_signed_external: FeatureVersion,
    pub hash: FeatureVersion,
    pub duplicated_key_ids_witness: FeatureVersion,
    pub duplicated_keys_witness: FeatureVersion,
    pub validate_identity_public_keys_structure: FeatureVersion,
}
