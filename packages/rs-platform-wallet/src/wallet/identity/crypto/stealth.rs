//! DIP-33 stealth payment key derivation and one-time address construction.
//!
//! This module composes the pure stealth curve math from
//! [`platform_encryption::stealth`] with DashPay's wallet concerns: the DIP-9
//! feature-33' key derivation, and the formation of concrete payment
//! destinations on the two transparent rails — Dash Core P2PKH addresses and
//! Platform payment addresses ([DIP-18]).
//!
//! # Derivation path
//!
//! Detection keys live under DIP-9 feature `33'`:
//!
//! ```text
//! m / 9' / coin' / 33' / account' / key_class' / index'
//! ```
//!
//! with `key_class'` selecting the role: `0'` scan, `1'` spend, `2'`
//! notification out-key. All segments are hardened; disclosure of the scan
//! key reveals nothing about the spend or out keys.
//!
//! # Roles
//!
//! * **Payer** ([`derive_one_time_destination`]) — given the recipient's
//!   published scan/spend public keys, derives a fresh one-time destination
//!   and the ephemeral public key `R` to publish in the payment notification.
//! * **Receiver** ([`recognize_one_time_destination`]) — given `R` and the
//!   recipient's scan secret + spend public key, re-derives the destination to
//!   match against the chain. Needs no spend secret (watch-only capable).
//! * **Spender** ([`derive_one_time_secret_key`]) — given `R` and the scan +
//!   spend secrets, derives the one-time secret key that controls the output.
//!
//! [DIP-18]: https://github.com/dashpay/dips/blob/master/dip-0018.md

use dashcore::hashes::Hash;
use dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use dashcore::{Address, PublicKey as DashPublicKey};
use dpp::address_funds::PlatformAddress;
use key_wallet::bip32::{ChildNumber, DerivationPath};
use key_wallet::dip9::{DASH_COIN_TYPE, DASH_TESTNET_COIN_TYPE, FEATURE_PURPOSE};
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use platform_encryption::{
    one_time_public_key, one_time_secret_key, stealth_shared_point, StealthRail,
};

use crate::error::PlatformWalletError;

/// DIP-9 feature index for DashPay payment detection keys (matches DIP-33).
pub const FEATURE_PURPOSE_PAYMENT_DETECTION: u32 = 33;

/// `key_class'` for the scan key.
pub const KEY_CLASS_SCAN: u32 = 0;
/// `key_class'` for the spend key.
pub const KEY_CLASS_SPEND: u32 = 1;
/// `key_class'` for the notification out-key.
pub const KEY_CLASS_NOTIF_OUT: u32 = 2;

/// Which transparent rail a one-time destination is being built for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentRail {
    /// Dash Core chain P2PKH output.
    Core,
    /// Platform payment address (P2PKH).
    Platform,
}

impl From<PaymentRail> for StealthRail {
    fn from(rail: PaymentRail) -> Self {
        match rail {
            PaymentRail::Core => StealthRail::Core,
            PaymentRail::Platform => StealthRail::Platform,
        }
    }
}

/// A one-time payment destination on a specific rail. The variant matches the
/// rail requested; both wrap the same underlying `hash160(P_n)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OneTimeDestination {
    /// A Dash Core chain P2PKH address.
    Core(Address),
    /// A Platform payment address (P2PKH storage form).
    Platform(PlatformAddress),
}

/// The DIP-33 detection-key derivation path
/// `m/9'/coin'/33'/account'/key_class'/index'` (all hardened).
pub fn payment_detection_derivation_path(
    network: Network,
    account: u32,
    key_class: u32,
    index: u32,
) -> Result<DerivationPath, PlatformWalletError> {
    let coin_type: u32 = match network {
        Network::Mainnet => DASH_COIN_TYPE,
        _ => DASH_TESTNET_COIN_TYPE,
    };
    let hardened = |value: u32, label: &str| {
        ChildNumber::from_hardened_idx(value).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Invalid {label} index for payment detection path: {e}"
            ))
        })
    };
    Ok(DerivationPath::from(vec![
        hardened(FEATURE_PURPOSE, "feature purpose")?,
        hardened(coin_type, "coin type")?,
        hardened(FEATURE_PURPOSE_PAYMENT_DETECTION, "feature")?,
        hardened(account, "account")?,
        hardened(key_class, "key class")?,
        hardened(index, "index")?,
    ]))
}

