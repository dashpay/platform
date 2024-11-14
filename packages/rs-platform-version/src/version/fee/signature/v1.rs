use crate::version::fee::signature::FeeSignatureVersion;

pub const FEE_SIGNATURE_VERSION1: FeeSignatureVersion = FeeSignatureVersion {
    verify_signature_ecdsa_secp256k1: 15_000,
    verify_signature_bls12_381: 300_000,
    verify_signature_ecdsa_hash160: 15_500,
    verify_signature_bip13_script_hash: 300_000,
    verify_signature_eddsa25519_hash160: 3_500,
};
