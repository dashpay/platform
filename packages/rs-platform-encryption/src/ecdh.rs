//! DIP-15 ECDH shared-secret derivation.

use secp256k1::{PublicKey, SecretKey};

/// Derive a shared secret key using ECDH as specified in DIP-15
///
/// This uses libsecp256k1_ecdh which computes: SHA256((y[31]&0x1|0x2) || x)
/// where (x, y) is the EC point result of scalar multiplication
///
/// # Arguments
/// * `private_key` - The private key for this side of the exchange
/// * `public_key` - The public key from the other party
///
/// # Returns
/// A 32-byte shared secret key
pub fn derive_shared_key_ecdh(private_key: &SecretKey, public_key: &PublicKey) -> [u8; 32] {
    use secp256k1::ecdh::SharedSecret;

    // Use secp256k1's built-in ECDH which matches libsecp256k1_ecdh
    // This computes SHA256((y[31]&0x1|0x2) || x) internally
    let shared_secret = SharedSecret::new(public_key, private_key);

    let mut key = [0u8; 32];
    key.copy_from_slice(shared_secret.as_ref());
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::rand::thread_rng;
    use secp256k1::Secp256k1;

    #[test]
    fn test_ecdh_key_derivation() {
        let secp = Secp256k1::new();

        // Generate two key pairs
        let (secret1, public1) = secp.generate_keypair(&mut thread_rng());
        let (secret2, public2) = secp.generate_keypair(&mut thread_rng());

        // Derive shared keys from both sides
        let shared1 = derive_shared_key_ecdh(&secret1, &public2);
        let shared2 = derive_shared_key_ecdh(&secret2, &public1);

        // Both sides should derive the same shared key
        assert_eq!(shared1, shared2);
    }

    /// Known-answer test for the ECDH shared-key convention. We had been
    /// trusting `SharedSecret::new` to compute `SHA256((y[31]&1|2) || x)` from
    /// a library comment, not bytes — a cross-impl mismatch with dashj would
    /// break contactRequest/contactInfo interop silently. This recomputes the
    /// shared key by hand for fixed keys and pins (a) symmetry `a·B == b·A` and
    /// (b) the exact compressed-y-prefix-‖-x preimage convention.
    #[test]
    fn ecdh_matches_sha256_y_parity_prefix_convention() {
        use secp256k1::{Scalar, Secp256k1};
        use sha2::{Digest, Sha256};

        let secp = Secp256k1::new();
        let priv_a = SecretKey::from_slice(&[0xC0u8; 32]).expect("valid scalar");
        let priv_b = SecretKey::from_slice(&[0x0Du8; 32]).expect("valid scalar");
        let pub_a = PublicKey::from_secret_key(&secp, &priv_a);
        let pub_b = PublicKey::from_secret_key(&secp, &priv_b);

        let ab = derive_shared_key_ecdh(&priv_a, &pub_b);
        let ba = derive_shared_key_ecdh(&priv_b, &pub_a);
        assert_eq!(ab, ba, "ECDH must be symmetric (a·B == b·A)");
        assert_eq!(ab, derive_shared_key_ecdh(&priv_a, &pub_b), "deterministic");

        // Recompute by hand: shared point P = a·B; shared key =
        // SHA256( (0x02 | (P.y & 1)) ‖ P.x ). Pins that it's the compressed-y
        // prefix + x, NOT x‖y or some other layout.
        let scalar_a = Scalar::from_be_bytes([0xC0u8; 32]).expect("scalar in range");
        let shared_point = pub_b.mul_tweak(&secp, &scalar_a).expect("point mul");
        let uncompressed = shared_point.serialize_uncompressed(); // 0x04 ‖ x(32) ‖ y(32)
        let prefix = 0x02u8 | (uncompressed[64] & 1); // y parity from the last y byte
        let mut preimage = Vec::with_capacity(33);
        preimage.push(prefix);
        preimage.extend_from_slice(&uncompressed[1..33]); // x
        let mut manual = [0u8; 32];
        manual.copy_from_slice(&Sha256::digest(&preimage));
        assert_eq!(ab, manual, "ECDH must be SHA256((y&1|2)‖x)");
    }
}
