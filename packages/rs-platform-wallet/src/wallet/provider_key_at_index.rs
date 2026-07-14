//! On-demand per-index derivation of a wallet's provider key-material
//! keys — the BLS masternode **operator** keys
//! ([`AccountType::ProviderOperatorKeys`], FFI tag 10) and the Ed25519
//! **platform-node** keys ([`AccountType::ProviderPlatformKeys`], FFI
//! tag 11).
//!
//! These two accounts don't hold on-chain addresses or a balance; each
//! holds an extended public key over its own curve from which the
//! wallet derives one key per index. This module joins that account
//! xpub (and, when a private key is wanted, the account seed) back to
//! per-index key material so a caller can list / reveal a provider key
//! without knowing anything about the DIP-3 derivation-path shapes —
//! the whole lookup-and-derive happens here, on the Rust side, in a
//! single call. Mirrors the sibling
//! [`derive_core_address_private_key`](PlatformWallet::derive_core_address_private_key).
//!
//! # The curve asymmetry (why the two kinds behave differently)
//!
//! The managed pool derives an operator key at
//! [`AddressPoolType::Absent`](key_wallet::managed_account::address_pool::AddressPoolType::Absent)
//! — a **non-hardened** child index — and a platform-node key at
//! [`AddressPoolType::AbsentHardened`](key_wallet::managed_account::address_pool::AddressPoolType::AbsentHardened)
//! — a **hardened** child index. That single fact drives everything:
//!
//! - **Operator (BLS/BIP32):** non-hardened derivation works from the
//!   account *public* key (`ckd_pub`), so the 48-byte operator public
//!   key at an index needs neither a seed nor the mnemonic resolver.
//! - **Platform node (Ed25519/SLIP-10):** SLIP-10 Ed25519 is
//!   hardened-only — there is **no** public-key derivation. Even the
//!   32-byte public key at an index requires the account private key,
//!   so listing platform-node keys always needs the seed (hence the
//!   resolver for external-signable wallets).
//!
//! # Key source
//!
//! The BLS operator and Ed25519 platform-node accounts are derived from
//! the wallet's **raw BIP39 seed** in each curve's own HD scheme — the
//! exact input [`Wallet::add_bls_account`] / `add_eddsa_account` feed to
//! `BLSAccount::from_seed` / `EdDSAAccount::from_seed` (rust-dashcore
//! #879). Concretely:
//!
//! - **Operator public** at index `i` = the account xpub's non-hardened
//!   **legacy** `ckd_pub` child, `ExtendedBLSPubKey::derive_pub_legacy(i)`
//!   — no seed.
//! - **Operator private** at `i` = raw seed → `ExtendedBLSPrivKey`
//!   master → **legacy** DIP-3 account path → non-hardened legacy child
//!   `i`.
//! - **Platform node** at `i` = raw seed → `ExtendedEd25519PrivKey`
//!   master → DIP-3 account path → **hardened** child `i'` (SLIP-10 is
//!   hardened-only).
//!
//! This composes the upstream **extended-key primitives directly** rather
//! than the account's `derive_bls_key_at_index` / `derive_from_seed_*`
//! convenience wrappers, because those gate on the account's
//! `is_watch_only` flag *asymmetrically*: the public wrapper requires a
//! watch-only account, while `derive_from_seed_*` (via
//! `derive_xpriv_from_master_xpriv`) requires a **non**-watch-only one.
//! A resident wallet's provider account is non-watch-only and a restored
//! external-signable wallet's is watch-only, so no single wrapper works
//! for both. The primitives don't consult `is_watch_only`, so they derive
//! correctly regardless — and are byte-identical to what
//! `Wallet::from_mnemonic` account creation produces (pinned to the
//! dashbls reference vectors in the module tests).
//!
//! **Never** derive a secp256k1 child scalar at the DIP-3 path and feed
//! it to `new_master`: that pre-#879 hybrid yields a different point and
//! is exactly the bug that made these keys disagree with DashSync/dashbls.
//!
//! The raw 64-byte seed is obtained two ways, matching
//! [`derive_core_address_private_key`](PlatformWallet::derive_core_address_private_key):
//! `Some(seed)` for external-signable / watch-only wallets whose mnemonic
//! the caller resolved on demand; `None` to take a resident key-bearing
//! wallet's own [`wallet_seed_bytes`](key_wallet::wallet::Wallet::wallet_seed_bytes).
//! The seed and any returned scalar are wrapped in [`Zeroizing`] so they
//! are scrubbed when dropped.

