use crate::version::fee::signature::FeeSignatureVersion;

pub const FEE_SIGNATURE_VERSION1: FeeSignatureVersion = FeeSignatureVersion {
    verify_signature_ecdsa_secp256k1: 15_000,
    verify_signature_bls12_381: 300_000,
    verify_signature_ecdsa_hash160: 15_500,
    verify_signature_bip13_script_hash: 300_000,
    verify_signature_eddsa25519_hash160: 3_500,
    // Reading the 1,952 byte key and verifying takes about 6.5 times the processor time of an
    // ECDSA_SECP256K1 check (90 against 14 microseconds on the machine that measured both),
    // rounded up. Unread before protocol version 16, where no ML_DSA_65 key exists.
    verify_signature_ml_dsa_65: 100_000,
};
