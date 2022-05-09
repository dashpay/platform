use crate::{prelude::*, util::hash::hash};
use anyhow::anyhow;
use dashcore::{
    secp256k1::{
        ecdsa::{RecoverableSignature, Signature},
        Message, Secp256k1, Verification, VerifyOnly,
    },
    PrivateKey as ECDSAPrivateKey, PubkeyHash, PublicKey as ECDSAPublicKey,
};

/// verify the hash signature
pub fn verify_hash_signature(
    _hash: &[u8],
    _signature: &[u8],
    _public_key_hash: &[u8],
) -> Result<(), anyhow::Error> {
    unimplemented!()
}

/// verify the signature with public key
pub fn verify_signature(
    data: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<(), anyhow::Error> {
    let data_hash = hash(data);

    let msg = Message::from_slice(&data_hash).map_err(anyhow::Error::msg)?;
    let sig = Signature::from_compact(signature).map_err(anyhow::Error::msg)?;
    let pub_key = ECDSAPublicKey::from_slice(public_key).map_err(anyhow::Error::msg)?;
    let secp = dashcore::secp256k1::Secp256k1::new();

    secp.verify_ecdsa(&msg, &sig, &pub_key.inner)
        .map_err(anyhow::Error::msg)
}

pub fn sign(data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
    let data_hash = hash(data);
    let pk = dashcore::secp256k1::SecretKey::from_slice(private_key)
        .map_err(|e| anyhow!("Invalid ECDSA private key: {}", e))?;

    // TODO enable support for features in rust-dpp and allow to use global objects (SECP256K1)
    let secp = dashcore::secp256k1::Secp256k1::new();
    let msg = Message::from_slice(&data_hash).map_err(anyhow::Error::msg)?;

    // !TODO
    // !it generates the wrong signature, without the recovery bit (64bytes long) instead to 65long
    let signature = secp.sign_ecdsa(&msg, &pk);
    Ok(signature.serialize_compact().to_vec())
}

#[cfg(test)]
mod test {

    #[test]
    fn sign_and_verify_data() {
        let private_key_string = "032f352abd3fb62c3c5b543bb6eae515a1b99a202b367ab9c6e155ba689d0ff4";
        let public_key_string =
            "02716899be7008396a0b34dd49d9707b01e86265f9556ab54a493e712d42946e7a";
        // publicKey = new PublicKey(publicKeyString);
        // 	let private_key
    }
}