use dashcore::hashes::{hash160, Hash};
use key_wallet::account::AccountType;
use key_wallet::bip32::ChildNumber;
use key_wallet::derivation_bls_bip32::ExtendedBLSPrivKey;
use key_wallet::derivation_slip10::{ExtendedEd25519PrivKey, ExtendedEd25519PubKey};
use zeroize::Zeroizing;

use super::platform_wallet::PlatformWallet;
use crate::changeset::ProviderPlatformNodePubKey;
use crate::error::PlatformWalletError;

/// Number of platform-node (Ed25519) keys pre-derived and persisted at
/// wallet registration.
///
/// Ed25519/SLIP-10 is hardened-only, so the wallet can never extend
/// this pool later via the gap-limit the way funds pools do — there is
/// no public derivation to walk without the seed. Pre-generating a
/// fixed batch while the seed is in hand at registration is the only
/// option; 20 mirrors the standard address gap limit so the Node Keys
/// screen has a full first page to show from persistence alone.
pub const PLATFORM_NODE_KEY_PREDERIVE_COUNT: u32 = 20;

/// Derive the platform-node (Ed25519) extended private key at `index`
/// from the raw BIP39 `seed`: SLIP-10 master → DIP-3
/// `ProviderPlatformKeys` account path → hardened child `index'`.
///
/// The single canonical platform-node private derivation, shared by
/// [`derive_platform_node_public_keys`] (the registration snapshot) and
/// [`PlatformWallet::derive_provider_key_at_index`] (the per-index
/// reveal) so the two can never diverge — the same anti-duplication
/// rationale as the rest of this module. Composed from the upstream
/// extended-key primitives directly (gate-free; see the module docs on
/// why the account's `is_watch_only`-gated `derive_from_seed_*` wrapper
/// is avoided).
///
/// # Errors
/// [`PlatformWalletError::KeyDerivation`] if the account path can't be
/// built, the SLIP-10 master / account key can't be derived, or the
/// per-index child derivation fails.
fn platform_node_xpriv_at(
    seed: &[u8],
    network: key_wallet::Network,
    index: u32,
) -> Result<ExtendedEd25519PrivKey, PlatformWalletError> {
    let account_path = AccountType::ProviderPlatformKeys
        .derivation_path(network)
        .map_err(|e| {
            PlatformWalletError::KeyDerivation(format!(
                "failed to build provider platform-node account path: {e}"
            ))
        })?;
    let master = ExtendedEd25519PrivKey::new_master(network, seed).map_err(|e| {
        PlatformWalletError::KeyDerivation(format!(
            "failed to build Ed25519 master from platform-node seed: {e}"
        ))
    })?;
    let account_xpriv = master.derive_priv(&account_path).map_err(|e| {
        PlatformWalletError::KeyDerivation(format!(
            "failed to derive Ed25519 platform-node account key at {account_path}: {e}"
        ))
    })?;
    // SLIP-10 Ed25519 is hardened-only — hardened child `index'`.
    let child = ChildNumber::from_hardened_idx(index).map_err(|e| {
        PlatformWalletError::KeyDerivation(format!("invalid platform-node key index {index}: {e}"))
    })?;
    account_xpriv.derive_priv(&[child]).map_err(|e| {
        PlatformWalletError::KeyDerivation(format!(
            "failed to derive Ed25519 platform-node key at index {index}: {e}"
        ))
    })
}

