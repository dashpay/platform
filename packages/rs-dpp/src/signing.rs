#[cfg(all(feature = "bls-signatures", feature = "message-signature-verification"))]
use crate::bls::{self, Signature};
#[cfg(any(
    feature = "message-signing",
    feature = "message-signature-verification"
))]
use crate::identity::KeyType;
#[cfg(any(
    feature = "message-signing",
    feature = "message-signature-verification"
))]
use crate::serialization::PlatformMessageSignable;
#[cfg(feature = "message-signature-verification")]
use crate::{
    consensus::signature::{
        BasicBLSError, BasicECDSAError, SignatureError, SignatureShouldNotBePresentError,
    },
    validation::SimpleConsensusValidationResult,
};
#[cfg(feature = "message-signing")]
use crate::{BlsModule, ProtocolError};
use anyhow::ensure;
use dashcore::crypto::ecdsa::{EcdsaError, EcdsaSignature};
use dashcore::sign_message;
use dashcore::signer;

// The low-S helpers below are needed by `address_funds` as well, which every feature cut
// compiles, so the module is always present and only this impl is gated.
#[cfg(any(
    feature = "message-signing",
    feature = "message-signature-verification"
))]
impl PlatformMessageSignable for &[u8] {
    #[cfg(feature = "message-signature-verification")]
    fn verify_signature(
        &self,
        public_key_type: KeyType,
        public_key_data: &[u8],
        signature: &[u8],
    ) -> SimpleConsensusValidationResult {
        let signable_data = self;
        match public_key_type {
            KeyType::ECDSA_SECP256K1 => {
                if let Err(e) =
                    verify_low_s_data_signature(signable_data, signature, public_key_data)
                {
                    // dbg!(format!(
                    //     "error with signature {} data {} public key {}",
                    //     hex::encode(signature),
                    //     hex::encode(signable_data),
                    //     hex::encode(public_key_data)
                    // ));
                    SimpleConsensusValidationResult::new_with_error(
                        SignatureError::BasicECDSAError(BasicECDSAError::new(e.to_string())).into(),
                    )
                } else {
                    SimpleConsensusValidationResult::default()
                }
            }
            KeyType::BLS12_381 => {
                let public_key = match bls::PublicKey::try_from(public_key_data) {
                    Ok(public_key) => public_key,
                    Err(e) => {
                        // dbg!(format!("bls public_key could not be recovered"));
                        return SimpleConsensusValidationResult::new_with_error(
                            SignatureError::BasicBLSError(BasicBLSError::new(e.to_string())).into(),
                        );
                    }
                };
                let signature_bytes: [u8; 96] = match signature.try_into() {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        return SimpleConsensusValidationResult::new_with_error(
                            SignatureError::BasicBLSError(BasicBLSError::new(format!(
                                "Signature was {} bytes, expected 96 bytes",
                                signature.len()
                            )))
                            .into(),
                        )
                    }
                };
                let signature = match Signature::from_compressed(&signature_bytes) {
                    Some(g2) => g2,
                    None => {
                        return SimpleConsensusValidationResult::new_with_error(
                            SignatureError::BasicBLSError(BasicBLSError::new("bls signature does not conform to proper bls signature serialization".to_string())).into(),
                        );
                    }
                };

                if signature.verify(&public_key, signable_data).is_err() {
                    SimpleConsensusValidationResult::new_with_error(
                        SignatureError::BasicBLSError(BasicBLSError::new(
                            "bls signature was incorrect".to_string(),
                        ))
                        .into(),
                    )
                } else {
                    SimpleConsensusValidationResult::default()
                }
            }
            KeyType::ECDSA_HASH160 => {
                if !signature.is_empty() {
                    SimpleConsensusValidationResult::new_with_error(
                        SignatureError::SignatureShouldNotBePresentError(
                            SignatureShouldNotBePresentError::new("ecdsa_hash160 keys should not have a signature as that would reveal the public key".to_string()),
                        ).into()
                    )
                } else {
                    SimpleConsensusValidationResult::default()
                }
            }
            KeyType::BIP13_SCRIPT_HASH => {
                if !signature.is_empty() {
                    SimpleConsensusValidationResult::new_with_error(
                        SignatureError::SignatureShouldNotBePresentError(
                            SignatureShouldNotBePresentError::new("script hash keys should not have a signature as that would reveal the script".to_string())
                        ).into())
                } else {
                    SimpleConsensusValidationResult::default()
                }
            }
            KeyType::EDDSA_25519_HASH160 => {
                if !signature.is_empty() {
                    SimpleConsensusValidationResult::new_with_error(
                        SignatureError::SignatureShouldNotBePresentError(
                            SignatureShouldNotBePresentError::new("eddsa hash 160 keys should not have a signature as that would reveal the script".to_string())
                        ).into()
                    )
                } else {
                    SimpleConsensusValidationResult::default()
                }
            }
        }
    }
    #[cfg(feature = "message-signing")]
    fn sign_by_private_key(
        &self,
        private_key: &[u8],
        key_type: KeyType,
        bls: &impl BlsModule,
    ) -> Result<Vec<u8>, ProtocolError> {
        match key_type {
            KeyType::BLS12_381 => Ok(bls.sign(self, private_key)?),

            // https://github.com/dashevo/platform/blob/9c8e6a3b6afbc330a6ab551a689de8ccd63f9120/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L169
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                let signature = signer::sign(self, private_key)?;
                Ok(signature.to_vec())
            }

            // the default behavior from
            // https://github.com/dashevo/platform/blob/6b02b26e5cd3a7c877c5fdfe40c4a4385a8dda15/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L187
            // is to return the error for the BIP13_SCRIPT_HASH
            KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                Err(ProtocolError::InvalidSigningKeyTypeError(format!(
                    "key type {} can not sign",
                    key_type
                )))
            }
        }
    }
}

/// Verifies a compact recoverable ECDSA signature over `data` under Platform's low-S rule.
///
/// Platform has always rejected a high-S signature, so the rule is consensus. Up to
/// rust-dashcore `aca02026` it held only because the curve backend refused high-S inside
/// `verify_data_signature`; from rust-dashcore#1214 on, that function normalizes S before
/// verifying, as Dash Core's `CPubKey::Verify` does. The rule is Platform's, not the
/// library's, so it is stated here. Signing never produces a high-S signature, so no honest
/// signer is affected.
///
/// The library verifies first, so a malformed signature or public key fails exactly as it
/// did before, with the library's own error. Only a signature the library accepts is then
/// held to the low-S rule; a high-S one fails with [`EcdsaError::VerifyFailed`], the error
/// the backend returned for it when it still refused high-S itself.
pub fn verify_low_s_data_signature(
    data: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<(), anyhow::Error> {
    signer::verify_data_signature(data, signature, public_key)?;
    let recoverable = sign_message::from_slice(signature)?;
    ensure!(
        is_low_s(&recoverable.signature()?),
        EcdsaError::VerifyFailed
    );
    Ok(())
}

/// Whether S is in the lower half of the curve order.
///
/// Asked of the curve backend by normalizing a copy: normalization is a no-op exactly when S
/// is already low. `dashcore::crypto::ecdsa::EcdsaSignature::is_low_s` says the same from
/// rust-dashcore#1214 on; this form compiles on the current pin as well.
pub fn is_low_s(signature: &EcdsaSignature) -> bool {
    let backend = secp256k1::ecdsa::Signature::from(*signature);
    let mut normalized = backend;
    normalized.normalize_s();
    normalized == backend
}
