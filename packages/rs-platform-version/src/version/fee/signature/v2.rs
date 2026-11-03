use crate::version::fee::signature::v1::FEE_SIGNATURE_VERSION1;
use crate::version::fee::signature::FeeSignatureVersion;

/// Introduced in protocol version 15 (5.1), with the `REDPALLAS_HECATE` key type.
pub const FEE_SIGNATURE_VERSION2: FeeSignatureVersion = FeeSignatureVersion {
    // A RedPallas verification is a two-scalar multiplication on Pallas plus a BLAKE2b
    // challenge: about three times a secp256k1 verification and a sixth of a BLS one.
    verify_signature_redpallas: 50_000,
    ..FEE_SIGNATURE_VERSION1
};
