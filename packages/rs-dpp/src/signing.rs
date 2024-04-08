#[cfg(feature = "message-signature-verification")]
use crate::consensus::signature::{
    BasicBLSError, BasicECDSAError, SignatureError, SignatureShouldNotBePresentError,
};
use crate::identity::KeyType;
use crate::serialization::PlatformMessageSignable;
#[cfg(feature = "message-signature-verification")]
use crate::validation::SimpleConsensusValidationResult;
#[cfg(feature = "message-signing")]
use crate::{BlsModule, ProtocolError};
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
                    signer::verify_data_signature(signable_data, signature, public_key_data)
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
                let public_key = match bls_signatures::PublicKey::from_bytes(public_key_data) {
                    Ok(public_key) => public_key,
                    Err(e) => {
                        // dbg!(format!("bls public_key could not be recovered"));
                        return SimpleConsensusValidationResult::new_with_error(
                            SignatureError::BasicBLSError(BasicBLSError::new(e.to_string())).into(),
                        );
                    }
                };
                let signature = match bls_signatures::Signature::from_bytes(signature) {
                    Ok(public_key) => public_key,
                    Err(e) => {
                        // dbg!(format!("bls signature could not be recovered"));
                        return SimpleConsensusValidationResult::new_with_error(
                            SignatureError::BasicBLSError(BasicBLSError::new(e.to_string())).into(),
                        );
                    }
                };
                if !public_key.verify(&signature, signable_data) {
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