/// Derive a DIP-33 detection secret key at the given role and index.
pub fn derive_payment_detection_secret_key(
    wallet: &Wallet,
    network: Network,
    account: u32,
    key_class: u32,
    index: u32,
) -> Result<SecretKey, PlatformWalletError> {
    let path = payment_detection_derivation_path(network, account, key_class, index)?;
    let ext_priv = wallet.derive_extended_private_key(&path).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to derive payment detection key: {e}"
        ))
    })?;
    let secret_bytes = zeroize::Zeroizing::new(ext_priv.private_key.secret_bytes());
    SecretKey::from_slice(&*secret_bytes).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Invalid derived payment detection key: {e}"
        ))
    })
}

/// Form the rail-specific destination from a one-time public key.
fn destination_from_public_key(
    one_time_public: &PublicKey,
    rail: PaymentRail,
    network: Network,
) -> OneTimeDestination {
    let dash_pubkey = DashPublicKey::new(*one_time_public);
    match rail {
        PaymentRail::Core => OneTimeDestination::Core(Address::p2pkh(&dash_pubkey, network)),
        PaymentRail::Platform => {
            let hash = dash_pubkey.pubkey_hash();
            OneTimeDestination::Platform(PlatformAddress::P2pkh(hash.to_byte_array()))
        }
    }
}

/// Payer: derive the one-time destination for output `n` to a recipient whose
/// published detection keys are `scan_public` / `spend_public`, using a fresh
/// ephemeral secret `ephemeral_secret`.
///
/// Returns the destination together with the ephemeral public key `R` that
/// must be published in the payment notification so the recipient can detect
/// and spend the output. The same `ephemeral_secret` (hence `R`) is shared
/// across every output `n` of one payment.
pub fn derive_one_time_destination(
    ephemeral_secret: &SecretKey,
    scan_public: &PublicKey,
    spend_public: &PublicKey,
    rail: PaymentRail,
    n: u32,
    network: Network,
) -> Result<(OneTimeDestination, PublicKey), PlatformWalletError> {
    let secp = Secp256k1::new();
    let ephemeral_public = PublicKey::from_secret_key(&secp, ephemeral_secret);
    let shared = stealth_shared_point(&secp, ephemeral_secret, scan_public)
        .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
    let one_time_public = one_time_public_key(
        &secp,
        spend_public,
        &shared,
        &ephemeral_public,
        rail.into(),
        n,
    )
    .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
    Ok((
        destination_from_public_key(&one_time_public, rail, network),
        ephemeral_public,
    ))
}

/// Receiver / watch service: re-derive the one-time destination for output `n`
/// given the payer's published `R`, the recipient's scan secret, and the
/// recipient's spend public key. Compare the result against the referenced
/// on-chain output. Requires no spend secret.
pub fn recognize_one_time_destination(
    scan_secret: &SecretKey,
    spend_public: &PublicKey,
    ephemeral_public: &PublicKey,
    rail: PaymentRail,
    n: u32,
    network: Network,
) -> Result<OneTimeDestination, PlatformWalletError> {
    let secp = Secp256k1::new();
    let shared = stealth_shared_point(&secp, scan_secret, ephemeral_public)
        .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
    let one_time_public = one_time_public_key(
        &secp,
        spend_public,
        &shared,
        ephemeral_public,
        rail.into(),
        n,
    )
    .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
    Ok(destination_from_public_key(&one_time_public, rail, network))
}

