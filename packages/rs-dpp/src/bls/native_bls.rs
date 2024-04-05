use crate::{BlsModule, ProtocolError, PublicKeyValidationError};
use anyhow::anyhow;
use bls_signatures::{PrivateKey, PublicKey};

#[derive(Default)]
pub struct NativeBlsModule;
impl BlsModule for NativeBlsModule {
    fn validate_public_key(&self, pk: &[u8]) -> Result<(), PublicKeyValidationError> {
        match PublicKey::from_bytes(pk) {
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
        let public_key = PublicKey::from_bytes(public_key).map_err(anyhow::Error::msg)?;
        let signature =
            bls_signatures::Signature::from_bytes(signature).map_err(anyhow::Error::msg)?;
        match public_key.verify(&signature, data) {
            true => Ok(true),
            // TODO change to specific error type
            false => Err(anyhow!("Verification failed").into()),
        }
    }

    fn private_key_to_public_key(&self, private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let fixed_len_key: [u8; 32] = private_key
            .try_into()
            .map_err(|_| anyhow!("the BLS private key must be 32 bytes long"))?;
        let pk = PrivateKey::from_bytes(&fixed_len_key, false).map_err(anyhow::Error::msg)?;
        let public_key = pk.g1_element().map_err(anyhow::Error::msg)?;
        let public_key_bytes = public_key.to_bytes().to_vec();
        Ok(public_key_bytes)
    }

    fn sign(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let fixed_len_key: [u8; 32] = private_key
            .try_into()
            .map_err(|_| anyhow!("the BLS private key must be 32 bytes long"))?;
        let pk = PrivateKey::from_bytes(&fixed_len_key, false).map_err(anyhow::Error::msg)?;
        Ok(pk.sign(data).to_bytes().to_vec())
    }
}
