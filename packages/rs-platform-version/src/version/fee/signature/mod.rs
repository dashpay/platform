use bincode::{Decode, Encode};
use sha2::{Digest, Sha256};

pub mod v1;

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct FeeSignatureVersion {
    pub verify_signature_ecdsa_secp256k1: u64,
    pub verify_signature_bls12_381: u64,
    pub verify_signature_ecdsa_hash160: u64,
    pub verify_signature_bip13_script_hash: u64,
    pub verify_signature_eddsa25519_hash160: u64,
}

impl FeeSignatureVersion {
    pub(crate) fn to_hash(&self) -> u64 {
        let mut hasher = Sha256::new();
        Digest::update(
            &mut hasher,
            &self.verify_signature_ecdsa_secp256k1.to_be_bytes(),
        );
        Digest::update(&mut hasher, &self.verify_signature_bls12_381.to_be_bytes());
        Digest::update(
            &mut hasher,
            &self.verify_signature_ecdsa_hash160.to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self.verify_signature_bip13_script_hash.to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self.verify_signature_eddsa25519_hash160.to_be_bytes(),
        );

        let result = hasher.finalize();
        // Use the first 8 bytes of the hash as the u64 representation
        let hash_bytes: [u8; 8] = result[0..8].try_into().unwrap();
        u64::from_be_bytes(hash_bytes)
    }
}
