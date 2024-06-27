pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct FeeSignatureVersion {
    pub verify_signature_ecdsa_secp256k1: u64,
    pub verify_signature_bls12_381: u64,
    pub verify_signature_ecdsa_hash160: u64,
    pub verify_signature_bip13_script_hash: u64,
    pub verify_signature_eddsa25519_hash160: u64,
}

impl PartialEq for FeeSignatureVersion {
    fn eq(&self, other: &Self) -> bool {
        self.verify_signature_ecdsa_secp256k1 == other.verify_signature_ecdsa_secp256k1
            && self.verify_signature_bls12_381 == other.verify_signature_bls12_381
            && self.verify_signature_ecdsa_hash160 == other.verify_signature_ecdsa_hash160
            && self.verify_signature_bip13_script_hash == other.verify_signature_bip13_script_hash
            && self.verify_signature_eddsa25519_hash160 == other.verify_signature_eddsa25519_hash160
    }
}
