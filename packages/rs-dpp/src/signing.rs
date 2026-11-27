#[cfg(feature = "bls-signatures")]
use crate::bls::{self, Signature};
use crate::identity::KeyType;
use crate::serialization::PlatformMessageSignable;
#[cfg(feature = "message-signature-verification")]
use crate::util::ecdsa;
#[cfg(any(
    feature = "message-signature-verification",
    feature = "message-signing"
))]
use crate::util::ml_dsa_65;
#[cfg(feature = "message-signature-verification")]
use crate::{
    consensus::signature::{
        BasicBLSError, BasicECDSAError, BasicMLDSAError, SignatureError,
        SignatureShouldNotBePresentError,
    },
    validation::SimpleConsensusValidationResult,
};
#[cfg(feature = "message-signing")]
use crate::{BlsModule, ProtocolError};
#[cfg(feature = "message-signing")]
use dashcore::signer;

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
                    ecdsa::verify_data_signature(signable_data, signature, public_key_data)
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
            KeyType::ML_DSA_65 => {
                if let Err(e) =
                    ml_dsa_65::verify_data_signature(signable_data, signature, public_key_data)
                {
                    SimpleConsensusValidationResult::new_with_error(
                        SignatureError::BasicMLDSAError(BasicMLDSAError::new(e.to_string())).into(),
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

            KeyType::ML_DSA_65 => {
                let seed: &[u8; 32] =
                    private_key
                        .try_into()
                        .map_err(|_| ProtocolError::PrivateKeySizeError {
                            got: private_key.len() as u32,
                        })?;
                ml_dsa_65::sign(self, seed).map_err(|e| ProtocolError::Generic(e.to_string()))
            }
        }
    }
}

#[cfg(all(
    test,
    feature = "message-signing",
    feature = "message-signature-verification"
))]
mod tests {
    use super::*;
    use crate::bls::native_bls::NativeBlsModule;
    use crate::consensus::ConsensusError;

    /// A proof of possession by an ML_DSA_65 key is its signature over the message itself.
    #[test]
    fn should_verify_a_message_signed_by_an_ml_dsa_65_key_and_refuse_any_other_signature() {
        let private_key = [5u8; 32];
        let public_key = ml_dsa_65::public_key_from_seed(&private_key);
        let message: &[u8] = b"the signable bytes of a transition";

        let signature = message
            .sign_by_private_key(&private_key, KeyType::ML_DSA_65, &NativeBlsModule)
            .expect("expected to sign");
        assert!(message
            .verify_signature(KeyType::ML_DSA_65, &public_key, &signature)
            .is_valid());

        let other_message: &[u8] = b"the signable bytes of another transition";
        let mut altered = signature.clone();
        altered[100] ^= 0x01;
        for (message, public_key, signature, label) in [
            (
                other_message,
                &public_key[..],
                &signature[..],
                "another message",
            ),
            (
                message,
                &public_key[..],
                &altered[..],
                "an altered signature",
            ),
            (message, &public_key[..], &[][..], "no signature"),
            (message, &public_key[..1951], &signature[..], "a short key"),
        ] {
            let result = message.verify_signature(KeyType::ML_DSA_65, public_key, signature);
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [ConsensusError::SignatureError(
                        SignatureError::BasicMLDSAError(_)
                    )]
                ),
                "{label}: {:?}",
                result.errors
            );
        }
    }

    #[test]
    fn should_refuse_to_sign_with_an_ml_dsa_65_private_key_that_is_not_32_bytes() {
        let message: &[u8] = b"message";
        assert!(matches!(
            message.sign_by_private_key(&[5u8; 31], KeyType::ML_DSA_65, &NativeBlsModule),
            Err(ProtocolError::PrivateKeySizeError { got: 31 })
        ));
    }
}
