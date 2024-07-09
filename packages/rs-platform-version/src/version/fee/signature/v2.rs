use crate::version::fee::signature::FeeSignatureVersion;

pub const FEE_SIGNATURE_VERSION2: FeeSignatureVersion = FeeSignatureVersion {
    verify_signature_ecdsa_secp256k1: 300000,
    verify_signature_bls12_381: 600000,
    verify_signature_ecdsa_hash160: 400000,
    verify_signature_bip13_script_hash: 600000,
    verify_signature_eddsa25519_hash160: 300000,
};
