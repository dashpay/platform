//! Basic-scheme BLS checks with the `blst` backend the SDK already ships.
use crate::wire::{Hash, Result};
use crate::Error;
use blst::min_pk::{PublicKey, Signature};
use blst::BLST_ERROR;

const BLS_DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";

/// Decode a public key that is canonical, in the prime-order subgroup and not
/// the point at infinity.
pub(crate) fn public_key(key: &[u8; 48]) -> Result<PublicKey> {
    let pk = PublicKey::key_validate(key).map_err(|_| Error("BLS public key"))?;
    if pk.compress() != *key {
        return Err(Error("BLS noncanonical public key"));
    }
    Ok(pk)
}

/// Accept the signature encodings Core deserializes: all zeros (a null
/// signature), or a canonical compressed point in the subgroup other than the
/// point at infinity, which Core resets and then rejects as malleable.
///
/// Core also accepts, as null, bytes that are invalid in the Basic scheme but
/// valid in the legacy one. No block can carry those where this is used (Core
/// checks mined commitments' signatures, and a coinbase may not drop a
/// ChainLock its predecessor carried), so they are rejected here.
pub(crate) fn check_signature_encoding(signature: &[u8; 96]) -> Result<()> {
    if signature.iter().all(|byte| *byte == 0) {
        return Ok(());
    }
    let sig =
        Signature::sig_validate(signature, true).map_err(|_| Error("BLS signature encoding"))?;
    if sig.compress() != *signature {
        return Err(Error("BLS signature encoding"));
    }
    Ok(())
}

/// Verify a Basic-scheme signature over a 32-byte message hash.
pub(crate) fn verify(key: &[u8; 48], signature: &[u8; 96], message: &Hash) -> Result<()> {
    let pk = public_key(key)?;
    let sig = Signature::sig_validate(signature, true).map_err(|_| Error("BLS signature"))?;
    if sig.compress() != *signature {
        return Err(Error("BLS noncanonical signature"));
    }
    // Both inputs were validated above, so verification need not repeat it.
    if sig.verify(false, message, BLS_DST, &[], &pk, false) != BLST_ERROR::BLST_SUCCESS {
        return Err(Error("BLS verification"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use blst::min_pk::SecretKey;

    #[test]
    fn should_reject_infinity_noncanonical_and_torsion_encodings() {
        let secret = SecretKey::key_gen(&[42u8; 32], &[]).unwrap();
        let key = secret.sk_to_pk().compress();
        let message = [0u8; 32];
        let signature = secret.sign(&message, BLS_DST, &[]).compress();
        assert_eq!(verify(&key, &signature, &message), Ok(()));
        assert!(verify(&key, &signature, &[1u8; 32]).is_err());

        let mut infinity_key = [0u8; 48];
        infinity_key[0] = 0xc0;
        let mut infinity_signature = [0u8; 96];
        infinity_signature[0] = 0xc0;
        assert!(verify(&infinity_key, &infinity_signature, &message).is_err());
        assert!(verify(&key, &infinity_signature, &message).is_err());
        assert!(verify(&infinity_key, &signature, &message).is_err());

        // (0, 2) lies on G1 but has order 3, outside the prime-order subgroup.
        let mut torsion_key = [0u8; 48];
        torsion_key[0] = 0x80;
        assert!(verify(&torsion_key, &signature, &message).is_err());

        let mut field_modulus: [u8; 48] = hex::decode(
            "1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab",
        )
        .unwrap()
        .try_into()
        .unwrap();
        field_modulus[0] |= 0x80;
        assert!(verify(&field_modulus, &signature, &message).is_err());
        assert!(verify(&[0xff; 48], &signature, &message).is_err());
        assert!(verify(&key, &[0xff; 96], &message).is_err());
    }

    #[test]
    fn should_accept_only_signature_encodings_core_deserializes() {
        let secret = SecretKey::key_gen(&[7u8; 32], &[]).unwrap();
        let signature = secret.sign(&[0u8; 32], BLS_DST, &[]).compress();
        assert_eq!(check_signature_encoding(&signature), Ok(()));
        assert_eq!(check_signature_encoding(&[0; 96]), Ok(()));
        // Core resets the identity element and then rejects it as malleable.
        let mut infinity = [0u8; 96];
        infinity[0] = 0xc0;
        assert!(check_signature_encoding(&infinity).is_err());
        // Uncompressed flag, garbage, and a nonzero infinity payload.
        let mut uncompressed = signature;
        uncompressed[0] &= 0x7f;
        assert!(check_signature_encoding(&uncompressed).is_err());
        assert!(check_signature_encoding(&[0xff; 96]).is_err());
        let mut dirty_infinity = infinity;
        dirty_infinity[95] = 1;
        assert!(check_signature_encoding(&dirty_infinity).is_err());
    }
}
