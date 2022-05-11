use std::convert::TryInto;

use crate::util::hash::hash;
use anyhow::{anyhow, bail};
use dashcore::{
    secp256k1::{
        ecdsa::{RecoverableSignature, RecoveryId},
        Message,
    },
    PublicKey as ECDSAPublicKey,
};

/// verifies the ECDSA signature
/// The provided signature must be recoverable. Which means: it must contain the recovery byte as a prefix
pub fn verify_data_signature(
    data: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<(), anyhow::Error> {
    let data_hash = hash(data);

    let msg = Message::from_slice(&data_hash).map_err(anyhow::Error::msg)?;
    let sig: RecoverableSignature = RecoverableSignature::from_compact_signature(signature)?;

    let pub_key = ECDSAPublicKey::from_slice(public_key).map_err(anyhow::Error::msg)?;
    let secp = dashcore::secp256k1::Secp256k1::new();

    secp.verify_ecdsa(&msg, &sig.to_standard(), &pub_key.inner)
        .map_err(anyhow::Error::msg)
}

/// verifies the the hash signature. From provided signature and hash recovers the public key
/// and compares with the provided one
pub fn verify_hash_signature(
    data_hash: &[u8],
    data_signature: &[u8],
    public_key_id: &[u8],
) -> Result<(), anyhow::Error> {
    let signature: RecoverableSignature =
        RecoverableSignature::from_compact_signature(data_signature)?;

    let secp = dashcore::secp256k1::Secp256k1::new();
    let msg = Message::from_slice(data_hash).map_err(anyhow::Error::msg)?;
    let recovered_public_key = secp
        .recover_ecdsa(&msg, &signature)
        .map_err(anyhow::Error::msg)?;

    let public_key = ECDSAPublicKey::from_slice(public_key_id).map_err(anyhow::Error::msg)?;
    let are_equal = match public_key.compressed {
        true => public_key.to_bytes() == recovered_public_key.serialize(),
        false => public_key.to_bytes() == recovered_public_key.serialize_uncompressed(),
    };

    if are_equal {
        Ok(())
    } else {
        bail!("the signature isn't valid")
    }
}

/// sign and get the ECDSA signature
pub fn sign(data: &[u8], private_key: &[u8]) -> Result<[u8; 65], anyhow::Error> {
    let data_hash = hash(data);
    sign_hash(&data_hash, private_key)
}

pub fn sign_hash(data_hash: &[u8], private_key: &[u8]) -> Result<[u8; 65], anyhow::Error> {
    let pk = dashcore::secp256k1::SecretKey::from_slice(private_key)
        .map_err(|e| anyhow!("Invalid ECDSA private key: {}", e))?;

    // TODO enable support for features in rust-dpp and allow to use global objects (SECP256K1)
    let secp = dashcore::secp256k1::Secp256k1::new();
    let msg = Message::from_slice(data_hash).map_err(anyhow::Error::msg)?;

    let signature = secp
        .sign_ecdsa_recoverable(&msg, &pk)
        // TODO the compression flag should be obtained from the private key type
        .to_compact_signature(true);
    Ok(signature)
}

pub trait CompactSignature
where
    Self: Sized,
{
    /// Converts the Signature with Recovery byte to the compact format where
    /// the first byte of signature is occupied by the recovery byte
    fn to_compact_signature(&self, is_compressed: bool) -> [u8; 65];
    /// Creates the Self from compacted version of signature
    fn from_compact_signature(signature: impl AsRef<[u8]>) -> Result<Self, anyhow::Error>;
}

impl CompactSignature for RecoverableSignature {
    fn from_compact_signature(signature: impl AsRef<[u8]>) -> Result<Self, anyhow::Error> {
        if signature.as_ref().len() != 65 {
            bail!("the signature must be 65 bytes length")
        }

        let recovery_byte = signature.as_ref()[0];
        let number = u8::from_be(recovery_byte) as i32;
        let mut i = number - 27 - 4;
        if i < 0 {
            i += 4;
        }
        if !((i == 0) || (i == 1) || (i == 2) || (i == 3)) {
            bail!("the recovery number must be between 0..4, got: '{}'", i);
        }

        RecoverableSignature::from_compact(
            &signature.as_ref()[1..],
            RecoveryId::from_i32(i).unwrap(),
        )
        .map_err(anyhow::Error::msg)
    }

    fn to_compact_signature(&self, is_compressed: bool) -> [u8; 65] {
        let (recovery_byte, signature) = self.serialize_compact();
        let mut val = recovery_byte.to_i32() + 27 + 4;
        if !is_compressed {
            val -= 4;
        }
        let prefix = val.to_le_bytes()[0];
        let compact_signature = [&[prefix], signature.as_slice()].concat();
        compact_signature.try_into().unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use dashcore::PublicKey;

    #[test]
    fn sign_and_verify_data() {
        let private_key_string = "032f352abd3fb62c3c5b543bb6eae515a1b99a202b367ab9c6e155ba689d0ff4";
        // the compressed version of Public key
        let public_key_string =
            "02716899be7008396a0b34dd49d9707b01e86265f9556ab54a493e712d42946e7a";
        let data = "fafafa";

        let private_key_bytes = hex::decode(private_key_string).unwrap();
        let public_key_bytes = hex::decode(public_key_string).unwrap();
        let data_bytes = hex::decode(data).unwrap();

        let signature = sign(&data_bytes, &private_key_bytes).unwrap();
        let mut pk = PublicKey::from_slice(&public_key_bytes).unwrap();
        pk.compressed = false;

        verify_data_signature(&data_bytes, &signature, &pk.to_bytes()).unwrap();
    }
}
