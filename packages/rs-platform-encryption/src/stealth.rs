//! DIP-33 stealth one-time key derivation.
//!
//! Implements the curve math of DIP-33 "One-Time Address Derivation": from a
//! recipient's published scan/spend key pair, a payer derives a fresh one-time
//! public key `P_n = B_spend + t_n·G` that only the recipient can recognize
//! (with the scan secret) and spend (with the spend secret). The same math
//! serves both transparent rails — Dash Core P2PKH outputs and Platform
//! payment addresses — separated by a rail domain byte inside the tweak hash,
//! so the two rails' one-time key spaces can never collide even under
//! (faulty) ephemeral-key reuse.
//!
//! This module is pure curve-and-hash math: no derivation paths, no address
//! encoding, no wallet state. Address formation (Core `Address::p2pkh` /
//! `PlatformAddress::P2pkh`) and the DIP-9 feature-33' key derivation live in
//! `platform-wallet`.

use secp256k1::{PublicKey, Scalar, Secp256k1, SecretKey, Verification};
use sha2::{Digest, Sha256};

use crate::error::CryptoError;

/// Domain-separation tag for the one-time tweak hash (DIP-33).
const STEALTH_TAG: &[u8] = b"DashPay/Stealth/v1";

/// The secp256k1 group order `n`, big-endian.
const CURVE_ORDER_BE: [u8; 32] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE,
    0xBA, 0xAE, 0xDC, 0xE6, 0xAF, 0x48, 0xA0, 0x3B, 0xBF, 0xD2, 0x5E, 0x8C, 0xD0, 0x36, 0x41, 0x41,
];

/// Payment rail domain byte mixed into the one-time tweak (DIP-33).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StealthRail {
    /// Dash Core chain P2PKH outputs.
    Core = 0x00,
    /// Platform payment addresses (DIP-17/DIP-18).
    Platform = 0x02,
}

/// Compute the Diffie–Hellman shared point for stealth derivation.
///
/// Payer side: `secret_half = r` (ephemeral), `public_half = B_scan`.
/// Recipient side: `secret_half = b_scan`, `public_half = R`.
/// Both produce the same point `S = r·B_scan = b_scan·R`.
pub fn stealth_shared_point<C: Verification>(
    secp: &Secp256k1<C>,
    secret_half: &SecretKey,
    public_half: &PublicKey,
) -> Result<PublicKey, CryptoError> {
    public_half
        .mul_tweak(secp, &Scalar::from(*secret_half))
        .map_err(|_| CryptoError::StealthPointOperation)
}

/// Compute the one-time tweak `t_n` for output `n` on `rail` (DIP-33):
///
/// `t_n = int_be(SHA256("DashPay/Stealth/v1" || ser(S) || ser(R) || rail || LE32(n))) mod order`
///
/// where `S` is the shared point and `R` the payer's ephemeral public key.
/// Errors with [`CryptoError::ZeroStealthTweak`] on the negligible zero case,
/// in which the payer must restart with a fresh ephemeral key.
pub fn one_time_tweak(
    shared_point: &PublicKey,
    ephemeral_public: &PublicKey,
    rail: StealthRail,
    n: u32,
) -> Result<Scalar, CryptoError> {
    let mut hasher = Sha256::new();
    hasher.update(STEALTH_TAG);
    hasher.update(shared_point.serialize());
    hasher.update(ephemeral_public.serialize());
    hasher.update([rail as u8]);
    hasher.update(n.to_le_bytes());
    let digest: [u8; 32] = hasher.finalize().into();

    let reduced = reduce_mod_order(digest);
    if reduced == [0u8; 32] {
        return Err(CryptoError::ZeroStealthTweak);
    }
    // In range by construction after reduction.
    Scalar::from_be_bytes(reduced).map_err(|_| CryptoError::StealthPointOperation)
}

/// Derive the one-time public key `P_n = B_spend + t_n·G`.
///
/// Callable by the payer (who computed `S` from `r` and `B_scan`) and by the
/// recipient or a watch service (who computed `S` from `b_scan` and `R`);
/// neither needs the spend secret.
pub fn one_time_public_key<C: Verification>(
    secp: &Secp256k1<C>,
    spend_public: &PublicKey,
    shared_point: &PublicKey,
    ephemeral_public: &PublicKey,
    rail: StealthRail,
    n: u32,
) -> Result<PublicKey, CryptoError> {
    let tweak = one_time_tweak(shared_point, ephemeral_public, rail, n)?;
    spend_public
        .add_exp_tweak(secp, &tweak)
        .map_err(|_| CryptoError::StealthPointOperation)
}

