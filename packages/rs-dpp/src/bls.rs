use crate::{ProtocolError, PublicKeyValidationError};
use anyhow::anyhow;
use bls_signatures::{verify_messages, PrivateKey, PublicKey, Serialize};
use std::convert::TryInto;

pub trait BlsModule {
    fn validate_public_key(&self, pk: &[u8]) -> Result<(), PublicKeyValidationError>;
    fn verify_signature(
        &self,
        signature: &[u8],
        data: &[u8],
        public_key: &[u8],
    ) -> Result<bool, ProtocolError>;
    fn private_key_to_public_key(&self, private_key: &[u8]) -> Result<Vec<u8>, ProtocolError>;
    fn sign(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, ProtocolError>;
}

// TODO: write tests for the native BLS module

#[derive(Default)]
#[cfg(not(target_arch = "wasm32"))]
pub struct NativeBlsModule;

#[cfg(not(target_arch = "wasm32"))]
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
        let pk = PublicKey::from_bytes(public_key).map_err(anyhow::Error::msg)?;
        let signature =
            bls_signatures::Signature::from_bytes(signature).map_err(anyhow::Error::msg)?;
        match verify_messages(&signature, &[&data], &[pk]) {
            true => Ok(true),
            // TODO change to specific error type
            false => Err(anyhow!("Verification failed").into()),
        }
    }

    fn private_key_to_public_key(&self, private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let fixed_len_key: [u8; 32] = private_key
            .try_into()
            .map_err(|_| anyhow!("the BLS private key must be 32 bytes long"))?;
        let pk = PrivateKey::from_bytes(&fixed_len_key).map_err(anyhow::Error::msg)?;
        let public_key = pk.public_key().as_bytes();
        Ok(public_key)
    }

    fn sign(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let fixed_len_key: [u8; 32] = private_key
            .try_into()
            .map_err(|_| anyhow!("the BLS private key must be 32 bytes long"))?;
        let pk = PrivateKey::from_bytes(&fixed_len_key).map_err(anyhow::Error::msg)?;
        Ok(pk.sign(data).as_bytes())
    }
}
