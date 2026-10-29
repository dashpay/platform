use crate::consensus::signature::PublicKeyIsDisabledError;
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::{IdentityPublicKey, KeyType};
use crate::state_transition::errors::{PublicKeyMismatchError, StateTransitionIsNotSignedError};
use crate::state_transition::StateTransition;
use crate::{BlsModule, ProtocolError};
use platform_version::version::PlatformVersion;

impl StateTransition {
    /// Kept as is for replay: the BLS verdict is dropped, so a BLS12_381 signature that does
    /// not verify passes.
    #[inline(always)]
    pub(super) fn verify_identity_signed_signature_v0(
        &self,
        public_key: &IdentityPublicKey,
        bls: &impl BlsModule,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        self.identity_signed_signature_verdict_v0(public_key, bls, platform_version)
            .map(|_| ())
    }

    /// The checks of generation 0, returning whether the signature verifies. Today only the
    /// BLS12_381 arm returns `false`, for a signature that is well formed but does not verify;
    /// the ECDSA arms report a failed check as an error. Generation 1 runs the same checks.
    ///
    /// The ECDSA_HASH160 arm reads `platform_version` only to select `verify_hash_signature`:
    /// 0, the check this arm made before, at protocol versions 1 to 14, and 1, which also refuses
    /// a high-S signature, from 15.
    pub(super) fn identity_signed_signature_verdict_v0(
        &self,
        public_key: &IdentityPublicKey,
        bls: &impl BlsModule,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        // self.verify_public_key_level_and_purpose(public_key)?;
        if public_key.disabled_at().is_some() {
            return Err(ProtocolError::PublicKeyIsDisabledError(
                PublicKeyIsDisabledError::new(public_key.id()),
            ));
        }

        let Some(signature) = self.signature() else {
            return Err(ProtocolError::CorruptedCodeExecution("verifying identity signature for a state transition that doesn't use identity signatures".to_string()));
        };
        if signature.is_empty() {
            return Err(ProtocolError::StateTransitionIsNotSignedError(
                StateTransitionIsNotSignedError::new(self.clone()),
            ));
        }

        if self.signature_public_key_id() != Some(public_key.id()) {
            return Err(ProtocolError::PublicKeyMismatchError(
                PublicKeyMismatchError::new(public_key.clone()),
            ));
        }

        let public_key_bytes = public_key.data().as_slice();
        match public_key.key_type() {
            KeyType::ECDSA_HASH160 => self
                .verify_ecdsa_hash_160_signature_by_public_key_hash(
                    public_key_bytes,
                    platform_version,
                )
                .map(|()| true),

            KeyType::ECDSA_SECP256K1 => self
                .verify_ecdsa_signature_by_public_key(public_key_bytes)
                .map(|()| true),

            KeyType::BLS12_381 => self.verify_bls_signature_by_public_key(public_key_bytes, bls),

            // per https://github.com/dashevo/platform/pull/353, signing and verification is not supported
            KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => Ok(true),
        }
    }
}