/// Derive the first `count` platform-node (Ed25519) public keys from a
/// **seed-bearing** [`Wallet`](key_wallet::wallet::Wallet), returning
/// the 32-byte public key + 20-byte `hash160` node id per hardened
/// index.
///
/// Used at registration (`PlatformWalletManager::register_wallet`)
/// to snapshot the pool while the seed is available, because the
/// platform-node curve is hardened-only and the pool can never be
/// extended later from an external-signable / watch-only wallet. The
/// derivation is the canonical rust-dashcore #879 scheme — the wallet's
/// **raw BIP39 seed** → [`ExtendedEd25519PrivKey`] master → DIP-3
/// `ProviderPlatformKeys` account path → hardened child `i` → public key
/// — composed from the upstream extended-key primitives directly (see the
/// module docs on why the account's `is_watch_only`-gated
/// `derive_from_seed_*` wrapper is avoided). It is byte-identical to what
/// [`PlatformWallet::derive_provider_key_at_index`] and account creation
/// produce. Only the public parts leave this function — the raw seed is
/// wrapped in [`Zeroizing`] and scrubbed on drop.
///
/// # Errors
/// [`PlatformWalletError::AddressNotFound`] if the wallet has no
/// platform-node account, or [`PlatformWalletError::KeyDerivation`] if
/// there's no resident seed to derive from (i.e. it's already
/// external-signable / watch-only) or any per-index derivation fails.
pub fn derive_platform_node_public_keys(
    wallet: &key_wallet::wallet::Wallet,
    network: key_wallet::Network,
    count: u32,
) -> Result<Vec<ProviderPlatformNodePubKey>, PlatformWalletError> {
    let account_type = AccountType::ProviderPlatformKeys;

    // Existence check — a missing platform-node account is a caller error,
    // not a derivation failure (the account object itself isn't needed for
    // the gate-free direct derivation below).
    if wallet
        .accounts
        .eddsa_account_of_type(account_type)
        .is_none()
    {
        return Err(PlatformWalletError::AddressNotFound(
            "wallet has no Ed25519 provider-platform-keys account".to_string(),
        ));
    }

    // Raw 64-byte BIP39 seed — the exact input `add_eddsa_account` feeds
    // to `EdDSAAccount::from_seed`. `None` once the wallet has been
    // downgraded to external-signable, but registration snapshots this
    // while the wallet is still seed-bearing.
    let seed: Zeroizing<[u8; 64]> =
        Zeroizing::new(wallet.wallet_seed_bytes().ok_or_else(|| {
            PlatformWalletError::KeyDerivation(
                "wallet has no resident seed to pre-derive platform-node keys".to_string(),
            )
        })?);

    let mut out = Vec::with_capacity(count as usize);
    for index in 0..count {
        let xpriv = platform_node_xpriv_at(seed.as_ref(), network, index)?;
        let verifying = ExtendedEd25519PubKey::from_priv(&xpriv).map_err(|e| {
            PlatformWalletError::KeyDerivation(format!(
                "failed to obtain Ed25519 public key at index {index}: {e}"
            ))
        })?;
        let public_key: [u8; 32] = verifying.public_key.to_bytes();
        // The 20-byte platform node id = hash160(ed25519 pubkey), the
        // value a ProRegTx `platform_node_id` matcher compares against.
        let node_id: [u8; 20] = hash160::Hash::hash(&public_key).to_byte_array();
        out.push(ProviderPlatformNodePubKey {
            index,
            public_key,
            node_id,
        });
    }
    Ok(out)
}

/// Which provider key-material account to derive from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKeyKind {
    /// BLS masternode operator keys
    /// ([`AccountType::ProviderOperatorKeys`], FFI tag 10).
    Operator,
    /// Ed25519 platform-node keys
    /// ([`AccountType::ProviderPlatformKeys`], FFI tag 11).
    PlatformNode,
}

impl ProviderKeyKind {
    /// The upstream account type this kind maps to.
    fn account_type(self) -> AccountType {
        match self {
            ProviderKeyKind::Operator => AccountType::ProviderOperatorKeys,
            ProviderKeyKind::PlatformNode => AccountType::ProviderPlatformKeys,
        }
    }
}

/// One provider key derived at a single index, in the forms a caller
/// wants to display.
pub struct ProviderDerivedKey {
    /// The key index within the provider pool (`#0..`).
    pub index: u32,
    /// Raw curve public key bytes in the MODERN (IETF) serialization: 48
    /// for a BLS operator key (this is exactly the bytes a ProRegTx
    /// `operator_public_key` field carries), 32 for an Ed25519
    /// platform-node key.
    pub public_key_bytes: Vec<u8>,
    /// The SAME BLS G1 point serialized in the Dash **legacy** scheme (48
    /// bytes; same point, legacy flag bits in `byte[0]`) — the form
    /// dashbls/DashSync use across the BLS HD chain. `Some` only for
    /// operator (BLS) keys; `None` for Ed25519 platform-node keys, which
    /// have no legacy variant. Produced Rust-side via
    /// `ExtendedBLSPubKey::to_bytes_legacy` / `ExtendedBLSPrivKey::
    /// public_key_bytes_legacy` (key-wallet #879) — never transformed in
    /// Swift.
    pub legacy_public_key_bytes: Option<Vec<u8>>,
    /// The 20-byte platform node id — `hash160` of the Ed25519 public
    /// key, the value a ProRegTx `platform_node_id` field carries and
    /// the [`Payload::PubkeyHash`](dashcore::address::Payload) the pool
    /// matcher compares against. `Some` for [`ProviderKeyKind::PlatformNode`];
    /// `None` for [`ProviderKeyKind::Operator`] (whose on-chain field is
    /// the raw 48-byte BLS public key, not a hash).
    pub node_id: Option<[u8; 20]>,
    /// Raw private-key scalar (32 bytes), present only when the caller
    /// asked for it. BLS / Ed25519 keys have no WIF, so this is the
    /// only private form. Zeroized on drop.
    pub private_key: Option<Zeroizing<Vec<u8>>>,
}

