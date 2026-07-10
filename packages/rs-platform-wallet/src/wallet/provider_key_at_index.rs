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
//! The account seed (the 32 secret bytes at the account's DIP-3 path,
//! the exact input [`Wallet::add_bls_account`] / `add_eddsa_account`
//! feed to `new_master`) is obtained two ways, matching
//! [`derive_core_address_private_key`](PlatformWallet::derive_core_address_private_key):
//! `Some(master)` for external-signable / watch-only wallets whose
//! mnemonic the caller resolved on demand; `None` to derive from a
//! resident key-bearing wallet. The seed and any returned scalar are
//! wrapped in [`Zeroizing`] so they are scrubbed when dropped.

use dashcore::hashes::{hash160, Hash};
use dashcore::secp256k1::Secp256k1;
use key_wallet::account::AccountType;
use key_wallet::bip32::{ChildNumber, ExtendedPrivKey};
use key_wallet::derivation_bls_bip32::ExtendedBLSPrivKey;
use key_wallet::derivation_slip10::ExtendedEd25519PrivKey;
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

/// Derive the first `count` platform-node (Ed25519) public keys from a
/// **seed-bearing** [`Wallet`](key_wallet::wallet::Wallet), returning
/// the 32-byte public key + 20-byte `hash160` node id per hardened
/// index.
///
/// Used at registration (`PlatformWalletManager::register_wallet`)
/// to snapshot the pool while the seed is available, because the
/// platform-node curve is hardened-only and the pool can never be
/// extended later from an external-signable / watch-only wallet. The
/// derivation mirrors the private path in
/// [`PlatformWallet::derive_provider_key_at_index`] exactly: account
/// seed at the DIP-3 `ProviderPlatformKeys` path →
/// [`ExtendedEd25519PrivKey::new_master`] → hardened child `i` → public
/// key. Only the public parts leave this function — the account seed
/// is wrapped in [`Zeroizing`] and scrubbed on drop.
///
/// # Errors
/// [`PlatformWalletError::KeyDerivation`] if the account path can't be
/// built, the wallet has no resident private key to derive the account
/// seed (i.e. it's already watch-only), or any per-index derivation
/// fails.
pub fn derive_platform_node_public_keys(
    wallet: &key_wallet::wallet::Wallet,
    network: key_wallet::Network,
    count: u32,
) -> Result<Vec<ProviderPlatformNodePubKey>, PlatformWalletError> {
    let account_type = AccountType::ProviderPlatformKeys;

    // Account-level seed: the same secp256k1 secret bytes at the DIP-3
    // `ProviderPlatformKeys` path that `Wallet::add_eddsa_account` feeds
    // to `new_master`. Errors for a watch-only wallet with no resident
    // private key — but at registration the wallet is still seed-bearing.
    let account_path = account_type.derivation_path(network).map_err(|e| {
        PlatformWalletError::KeyDerivation(format!(
            "failed to build provider platform-node account path: {e}"
        ))
    })?;
    let secret = wallet.derive_private_key(&account_path).map_err(|e| {
        PlatformWalletError::KeyDerivation(format!(
            "failed to derive provider platform-node account seed at {account_path}: {e}"
        ))
    })?;
    let account_seed: Zeroizing<[u8; 32]> = Zeroizing::new(secret.secret_bytes());

    let ed_master =
        ExtendedEd25519PrivKey::new_master(network, account_seed.as_ref()).map_err(|e| {
            PlatformWalletError::KeyDerivation(format!(
                "failed to build Ed25519 master from platform-node seed: {e}"
            ))
        })?;

    let mut out = Vec::with_capacity(count as usize);
    for index in 0..count {
        // SLIP-10 Ed25519 is hardened-only — the pool derives
        // platform-node keys at a single hardened index
        // (`AddressPoolType::AbsentHardened`).
        let child = ChildNumber::from_hardened_idx(index).map_err(|e| {
            PlatformWalletError::KeyDerivation(format!(
                "invalid platform-node key index {index}: {e}"
            ))
        })?;
        let derived = ed_master.derive_priv(&[child]).map_err(|e| {
            PlatformWalletError::KeyDerivation(format!(
                "failed to derive Ed25519 platform-node key at index {index}: {e}"
            ))
        })?;
        let verifying = derived.public_key().map_err(|e| {
            PlatformWalletError::KeyDerivation(format!(
                "failed to obtain Ed25519 public key at index {index}: {e}"
            ))
        })?;
        // `to_bytes().to_vec()` is exactly how
        // `derive_provider_key_at_index` materialises the 32-byte Ed25519
        // public key; normalise into a fixed array for the display struct.
        let public_key_bytes = verifying.to_bytes().to_vec();
        let public_key: [u8; 32] = public_key_bytes.as_slice().try_into().map_err(|_| {
            PlatformWalletError::KeyDerivation(format!(
                "Ed25519 public key at index {index} was not 32 bytes"
            ))
        })?;
        // The 20-byte platform node id = hash160(ed25519 pubkey), the
        // value a ProRegTx `platform_node_id` matcher compares against.
        let node_id: [u8; 20] = hash160::Hash::hash(&public_key_bytes).to_byte_array();
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
    /// Raw curve public key bytes: 48 for a BLS operator key (this is
    /// exactly the bytes a ProRegTx `operator_public_key` field
    /// carries), 32 for an Ed25519 platform-node key.
    pub public_key_bytes: Vec<u8>,
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
    /// Public-only when `resolved_master` is `None` and
    /// `include_private` is `false` — but note the curve asymmetry
    /// documented at the module level: a [`ProviderKeyKind::Operator`]
    /// public key derives straight from the account xpub with no seed,
    /// whereas a [`ProviderKeyKind::PlatformNode`] key (Ed25519, SLIP-10
    /// hardened-only) always needs the account seed even for its public
    /// key, so a watch-only wallet must supply `resolved_master` to list
    /// platform-node keys at all.
    ///
    /// `resolved_master` selects the key source: `Some(master)` for
    /// external-signable / watch-only wallets whose mnemonic the caller
    /// resolved on demand; `None` to derive from a resident key-bearing
    /// wallet. `include_private` additionally requests the raw private
    /// scalar.
    ///
    /// # Errors
    /// - [`PlatformWalletError::AddressNotFound`] if this wallet has no
    ///   account of the requested kind.
    /// - [`PlatformWalletError::KeyDerivation`] if key derivation fails —
    ///   including passing `None` for a watch-only wallet that has no
    ///   resident private keys when a seed is required.
    pub fn derive_provider_key_at_index(
        &self,
        kind: ProviderKeyKind,
        index: u32,
        resolved_master: Option<&ExtendedPrivKey>,
        include_private: bool,
    ) -> Result<ProviderDerivedKey, PlatformWalletError> {
        let network = self.network();
        let account_type = kind.account_type();

        // Single read-lock: the account xpub read and the resident-key
        // derive both borrow the in-process wallet from the same guard.
        // No Swift callback runs under this guard — the mnemonic
        // resolver (if any) already ran on the FFI side and produced
        // `resolved_master` before we were called.
        let state = self.state_blocking();

        // The account-level seed (32 secret bytes at the account's DIP-3
        // path) is the input both curves' `new_master` consumes. Only
        // compute it when a curve master is actually needed: an operator
        // public listing derives straight from the account xpub, but an
        // operator private reveal and *all* platform-node derivations
        // (Ed25519/SLIP-10 is hardened-only) require it.
        let need_seed = include_private || matches!(kind, ProviderKeyKind::PlatformNode);

        let account_seed: Option<Zeroizing<[u8; 32]>> = if need_seed {
            let account_path = account_type.derivation_path(network).map_err(|e| {
                PlatformWalletError::KeyDerivation(format!(
                    "failed to build provider account path for {account_type:?}: {e}"
                ))
            })?;
            let seed = match resolved_master {
                Some(master) => {
                    // Same seed `Wallet::add_bls_account` /
                    // `add_eddsa_account` derive: the account-level
                    // secp256k1 private-key bytes at the DIP-3 path.
                    let secp = Secp256k1::new();
                    let derived = master.derive_priv(&secp, &account_path).map_err(|e| {
                        PlatformWalletError::KeyDerivation(format!(
                            "failed to derive provider account xpriv at {account_path}: {e}"
                        ))
                    })?;
                    Zeroizing::new(derived.private_key.secret_bytes())
                }
                None => {
                    // Resident key-bearing wallet — derive the account
                    // seed from its own root. Errors here for a watch-only
                    // / external-signable wallet (no resident private
                    // key), which the caller must instead service with a
                    // `resolved_master`.
                    let secret = state
                        .wallet()
                        .derive_private_key(&account_path)
                        .map_err(|e| {
                            PlatformWalletError::KeyDerivation(format!(
                                "failed to derive provider account key at {account_path} from \
                                 resident wallet: {e}"
                            ))
                        })?;
                    Zeroizing::new(secret.secret_bytes())
                }
            };
            Some(seed)
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

                // Non-hardened index — the pool's `AddressPoolType::Absent`.
                let child = ChildNumber::from_normal_idx(index).map_err(|e| {
                    PlatformWalletError::KeyDerivation(format!(
                        "invalid operator key index {index}: {e}"
                    ))
                })?;

                match account_seed {
                    // Private reveal: rebuild the BLS master from the
                    // account seed exactly as `Wallet::add_bls_account`
                    // does, then derive the child.
                    Some(seed) => {
                        let bls_master = ExtendedBLSPrivKey::new_master(network, seed.as_ref())
                            .map_err(|e| {
                                PlatformWalletError::KeyDerivation(format!(
                                    "failed to build BLS master from operator seed: {e}"
                                ))
                            })?;
                        let derived = bls_master.derive_priv(child).map_err(|e| {
                            PlatformWalletError::KeyDerivation(format!(
                                "failed to derive BLS operator key at index {index}: {e}"
                            ))
                        })?;
                        let public_key_bytes = derived.public_key_bytes().to_vec();

                        // The private-path public key must equal the
                        // watch-only `ckd_pub` derivation the pool /
                        // ProRegTx matcher use — proves the seed and the
                        // BLS ckd are consistent.
                        debug_assert_eq!(
                            account
                                .bls_public_key
                                .derive_pub(child)
                                .map(|p| p.to_bytes())
                                .ok(),
                            Some(derived.public_key_bytes()),
                            "BLS priv-derived operator pubkey diverged from ckd_pub"
                        );

                        let private_key = include_private
                            .then(|| Zeroizing::new(derived.private_key.to_be_bytes().to_vec()));

                        Ok(ProviderDerivedKey {
                            index,
                            public_key_bytes,
                            node_id: None,
                            private_key,
                        })
                    }
                    // Public-only: non-hardened `ckd_pub` off the account
                    // xpub — no seed / resolver needed for BLS.
                    None => {
                        let public_key_bytes = account
                            .bls_public_key
                            .derive_pub(child)
                            .map_err(|e| {
                                PlatformWalletError::KeyDerivation(format!(
                                    "failed to derive BLS operator public key at index \
                                     {index}: {e}"
                                ))
                            })?
                            .to_bytes()
                            .to_vec();
                        Ok(ProviderDerivedKey {
                            index,
                            public_key_bytes,
                            node_id: None,
                            private_key: None,
                        })
                    }
                }
            }
            ProviderKeyKind::PlatformNode => {
                // Existence check — a missing account is a caller error,
                // not a derivation failure. (The stored xpub itself is
                // only needed for the debug cross-check below.)
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
                let seed = account_seed.expect("platform-node derivation always seeds");

                let ed_master = ExtendedEd25519PrivKey::new_master(network, seed.as_ref())
                    .map_err(|e| {
                        PlatformWalletError::KeyDerivation(format!(
                            "failed to build Ed25519 master from platform-node seed: {e}"
                        ))
                    })?;

                // In debug, confirm the seed reproduces the stored
                // account xpub (proves the DIP-3 account path is right).
                #[cfg(debug_assertions)]
                {
                    use key_wallet::derivation_slip10::ExtendedEd25519PubKey;
                    if let (Ok(acct_pub), Some(account)) = (
                        ExtendedEd25519PubKey::from_priv(&ed_master),
                        state.wallet().accounts.eddsa_account_of_type(account_type),
                    ) {
                        debug_assert_eq!(
                            acct_pub.public_key.to_bytes(),
                            account.ed25519_public_key.public_key.to_bytes(),
                            "Ed25519 account seed diverged from stored account xpub"
                        );
                    }
                }

                // SLIP-10 Ed25519 is hardened-only; the pool derives
                // platform-node keys at a single hardened index
                // (`AddressPoolType::AbsentHardened`).
                let child = ChildNumber::from_hardened_idx(index).map_err(|e| {
                    PlatformWalletError::KeyDerivation(format!(
                        "invalid platform-node key index {index}: {e}"
                    ))
                })?;
                let derived = ed_master.derive_priv(&[child]).map_err(|e| {
                    PlatformWalletError::KeyDerivation(format!(
                        "failed to derive Ed25519 platform-node key at index {index}: {e}"
                    ))
                })?;
                let verifying = derived.public_key().map_err(|e| {
                    PlatformWalletError::KeyDerivation(format!(
                        "failed to obtain Ed25519 public key at index {index}: {e}"
                    ))
                })?;
                let public_key_bytes = verifying.to_bytes().to_vec();

                // The 20-byte platform node id = hash160(ed25519 pubkey),
                // exactly what the ProRegTx `platform_node_id` matcher
                // compares against (`Payload::PubkeyHash`).
                let node_id: [u8; 20] = hash160::Hash::hash(&public_key_bytes).to_byte_array();

                let private_key =
                    include_private.then(|| Zeroizing::new(derived.private_key.to_vec()));

                Ok(ProviderDerivedKey {
                    index,
                    public_key_bytes,
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
    use dashcore::hashes::{hash160, Hash};
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

    /// The two load-bearing invariants for platform-node keys, pinned so an
    /// upstream SLIP-10 / DIP-9 derivation regression can't silently hand
    /// out wrong key material: `node_id == hash160(pubkey)` at every index,
    /// and a stable golden pubkey/node-id for index 0 on testnet.
    #[test]
    fn platform_node_keys_are_consistent_and_pinned() {
        let wallet = seed_bearing_wallet(Network::Testnet);
        let keys = derive_platform_node_public_keys(&wallet, Network::Testnet, 20)
            .expect("platform-node derivation");

        assert_eq!(keys.len(), 20, "requested 20 keys");
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(k.index, i as u32, "index ordering");
            let expected_node_id: [u8; 20] = hash160::Hash::hash(&k.public_key).to_byte_array();
            assert_eq!(
                k.node_id, expected_node_id,
                "node_id must be hash160(ed25519 pubkey) at index {i}"
            );
        }

        // All indices produce distinct keys (hardened SLIP-10 children).
        let mut pubs: Vec<[u8; 32]> = keys.iter().map(|k| k.public_key).collect();
        pubs.sort();
        pubs.dedup();
        assert_eq!(pubs.len(), 20, "all 20 platform-node keys must be distinct");

        // Golden vector — regenerating the same mnemonic must reproduce
        // these exact bytes. A break here means the derivation path or an
        // upstream crate changed.
        assert_eq!(
            hex::encode(keys[0].public_key),
            "fb91ae39aba2a1b8f68016833bfcfcec8516d634237f5842a21b03c225e2b092",
            "platform-node index-0 pubkey golden vector"
        );
        assert_eq!(
            hex::encode(keys[0].node_id),
            "bb241cb734e78cfc8c537226322b1492d0458678",
            "platform-node index-0 node-id golden vector"
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
