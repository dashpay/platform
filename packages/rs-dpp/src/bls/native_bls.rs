use crate::bls_signatures::{
    Bls12381G2Impl, Pairing, PublicKey, SecretKey, Signature, SignatureSchemes,
};
use crate::{BlsModule, ProtocolError, PublicKeyValidationError};
use anyhow::anyhow;

#[derive(Default)]
pub struct NativeBlsModule;
impl BlsModule for NativeBlsModule {
    fn validate_public_key(&self, pk: &[u8]) -> Result<(), PublicKeyValidationError> {
        match PublicKey::<Bls12381G2Impl>::try_from(pk) {
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
        let public_key =
            PublicKey::<Bls12381G2Impl>::try_from(public_key).map_err(anyhow::Error::msg)?;
        let signature_96_bytes = signature
            .try_into()
            .map_err(|_| anyhow!("signature wrong size"))?;
        let g2_element =
            <Bls12381G2Impl as Pairing>::Signature::from_compressed(&signature_96_bytes)
                .into_option()
                .ok_or(anyhow!("signature derivation failed"))?;

        let signature = Signature::Basic(g2_element);

        match signature.verify(&public_key, data) {
            Ok(_) => Ok(true),
            Err(_) => Err(anyhow!("Verification failed").into()),
        }
    }

    fn private_key_to_public_key(&self, private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let fixed_len_key: [u8; 32] = private_key
            .try_into()
            .map_err(|_| anyhow!("the BLS private key must be 32 bytes long"))?;
        let pk = SecretKey::<Bls12381G2Impl>::from_be_bytes(&fixed_len_key)
            .into_option()
            .ok_or(anyhow!("Incorrect Priv Key"))?;
        let public_key = pk.public_key();
        let public_key_bytes = public_key.0.to_compressed().to_vec();
        Ok(public_key_bytes)
    }

    fn sign(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let fixed_len_key: [u8; 32] = private_key
            .try_into()
            .map_err(|_| anyhow!("the BLS private key must be 32 bytes long"))?;
        let pk = SecretKey::<Bls12381G2Impl>::from_be_bytes(&fixed_len_key)
            .into_option()
            .ok_or(anyhow!("Incorrect Priv Key"))?;
        Ok(pk
            .sign(SignatureSchemes::Basic, data)?
            .as_raw_value()
            .to_compressed()
            .to_vec())
    }
}