impl PlatformWallet {
    /// Derive this wallet's provider key of `kind` at `index`.
    ///
    /// Public-only when `resolved_seed` is `None` and `include_private`
    /// is `false` — but note the curve asymmetry documented at the module
    /// level: a [`ProviderKeyKind::Operator`] public key derives straight
    /// from the account xpub with no seed, whereas a
    /// [`ProviderKeyKind::PlatformNode`] key (Ed25519, SLIP-10
    /// hardened-only) always needs the seed even for its public key, so a
    /// watch-only wallet must supply `resolved_seed` to list platform-node
    /// keys at all.
    ///
    /// `resolved_seed` selects the key source: `Some(seed)` — the wallet's
    /// **raw BIP39 seed** — for external-signable / watch-only wallets
    /// whose mnemonic the caller resolved on demand; `None` to take a
    /// resident key-bearing wallet's own
    /// [`wallet_seed_bytes`](key_wallet::wallet::Wallet::wallet_seed_bytes).
    /// `include_private` additionally requests the raw private scalar.
    ///
    /// The derivation delegates to the account's own
    /// [`AccountDerivation`] routines (rust-dashcore #879) so every
    /// per-index key is byte-identical to what `Wallet::from_mnemonic`
    /// account creation produces; it never feeds a secp256k1 child scalar
    /// into a BLS/Ed25519 master (the pre-#879 hybrid).
    ///
    /// # Errors
    /// - [`PlatformWalletError::AddressNotFound`] if this wallet has no
    ///   account of the requested kind.
    /// - [`PlatformWalletError::KeyDerivation`] if key derivation fails —
    ///   including passing `None` for an external-signable / watch-only
    ///   wallet that has no resident seed when one is required.
    pub fn derive_provider_key_at_index(
        &self,
        kind: ProviderKeyKind,
        index: u32,
        resolved_seed: Option<&[u8]>,
        include_private: bool,
    ) -> Result<ProviderDerivedKey, PlatformWalletError> {
        let network = self.network();
        let account_type = kind.account_type();

        // Single read-lock: the account read and the resident-seed read
        // both borrow the in-process wallet from the same guard. No Swift
        // callback runs under this guard — the mnemonic resolver (if any)
        // already ran on the FFI side and produced `resolved_seed` before
        // we were called.
        let state = self.state_blocking();

        // Raw 64-byte BIP39 seed — the exact input the account's curve
        // master consumes (#879). Only obtained when a seed-bearing path
        // is actually needed: an operator public listing derives straight
        // from the account xpub, but an operator private reveal and *all*
        // platform-node derivations (Ed25519/SLIP-10 is hardened-only)
        // require it.
        let need_seed = include_private || matches!(kind, ProviderKeyKind::PlatformNode);

        let seed: Option<Zeroizing<Vec<u8>>> = if need_seed {
            let raw = match resolved_seed {
                // Caller-resolved raw seed (external-signable / watch-only).
                Some(seed) => Zeroizing::new(seed.to_vec()),
                None => {
                    // Resident key-bearing wallet — take its own raw BIP39
                    // seed. `None` for a watch-only / external-signable
                    // wallet (no resident seed), which the caller must
                    // instead service with a `resolved_seed`.
                    let raw = state.wallet().wallet_seed_bytes().ok_or_else(|| {
                        PlatformWalletError::KeyDerivation(
                            "wallet has no resident seed (external-signable / watch-only); a \
                             resolved seed is required to derive this provider key"
                                .to_string(),
                        )
                    })?;
                    Zeroizing::new(raw.to_vec())
                }
            };
            Some(raw)
        } else {
            None
        };

        match kind {
            ProviderKeyKind::Operator => {
                let account = state
                    .wallet()
                    .accounts
                    .bls_account_of_type(account_type)
                    .ok_or_else(|| {
                        PlatformWalletError::AddressNotFound(
                            "wallet has no BLS provider-operator-keys account".to_string(),
                        )
                    })?;

                // Non-hardened index — the operator pool's single
                // `AddressPoolType::Absent` chain (`account/i`).
                let child = ChildNumber::from_normal_idx(index).map_err(|e| {
                    PlatformWalletError::KeyDerivation(format!(
                        "invalid operator key index {index}: {e}"
                    ))
                })?;

                match &seed {
                    // Private reveal (or a caller that supplied a seed):
                    // raw seed → BLS master → **legacy** DIP-3 account path →
                    // non-hardened legacy child `index`. Composed from the
                    // upstream primitives directly (the account's
                    // `derive_from_seed_*` wrapper rejects a watch-only
                    // account even with a seed in hand — see module docs),
                    // yet byte-identical to it for a resident account.
                    Some(seed) => {
                        let master = ExtendedBLSPrivKey::new_master(network, seed.as_ref())
                            .map_err(|e| {
                                PlatformWalletError::KeyDerivation(format!(
                                    "failed to build BLS master from operator seed: {e}"
                                ))
                            })?;
                        let account_path = account_type.derivation_path(network).map_err(|e| {
                            PlatformWalletError::KeyDerivation(format!(
                                "failed to build BLS operator account path: {e}"
                            ))
                        })?;
                        let account_xpriv =
                            master.derive_path_legacy(&account_path).map_err(|e| {
                                PlatformWalletError::KeyDerivation(format!(
                                    "failed to derive BLS operator account key at \
                                     {account_path}: {e}"
                                ))
                            })?;
                        let xpriv = account_xpriv.derive_priv_legacy(child).map_err(|e| {
                            PlatformWalletError::KeyDerivation(format!(
                                "failed to derive BLS operator key at index {index}: {e}"
                            ))
                        })?;
                        let public_key_bytes = xpriv.public_key_bytes().to_vec();
                        // Same G1 point, Dash legacy serialization (key-wallet
                        // #879) — for the "BLS Public Key (Legacy)" display row.
                        let legacy_public_key_bytes =
                            Some(xpriv.public_key_bytes_legacy().to_vec());

                        // Always-on guard: the seed-derived public key MUST
                        // equal the account xpub's non-hardened legacy
                        // `ckd_pub` child. A mismatch means the wallet seed
                        // and its stored operator account xpub disagree
                        // (wrong seed / stale xpub) — exactly the failure
                        // this PR fixed — so refuse to hand out a mismatched
                        // key. `derive_pub_legacy` is a gate-free pure public
                        // operation (works for resident and watch-only
                        // accounts alike), and the extra derivation on the
                        // private-reveal path is negligible. If that public
                        // derivation itself can't be performed we log and
                        // proceed rather than fail: the private derivation
                        // already succeeded, so a valid key is in hand and
                        // failing here would only turn a working path into a
                        // spurious error.
                        match account.bls_public_key.derive_pub_legacy(child) {
                            Ok(xpub) => {
                                if xpub.to_bytes() != xpriv.public_key_bytes() {
                                    return Err(PlatformWalletError::KeyDerivation(format!(
                                        "BLS operator key at index {index}: seed-derived public \
                                         key does not match the account xpub derivation — the \
                                         wallet seed and its stored operator account xpub \
                                         disagree; refusing to return a mismatched key"
                                    )));
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    index,
                                    error = %e,
                                    "BLS operator seed/xpub cross-check skipped: account xpub \
                                     public derivation failed"
                                );
                            }
                        }

                        let private_key = include_private
                            .then(|| Zeroizing::new(xpriv.private_key.to_be_bytes().to_vec()));

                        Ok(ProviderDerivedKey {
                            index,
                            public_key_bytes,
                            legacy_public_key_bytes,
                            node_id: None,
                            private_key,
                        })
                    }
                    // Public-only: the account xpub's non-hardened **legacy**
                    // `ckd_pub` child — no seed / resolver needed for BLS.
                    // `derive_pub_legacy` is a pure public-key operation
                    // (no `is_watch_only` gate), so it works for both
                    // resident and restored (watch-only) accounts.
                    None => {
                        let xpub =
                            account
                                .bls_public_key
                                .derive_pub_legacy(child)
                                .map_err(|e| {
                                    PlatformWalletError::KeyDerivation(format!(
                                        "failed to derive BLS operator public key at index \
                                     {index}: {e}"
                                    ))
                                })?;
                        // Serialize the same G1 point both ways: modern/IETF
                        // and Dash legacy (key-wallet #879).
                        let public_key_bytes = xpub.to_bytes().to_vec();
                        let legacy_public_key_bytes = Some(xpub.to_bytes_legacy().to_vec());
                        Ok(ProviderDerivedKey {
                            index,
                            public_key_bytes,
                            legacy_public_key_bytes,
                            node_id: None,
                            private_key: None,
                        })
                    }
                }
            }
            ProviderKeyKind::PlatformNode => {
                // Existence check — a missing account is a caller error,
                // not a derivation failure (the account object itself isn't
                // needed for the gate-free direct derivation below).
                if state
                    .wallet()
                    .accounts
                    .eddsa_account_of_type(account_type)
                    .is_none()
                {
                    return Err(PlatformWalletError::AddressNotFound(
                        "wallet has no Ed25519 provider-platform-keys account".to_string(),
                    ));
                }

                // `need_seed` is always true for this kind.
                let seed = seed
                    .as_ref()
                    .expect("platform-node derivation always seeds");

                // Canonical raw-seed derivation via the shared helper (the
                // exact routine `derive_platform_node_public_keys` uses, so
                // the per-index reveal can never diverge from the persisted
                // batch). Composed from the upstream primitives directly —
                // the account's `derive_from_seed_*` wrapper rejects a
                // watch-only account even with a seed (see module docs).
                let xpriv = platform_node_xpriv_at(seed.as_ref(), network, index)?;
                let verifying = ExtendedEd25519PubKey::from_priv(&xpriv).map_err(|e| {
                    PlatformWalletError::KeyDerivation(format!(
                        "failed to obtain Ed25519 public key at index {index}: {e}"
                    ))
                })?;
                let public_key_bytes = verifying.public_key.to_bytes().to_vec();

                // The 20-byte platform node id = hash160(ed25519 pubkey),
                // exactly what the ProRegTx `platform_node_id` matcher
                // compares against (`Payload::PubkeyHash`).
                let node_id: [u8; 20] = hash160::Hash::hash(&public_key_bytes).to_byte_array();

                let private_key =
                    include_private.then(|| Zeroizing::new(xpriv.private_key.to_vec()));

                Ok(ProviderDerivedKey {
                    index,
                    public_key_bytes,
                    // Ed25519 platform-node keys have no BLS legacy variant.
                    legacy_public_key_bytes: None,
                    node_id: Some(node_id),
                    private_key,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // The upstream account wrappers (`derive_from_seed_*`) are used ONLY in
    // tests, as an independent cross-check that the module's gate-free
    // primitives match upstream for a resident (non-watch-only) account.
    use dashcore::hashes::{hash160, Hash};
    use key_wallet::account::derivation::AccountDerivation;
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    use key_wallet::Network;

    // Canonical all-`abandon` BIP-39 test vector — deterministic, so the
    // derived key material below is a stable golden vector.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    fn seed_bearing_wallet(network: Network) -> Wallet {
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid test mnemonic");
        Wallet::from_mnemonic(mnemonic, network, WalletAccountCreationOptions::Default)
            .expect("wallet construction")
    }

    /// The load-bearing invariants for platform-node keys, pinned so a
    /// derivation regression (like the pre-#879 secp256k1 hybrid) can't
    /// silently hand out wrong key material: `node_id == hash160(pubkey)`
    /// at every index, all indices distinct, our wrapper's per-index key
    /// equals the account's own canonical seed derivation, and a golden
    /// secret scalar pinned to the dashbls/SLIP-10 reference vector from
    /// key-wallet's `provider_key_derivation_tests.rs`.
    #[test]
    fn platform_node_keys_are_consistent_and_pinned() {
        let wallet = seed_bearing_wallet(Network::Mainnet);
        let seed = wallet.wallet_seed_bytes().expect("resident seed");
        let account = wallet
            .accounts
            .eddsa_account_of_type(AccountType::ProviderPlatformKeys)
            .expect("platform-node account");

        let keys = derive_platform_node_public_keys(&wallet, Network::Mainnet, 20)
            .expect("platform-node derivation");

        assert_eq!(keys.len(), 20, "requested 20 keys");
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(k.index, i as u32, "index ordering");
            let expected_node_id: [u8; 20] = hash160::Hash::hash(&k.public_key).to_byte_array();
            assert_eq!(
                k.node_id, expected_node_id,
                "node_id must be hash160(ed25519 pubkey) at index {i}"
            );

            // Cross-path: the registration wrapper's per-index key must be
            // byte-identical to the account's own canonical raw-seed
            // derivation — the same routine `derive_provider_key_at_index`
            // and `Wallet::from_mnemonic` account creation use.
            let up_xpriv = account
                .derive_from_seed_extended_xpriv_at(&seed, i as u32)
                .expect("account seed derivation");
            let up_pub = ExtendedEd25519PubKey::from_priv(&up_xpriv)
                .expect("ed25519 pub")
                .public_key
                .to_bytes();
            assert_eq!(
                k.public_key, up_pub,
                "wrapper pubkey must equal the account's canonical derivation at {i}"
            );
        }

        // All indices produce distinct keys (hardened SLIP-10 children).
        let mut pubs: Vec<[u8; 32]> = keys.iter().map(|k| k.public_key).collect();
        pubs.sort();
        pubs.dedup();
        assert_eq!(pubs.len(), 20, "all 20 platform-node keys must be distinct");

        // Golden vector — mainnet platform-node key 0 secret scalar
        // (`m/9'/5'/3'/4'/0'`), pinned to the SLIP-10 reference in
        // key-wallet `provider_key_derivation_tests.rs`. This is the value
        // DashSync/dashwallet-ios produce; a break means our derivation
        // diverged from the reference.
        let sk0 = account
            .derive_from_seed_private_key_at(&seed, 0)
            .expect("platform-node sk derivation");
        assert_eq!(
            hex::encode(sk0.to_bytes()),
            "5fa238b12be77347abf9b5957bd902d16c6aaca28d25c4267ffacbd7458dceb1",
            "mainnet platform-node index-0 secret golden (dashbls/SLIP-10 reference)"
        );
    }

    /// Operator (BLS) keys pinned to the dashbls reference vectors from
    /// key-wallet `provider_key_derivation_tests.rs` — the values
    /// DashSync/dashwallet-ios produce — on both networks. Exercises BOTH
    /// derivation paths this module actually uses: the public branch's
    /// `ExtendedBLSPubKey::derive_pub_legacy` (gate-free) and the private
    /// branch's gate-free primitive composition, cross-checked against
    /// each other, the account's own wrapper, and the golden vectors. This
    /// is the exact key the user reported as mismatched; the pre-#879
    /// secp256k1 hybrid produced a different point here, and the account's
    /// `derive_bls_key_at_index` wrapper can't derive it from a resident
    /// (non-watch-only) account at all.
    #[test]
    fn operator_keys_match_dashbls_reference_and_cross_check() {
        // --- Mainnet (`m/9'/5'/3'/3'/0`) ---
        let wallet = seed_bearing_wallet(Network::Mainnet);
        let seed = wallet.wallet_seed_bytes().expect("resident seed");
        let account = wallet
            .accounts
            .bls_account_of_type(AccountType::ProviderOperatorKeys)
            .expect("operator account");

        // Public-only path (exactly what `derive_provider_key_at_index`
        // does for the seedless branch): the account xpub's non-hardened
        // **legacy** `ckd_pub`. `derive_pub_legacy` is gate-free, so it
        // works on this resident (non-watch-only) account too.
        let child0 = ChildNumber::from_normal_idx(0).expect("child 0");
        let xpub0 = account
            .bls_public_key
            .derive_pub_legacy(child0)
            .expect("operator public derivation");
        assert_eq!(
            hex::encode(xpub0.to_bytes_legacy()),
            "078cad04aae29eb76171937eb7101452b401b026efbc27db840f130374e6a9ec8443d917277f8921e0ba6678a7709875",
            "operator key0 legacy pubkey golden (dashbls reference)"
        );
        // Modern/IETF (basic-scheme) serialization, verbatim from the
        // upstream reference test.
        assert_eq!(
            hex::encode(xpub0.to_bytes()),
            "878cad04aae29eb76171937eb7101452b401b026efbc27db840f130374e6a9ec8443d917277f8921e0ba6678a7709875",
            "operator key0 modern pubkey golden (dashbls reference)"
        );

        // Private (seed) path composed from the SAME gate-free primitives
        // `derive_provider_key_at_index` uses: raw seed → BLS master →
        // legacy account path → non-hardened legacy child.
        let account_path = AccountType::ProviderOperatorKeys
            .derivation_path(Network::Mainnet)
            .expect("account path");
        let master = ExtendedBLSPrivKey::new_master(Network::Mainnet, &seed).expect("bls master");
        let xpriv0 = master
            .derive_path_legacy(&account_path)
            .expect("account xpriv")
            .derive_priv_legacy(child0)
            .expect("child xpriv");
        assert_eq!(
            xpriv0.public_key_bytes(),
            xpub0.to_bytes(),
            "gate-free seed-derived operator pubkey must equal the public derivation"
        );
        assert_eq!(
            xpriv0.public_key_bytes_legacy(),
            xpub0.to_bytes_legacy(),
            "gate-free seed-derived operator legacy pubkey must equal the public derivation"
        );
        assert_eq!(
            hex::encode(xpriv0.private_key.to_be_bytes()),
            "11122e1ad656d0610ce0f80d40da874d67ea656a3e66ed371c915ec3a488a43a",
            "operator key0 secret golden (dashbls reference)"
        );

        // The gate-free composition must equal the account's own wrapper
        // for this resident (non-watch-only) account — proving they agree
        // where the wrapper is usable.
        let via_wrapper = account
            .derive_from_seed_extended_xpriv_at(&seed, 0)
            .expect("wrapper seed derivation");
        assert_eq!(
            via_wrapper.public_key_bytes(),
            xpriv0.public_key_bytes(),
            "gate-free primitives must match the account's derive_from_seed_* wrapper"
        );

        // --- Testnet (`m/9'/1'/3'/3'/0`) ---
        let wallet_t = seed_bearing_wallet(Network::Testnet);
        let account_t = wallet_t
            .accounts
            .bls_account_of_type(AccountType::ProviderOperatorKeys)
            .expect("operator account");
        let xpub0_t = account_t
            .bls_public_key
            .derive_pub_legacy(child0)
            .expect("operator public derivation");
        assert_eq!(
            hex::encode(xpub0_t.to_bytes_legacy()),
            "09d8beabae708de1638487f1aff44b38e8c07d9b09f22d76329d6c8ec01e2ad4d030b660bca40ddbd222373a72c5bcef",
            "testnet operator key0 legacy pubkey golden (dashbls reference)"
        );
        let seed_t = wallet_t.wallet_seed_bytes().expect("resident seed");
        let account_path_t = AccountType::ProviderOperatorKeys
            .derivation_path(Network::Testnet)
            .expect("account path");
        let xpriv0_t = ExtendedBLSPrivKey::new_master(Network::Testnet, &seed_t)
            .expect("bls master")
            .derive_path_legacy(&account_path_t)
            .expect("account xpriv")
            .derive_priv_legacy(child0)
            .expect("child xpriv");
        assert_eq!(
            xpriv0_t.public_key_bytes_legacy(),
            xpub0_t.to_bytes_legacy(),
            "testnet gate-free operator legacy pubkey must equal the public derivation"
        );
        assert_eq!(
            hex::encode(xpriv0_t.private_key.to_be_bytes()),
            "3346dfd71627f9f31cad3ee66fe7b673c32cb077b2eb38c621d7e61c30e46dbd",
            "testnet operator key0 secret golden (dashbls reference)"
        );
    }

    /// Re-deriving the same (mnemonic, network) is byte-stable — the watch
    /// -only restore path re-persists nothing, so display must be reproducible.
    #[test]
    fn platform_node_keys_are_stable_across_calls() {
        let wallet = seed_bearing_wallet(Network::Mainnet);
        let a = derive_platform_node_public_keys(&wallet, Network::Mainnet, 5).unwrap();
        let b = derive_platform_node_public_keys(&wallet, Network::Mainnet, 5).unwrap();
        assert_eq!(
            a.iter().map(|k| k.public_key).collect::<Vec<_>>(),
            b.iter().map(|k| k.public_key).collect::<Vec<_>>(),
            "platform-node derivation must be deterministic"
        );
    }

    /// Mainnet and testnet derive different platform-node key material from
    /// the same mnemonic (network-scoped DIP-9 coin type).
    #[test]
    fn platform_node_keys_differ_across_networks() {
        let mainnet = derive_platform_node_public_keys(
            &seed_bearing_wallet(Network::Mainnet),
            Network::Mainnet,
            1,
        )
        .unwrap();
        let testnet = derive_platform_node_public_keys(
            &seed_bearing_wallet(Network::Testnet),
            Network::Testnet,
            1,
        )
        .unwrap();
        assert_ne!(
            mainnet[0].public_key, testnet[0].public_key,
            "same mnemonic must yield different platform-node keys per network"
        );
    }
}