/// Derive the one-time secret key `p_n = b_spend + t_n (mod order)`.
///
/// Requires the spend secret; this is the only stealth operation a scan-only
/// watch service cannot perform.
pub fn one_time_secret_key(
    spend_secret: &SecretKey,
    shared_point: &PublicKey,
    ephemeral_public: &PublicKey,
    rail: StealthRail,
    n: u32,
) -> Result<SecretKey, CryptoError> {
    let tweak = one_time_tweak(shared_point, ephemeral_public, rail, n)?;
    spend_secret
        .add_tweak(&tweak)
        .map_err(|_| CryptoError::StealthPointOperation)
}

/// Reduce a 256-bit big-endian value modulo the curve order.
///
/// Any 256-bit value is below `2n` (the order's top bit is set), so a single
/// conditional subtraction is an exact reduction.
fn reduce_mod_order(bytes: [u8; 32]) -> [u8; 32] {
    if !ge_order(&bytes) {
        return bytes;
    }
    let mut out = [0u8; 32];
    let mut borrow = 0u16;
    for i in (0..32).rev() {
        let lhs = bytes[i] as u16;
        let rhs = CURVE_ORDER_BE[i] as u16 + borrow;
        if lhs >= rhs {
            out[i] = (lhs - rhs) as u8;
            borrow = 0;
        } else {
            out[i] = (lhs + 0x100 - rhs) as u8;
            borrow = 1;
        }
    }
    out
}

