use crate::bls_signatures::{PublicKey, SecretKey, Signature};
use crate::{BlsModule, ProtocolError, PublicKeyValidationError};

#[derive(Default)]
pub struct NativeBlsModule;
impl BlsModule for NativeBlsModule {
    fn validate_public_key(&self, pk: &[u8]) -> Result<(), PublicKeyValidationError> {
        match PublicKey::try_from(pk) {
            Ok(_) => Ok(()),
            Err(e) => Err(PublicKeyValidationError::new(e.to_string())),
        }
    }

    fn verify_signature(
        &self,
        signature: &[u8],
        data: &[u8],
        public_key: &[u8],
    ) -> Result<bool, ProtocolError> {
        let public_key = PublicKey::try_from(public_key)?;
        let signature_96_bytes =
            signature
                .try_into()
                .map_err(|_| ProtocolError::BlsSignatureSizeError {
                    got: signature.len() as u32,
                })?;
        let Some(signature) = Signature::from_compressed(&signature_96_bytes) else {
            return Ok(false); // We should not error because the signature could be given by an invalid source
        };

        match signature.verify(&public_key, data) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn private_key_to_public_key(&self, private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let fixed_len_key: [u8; 32] =
            private_key
                .try_into()
                .map_err(|_| ProtocolError::PrivateKeySizeError {
                    got: private_key.len() as u32,
                })?;
        let pk = SecretKey::from_be_bytes(&fixed_len_key).ok_or(
            ProtocolError::InvalidBLSPrivateKeyError("key not valid".to_string()),
        )?;
        let public_key = pk.public_key();
        let public_key_bytes = public_key.to_bytes().to_vec();
        Ok(public_key_bytes)
    }

    fn sign(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let fixed_len_key: [u8; 32] =
            private_key
                .try_into()
                .map_err(|_| ProtocolError::PrivateKeySizeError {
                    got: private_key.len() as u32,
                })?;
        let pk = SecretKey::from_be_bytes(&fixed_len_key).ok_or(
            ProtocolError::InvalidBLSPrivateKeyError("key not valid".to_string()),
        )?;
        Ok(pk.sign(data)?.to_bytes().to_vec())
    }
}
