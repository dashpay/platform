use crate::identity::IdentityPublicKey;
use crate::state_transition::StateTransition;
use crate::{BlsModule, ProtocolError};
use platform_version::version::PlatformVersion;

mod v0;
mod v1;

impl StateTransition {
    /// Verifies the transition's signature with `public_key`, the identity key it names.
    ///
    /// Generation 0 (protocol versions 1 to 13) accepts a BLS12_381 signature that is well
    /// formed but does not verify: it fails only when the BLS module errors, never on its
    /// `Ok(false)`. Generation 1 refuses that signature too.
    ///
    /// An ECDSA_HASH160 key is checked by recovering the public key, through
    /// `verify_hash_signature`, which `platform_version` selects too: from protocol version 15 it
    /// refuses a high-S signature.
    pub fn verify_identity_signed_signature(
        &self,
        public_key: &IdentityPublicKey,
        bls: &impl BlsModule,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match platform_version
            .dpp
            .state_transition_method_versions
            .verify_identity_signed_signature
        {
            0 => self.verify_identity_signed_signature_v0(public_key, bls, platform_version),
            1 => self.verify_identity_signed_signature_v1(public_key, bls, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "StateTransition::verify_identity_signed_signature".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