/// Big-endian comparison: `bytes >= CURVE_ORDER_BE`.
fn ge_order(bytes: &[u8; 32]) -> bool {
    for i in 0..32 {
        match bytes[i].cmp(&CURVE_ORDER_BE[i]) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sk(hex_str: &str) -> SecretKey {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(hex_str, &mut bytes).expect("valid hex");
        SecretKey::from_slice(&bytes).expect("valid scalar")
    }

    /// DIP-33 test vector inputs (test-only scalars).
    const B_SCAN: &str = "61aafd85dbca17133515038343b05ed2019ca465d1ba93dcbffbf9534d2f436c";
    const B_SPEND: &str = "8f54f1cfa054739a1d4a87847bdf6a024caf78ff4b2c91aeb75cb843280957ff";
    const R_EPHEMERAL: &str = "37ca08877b0beea1b1e7655649274689e89944edae537ecad42764e59ac31e3e";

    /// Known-answer test pinning the full DIP-33 vector table: shared point,
    /// tweaks, one-time public keys, and one-time secret keys for both rails.
    /// A mismatch here means we broke interop with every other implementation
    /// of the DIP, not just our own round-trip.
    #[test]
    fn dip33_known_answer_vectors() {
        let secp = Secp256k1::new();
        let b_scan = sk(B_SCAN);
        let b_spend = sk(B_SPEND);
        let r = sk(R_EPHEMERAL);

        let scan_pub = PublicKey::from_secret_key(&secp, &b_scan);
        let spend_pub = PublicKey::from_secret_key(&secp, &b_spend);
        let ephemeral_pub = PublicKey::from_secret_key(&secp, &r);

        assert_eq!(
            hex::encode(scan_pub.serialize()),
            "0218ee61cb2070d8c63456b2acd9cb0243e4a4ad0791c272e79c0eb8de88284448"
        );
        assert_eq!(
            hex::encode(spend_pub.serialize()),
            "0397b4690ee29e51d7da3186a683a563394981832f8860e405f8827bfd5dfbd7f6"
        );
        assert_eq!(
            hex::encode(ephemeral_pub.serialize()),
            "029d683d939d3bb5527ccc96d7ecd4e77274a530b4496fed79f08db99c8fe6e44e"
        );

        // Diffie–Hellman symmetry: payer and recipient reach the same point.
        let shared_payer = stealth_shared_point(&secp, &r, &scan_pub).expect("payer S");
        let shared_recipient =
            stealth_shared_point(&secp, &b_scan, &ephemeral_pub).expect("recipient S");
        assert_eq!(shared_payer, shared_recipient);
        assert_eq!(
            hex::encode(shared_payer.serialize()),
            "031218547890897c32cbc802902b8b8257cbcbd3cee2f9c393b7e4a74cb635b36e"
        );

        // (rail, n, expected t_n, expected P_n, expected p_n)
        let vectors: [(StealthRail, u32, &str, &str, &str); 3] = [
            (
                StealthRail::Core,
                0,
                "2fe3ccd4f560aadda6cedb4f4b9e0927ecbdf49c4864ba0bfbdb174a80e94cdf",
                "029459124e206a8113bdb4c862794788380bae4b1d8bac57e561a6c88eb4eb96f1",
                "bf38bea495b51e77c41962d3c77d732a396d6d9b93914bbab337cf8da8f2a4de",
            ),
            (
                StealthRail::Core,
                1,
                "e0867aad0724465861061aa9d89bf7cb71056bad6888b119ca5b4579f3663cf8",
                "03356bc4f9ef4256c306872ab072bdef1e317396f2ae04730c6f252e9a02a2c039",
                "6fdb6c7ca778b9f27e50a22e547b61cf030607c6046ca28cc1e59f304b3953b6",
            ),
            (
                StealthRail::Platform,
                0,
                "5bba1d655f05b3b9b91561f818c3d8091863148ed28f129b10e959e8a0847756",
                "03b45786021b88c3b0e8730a7164630e75eea49b13fe1fd304c97075103f6fa40e",
                "eb0f0f34ff5a2753d65fe97c94a3420b65128d8e1dbba449c846122bc88dcf55",
            ),
        ];

        for (rail, n, expected_t, expected_pub, expected_secret) in vectors {
            let tweak = one_time_tweak(&shared_payer, &ephemeral_pub, rail, n).expect("tweak");
            assert_eq!(hex::encode(tweak.to_be_bytes()), expected_t);

            let one_time_pub =
                one_time_public_key(&secp, &spend_pub, &shared_payer, &ephemeral_pub, rail, n)
                    .expect("one-time public key");
            assert_eq!(hex::encode(one_time_pub.serialize()), expected_pub);

            let one_time_secret =
                one_time_secret_key(&b_spend, &shared_payer, &ephemeral_pub, rail, n)
                    .expect("one-time secret key");
            assert_eq!(hex::encode(one_time_secret.secret_bytes()), expected_secret);

            // Spend-key consistency: p_n·G == P_n.
            assert_eq!(
                PublicKey::from_secret_key(&secp, &one_time_secret),
                one_time_pub,
                "one-time secret must control the one-time public key"
            );
        }
    }

    /// The rail byte must domain-separate the two rails: identical inputs on
    /// different rails may never produce the same tweak.
    #[test]
    fn rails_are_domain_separated() {
        let secp = Secp256k1::new();
        let b_scan = sk(B_SCAN);
        let r = sk(R_EPHEMERAL);
        let scan_pub = PublicKey::from_secret_key(&secp, &b_scan);
        let ephemeral_pub = PublicKey::from_secret_key(&secp, &r);
        let shared = stealth_shared_point(&secp, &r, &scan_pub).expect("S");

        let core = one_time_tweak(&shared, &ephemeral_pub, StealthRail::Core, 0).expect("t core");
        let platform =
            one_time_tweak(&shared, &ephemeral_pub, StealthRail::Platform, 0).expect("t platform");
        assert_ne!(core.to_be_bytes(), platform.to_be_bytes());
    }

    /// Output counters must produce distinct tweaks (multi-output payments).
    #[test]
    fn output_counters_are_distinct() {
        let secp = Secp256k1::new();
        let b_scan = sk(B_SCAN);
        let r = sk(R_EPHEMERAL);
        let scan_pub = PublicKey::from_secret_key(&secp, &b_scan);
        let ephemeral_pub = PublicKey::from_secret_key(&secp, &r);
        let shared = stealth_shared_point(&secp, &r, &scan_pub).expect("S");

        let t0 = one_time_tweak(&shared, &ephemeral_pub, StealthRail::Core, 0).expect("t0");
        let t1 = one_time_tweak(&shared, &ephemeral_pub, StealthRail::Core, 1).expect("t1");
        assert_ne!(t0.to_be_bytes(), t1.to_be_bytes());
    }

    /// Exactness of the single-conditional-subtraction modular reduction at
    /// the boundaries: values below the order pass through, the order itself
    /// reduces to zero, and order+1 reduces to one.
    #[test]
    fn reduce_mod_order_boundaries() {
        assert_eq!(reduce_mod_order([0u8; 32]), [0u8; 32]);

        let mut below = CURVE_ORDER_BE;
        below[31] -= 1;
        assert_eq!(reduce_mod_order(below), below);

        assert_eq!(reduce_mod_order(CURVE_ORDER_BE), [0u8; 32]);

        let mut above = CURVE_ORDER_BE;
        above[31] += 1;
        let mut one = [0u8; 32];
        one[31] = 1;
        assert_eq!(reduce_mod_order(above), one);

        let max = [0xFFu8; 32];
        let reduced = reduce_mod_order(max);
        assert!(!ge_order(&reduced), "reduction must land below the order");
    }
}
