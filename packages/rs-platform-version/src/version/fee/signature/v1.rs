use crate::version::fee::signature::FeeSignatureVersion;

pub const FEE_SIGNATURE_VERSION1: FeeSignatureVersion = FeeSignatureVersion {
    verify_signature_ecdsa_secp256k1: 3000,
    verify_signature_bls12_381: 6000,
    verify_signature_ecdsa_hash160: 4000,
    verify_signature_bip13_script_hash: 6000,
    verify_signature_eddsa25519_hash160: 3000,
};