/// Spender: derive the one-time secret key controlling output `n`, given the
/// payer's published `R`, the recipient's scan secret, and the recipient's
/// spend secret.
pub fn derive_one_time_secret_key(
    scan_secret: &SecretKey,
    spend_secret: &SecretKey,
    ephemeral_public: &PublicKey,
    rail: PaymentRail,
    n: u32,
) -> Result<SecretKey, PlatformWalletError> {
    let secp = Secp256k1::new();
    let shared = stealth_shared_point(&secp, scan_secret, ephemeral_public)
        .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
    one_time_secret_key(spend_secret, &shared, ephemeral_public, rail.into(), n)
        .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sk(byte: u8) -> SecretKey {
        SecretKey::from_slice(&[byte; 32]).expect("valid scalar")
    }

    /// End-to-end: what the payer derives on each rail is exactly what the
    /// receiver recognizes and what the spender's secret key controls.
    #[test]
    fn payer_receiver_spender_agree_on_both_rails() {
        let secp = Secp256k1::new();
        let scan_secret = sk(0x11);
        let spend_secret = sk(0x22);
        let ephemeral_secret = sk(0x33);
        let scan_public = PublicKey::from_secret_key(&secp, &scan_secret);
        let spend_public = PublicKey::from_secret_key(&secp, &spend_secret);

        for rail in [PaymentRail::Core, PaymentRail::Platform] {
            for n in 0..3 {
                let (payer_dest, r) = derive_one_time_destination(
                    &ephemeral_secret,
                    &scan_public,
                    &spend_public,
                    rail,
                    n,
                    Network::Testnet,
                )
                .expect("payer derivation");

                let receiver_dest = recognize_one_time_destination(
                    &scan_secret,
                    &spend_public,
                    &r,
                    rail,
                    n,
                    Network::Testnet,
                )
                .expect("receiver derivation");
                assert_eq!(payer_dest, receiver_dest, "payer and receiver must agree");

                let one_time_secret =
                    derive_one_time_secret_key(&scan_secret, &spend_secret, &r, rail, n)
                        .expect("spender derivation");
                let one_time_public = PublicKey::from_secret_key(&secp, &one_time_secret);
                let spend_dest =
                    destination_from_public_key(&one_time_public, rail, Network::Testnet);
                assert_eq!(
                    payer_dest, spend_dest,
                    "the one-time secret must control the destination"
                );
            }
        }
    }

    /// The two rails must yield different destinations for identical inputs
    /// (the rail byte in the tweak domain-separates them).
    #[test]
    fn rails_produce_distinct_destinations() {
        let secp = Secp256k1::new();
        let scan_secret = sk(0x44);
        let spend_secret = sk(0x55);
        let ephemeral_secret = sk(0x66);
        let scan_public = PublicKey::from_secret_key(&secp, &scan_secret);
        let spend_public = PublicKey::from_secret_key(&secp, &spend_secret);

        let (core_dest, _) = derive_one_time_destination(
            &ephemeral_secret,
            &scan_public,
            &spend_public,
            PaymentRail::Core,
            0,
            Network::Testnet,
        )
        .expect("core");
        let (platform_dest, _) = derive_one_time_destination(
            &ephemeral_secret,
            &scan_public,
            &spend_public,
            PaymentRail::Platform,
            0,
            Network::Testnet,
        )
        .expect("platform");

        let core_hash = match core_dest {
            OneTimeDestination::Core(addr) => addr.pubkey_hash().expect("p2pkh").to_byte_array(),
            _ => panic!("expected core"),
        };
        let platform_hash = match platform_dest {
            OneTimeDestination::Platform(PlatformAddress::P2pkh(h)) => h,
            _ => panic!("expected platform p2pkh"),
        };
        assert_ne!(core_hash, platform_hash);
    }

    /// The derivation path is fully hardened and shaped m/9'/coin'/33'/a'/c'/i'.
    #[test]
    fn derivation_path_is_hardened_and_well_formed() {
        let path = payment_detection_derivation_path(Network::Mainnet, 0, KEY_CLASS_SCAN, 0)
            .expect("path");
        let segments: Vec<ChildNumber> = path.into_iter().copied().collect();
        assert_eq!(segments.len(), 6);
        assert_eq!(segments[0], ChildNumber::from_hardened_idx(9).unwrap());
        assert_eq!(segments[1], ChildNumber::from_hardened_idx(5).unwrap());
        assert_eq!(segments[2], ChildNumber::from_hardened_idx(33).unwrap());
        assert!(segments.iter().all(|c| c.is_hardened()));
    }
}
