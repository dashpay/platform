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
//! This module is a thin adapter over key-wallet's **gate-free provider
//! key-derivation entry points** (rust-dashcore #881, issue #880). Those
//! functions derive the BLS operator and Ed25519 platform-node keys in
//! each curve's own HD scheme, are wallet-state-agnostic (no
//! `is_watch_only` gate), and match dashbls / DashSync:
//!
//! - **Operator public** at index `i` = [`BLSAccount::operator_public_key_at`]
//!   — the account xpub's non-hardened legacy `ckd_pub` child; no seed.
//! - **Operator private** at `i` = [`BLSAccount::operator_private_key_at`]
//!   — raw seed → BLS master → legacy DIP-3 account path → non-hardened
//!   legacy child; no account state.
//! - **Platform node** at `i` = [`EdDSAAccount::platform_node_key_at`] —
//!   raw seed → SLIP-10 master → DIP-3 account path → hardened child `i'`;
//!   no account state.
//!
//! We use these directly rather than the account's older
//! `derive_bls_key_at_index` / `derive_from_seed_*` convenience wrappers,
//! which gate on `is_watch_only` *asymmetrically* (the public wrapper
//! needs a watch-only account; `derive_from_seed_*` needs a
//! **non**-watch-only one) and so can't both serve a resident and a
//! restored external-signable wallet. The #880 entry points have no such
//! gate.
//!
//! Because the operator public key comes from the stored account xpub
//! while the operator private key comes from the raw seed, the
//! private-reveal path runs an **always-on cross-check** that the two
//! agree on the same G1 point (`operator_private_key_at(..).public_key()`
//! vs `operator_public_key_at(..)`) — the entry points derive
//! independently and do not verify against each other, so a stale /
//! corrupt persisted xpub would otherwise let a mismatched (public,
//! private) pair escape. On mismatch we return an error rather than a key.
//!
//! The raw 64-byte seed is obtained two ways, matching
//! [`derive_core_address_private_key`](PlatformWallet::derive_core_address_private_key):
//! `Some(seed)` for external-signable / watch-only wallets whose mnemonic
//! the caller resolved on demand; `None` to take a resident key-bearing
//! wallet's own [`wallet_seed_bytes`](key_wallet::wallet::Wallet::wallet_seed_bytes).
//! The seed and any returned scalar are wrapped in [`Zeroizing`] so they
//! are scrubbed when dropped.

use key_wallet::account::derivation::AccountDerivation;
use key_wallet::account::{AccountType, BLSAccount, EdDSAAccount};
use key_wallet::bip32::{ChildNumber, ExtendedPrivKey};
use key_wallet::managed_account::address_pool::AddressPoolType;
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
/// the 32-byte public key + 20-byte Tenderdash node id
/// (`SHA256(pubkey)[..20]`, #884) per hardened index.
///
/// Used at registration (`PlatformWalletManager::register_wallet`)
/// to snapshot the pool while the seed is available, because the
/// platform-node curve is hardened-only and the pool can never be
/// extended later from an external-signable / watch-only wallet. Each key
/// is the gate-free [`EdDSAAccount::platform_node_key_at`] entry point
/// (rust-dashcore #881) — raw seed → SLIP-10 master → DIP-3 account path
/// → hardened child `i'` — so it is byte-identical to what
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
    // not a derivation failure (the seed-based entry point below needs no
    // account state).
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
        let signing_key = EdDSAAccount::platform_node_key_at(seed.as_ref(), network, index)
            .map_err(|e| {
                PlatformWalletError::KeyDerivation(format!(
                    "failed to derive Ed25519 platform-node key at index {index}: {e}"
                ))
            })?;
        let public_key: [u8; 32] = signing_key.verifying_key().to_bytes();
        // The 20-byte platform node id = SHA256(ed25519 pubkey)[..20] — the
        // Tenderdash/CometBFT convention (rust-dashcore #884) that a ProRegTx
        // `platform_node_id` field carries, NOT a hash160.
        let node_id: [u8; 20] =
            dashcore::PlatformNodeId::from_ed25519_public_key(&public_key).to_byte_array();
        out.push(ProviderPlatformNodePubKey {
            index,
            public_key,
            node_id,
        });
    }
    Ok(out)
}

/// Insert a pre-derived platform-node (Ed25519) key batch into the
/// managed `ProviderPlatformKeys` pool of a `ManagedWalletInfo` so those
/// keys flow out as ordinary typed [`PublicKeyType::EdDSA`] address rows.
///
/// Called at registration (`PlatformWalletManager::register_wallet`),
/// AFTER `ManagedWalletInfo::from_wallet` has created the (empty)
/// platform-node account but BEFORE the address-pool snapshot is taken,
/// so the EdDSA keys ride the normal address-persistence pipeline (they
/// are persisted as typed core-address rows, restored by
/// `restore_core_address_pools`) and the in-memory pool matches what a
/// restore reconstructs. SLIP-10 Ed25519 is hardened-only, so this pool
/// can never be extended later from the account xpub — this one-time
/// registration-side population is the only source of these keys.
///
/// Each entry becomes a pool address keyed by its hardened index: the
/// address is the P2PKH payload of the entry's 20-byte platform node id
/// (`SHA256(pubkey)[..20]`, rust-dashcore #884), the path is the
/// `ProviderPlatformKeys` account path plus the hardened index, and the
/// typed key is `PublicKeyType::EdDSA(pubkey)`. The `node_id` field is
/// trusted here — it was just computed by
/// [`derive_platform_node_public_keys`] from the same pubkey. Mirrors the
/// `AddressInfo` shape key-wallet's own EdDSA pool derivation produces, so
/// a freshly-registered wallet and a restored one carry byte-identical
/// pool entries.
///
/// A no-op when the wallet has no managed platform-node account or no
/// `AbsentHardened` pool on it.
///
/// # Errors
/// [`PlatformWalletError::KeyDerivation`] if the `ProviderPlatformKeys`
/// account derivation path or a hardened child index cannot be built.
#[cfg(feature = "eddsa")]
pub fn populate_platform_node_pool(
    wallet_info: &mut key_wallet::wallet::managed_wallet_info::ManagedWalletInfo,
    keys: &[ProviderPlatformNodePubKey],
    network: key_wallet::Network,
) -> Result<(), PlatformWalletError> {
    use dashcore::hashes::Hash;
    use key_wallet::managed_account::address_pool::{AddressPoolType, AddressState, PublicKeyType};
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::AddressInfo;

    if keys.is_empty() {
        return Ok(());
    }
    let Some(account) = wallet_info.accounts.provider_platform_keys.as_mut() else {
        return Ok(());
    };

    let account_path = AccountType::ProviderPlatformKeys
        .derivation_path(network)
        .map_err(|e| {
            PlatformWalletError::KeyDerivation(format!(
                "failed to build ProviderPlatformKeys account path: {e:?}"
            ))
        })?;
    let base_children: Vec<key_wallet::bip32::ChildNumber> = account_path.as_ref().to_vec();

    let mut infos: Vec<AddressInfo> = Vec::with_capacity(keys.len());
    for key in keys {
        // Trust the entry's node id — it was just derived from `public_key`.
        let payload = dashcore::address::Payload::PubkeyHash(
            dashcore::PubkeyHash::from_byte_array(key.node_id),
        );
        let address = dashcore::Address::new(network, payload);
        let script_pubkey = address.script_pubkey();
        let child = key_wallet::bip32::ChildNumber::from_hardened_idx(key.index).map_err(|e| {
            PlatformWalletError::KeyDerivation(format!(
                "failed to build hardened child index {}: {e:?}",
                key.index
            ))
        })?;
        let mut children = base_children.clone();
        children.push(child);
        let path = key_wallet::bip32::DerivationPath::from(children);
        infos.push(AddressInfo {
            address,
            script_pubkey,
            public_key: Some(PublicKeyType::EdDSA(key.public_key.to_vec())),
            index: key.index,
            path,
            state: AddressState::Available,
            tx_count: 0,
            total_received: 0,
            total_sent: 0,
            balance: 0,
            label: None,
            metadata: std::collections::BTreeMap::new(),
        });
    }

    // The platform-node pool is `AbsentHardened` (SLIP-10 hardened-only).
    if let Some(pool) = account
        .managed_account_type_mut()
        .address_pools_mut()
        .into_iter()
        .find(|p| p.pool_type == AddressPoolType::AbsentHardened)
    {
        for info in infos {
            let idx = info.index;
            pool.address_index.insert(info.address.clone(), idx);
            pool.script_pubkey_index
                .insert(info.script_pubkey.clone(), idx);
            pool.highest_generated = Some(pool.highest_generated.map_or(idx, |h| h.max(idx)));
            pool.addresses.insert(idx, info);
        }
    }
    Ok(())
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
    /// secp256k1 masternode owner keys
    /// ([`AccountType::ProviderOwnerKeys`], FFI tag 9).
    Owner,
    /// secp256k1 masternode voting keys
    /// ([`AccountType::ProviderVotingKeys`], FFI tag 8).
    Voting,
}

impl ProviderKeyKind {
    /// The upstream account type this kind maps to.
    fn account_type(self) -> AccountType {
        match self {
            ProviderKeyKind::Operator => AccountType::ProviderOperatorKeys,
            ProviderKeyKind::PlatformNode => AccountType::ProviderPlatformKeys,
            ProviderKeyKind::Owner => AccountType::ProviderOwnerKeys,
            ProviderKeyKind::Voting => AccountType::ProviderVotingKeys,
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
    /// `ExtendedBLSPubKey::to_bytes_legacy` (key-wallet #879) — never
    /// transformed in Swift.
    pub legacy_public_key_bytes: Option<Vec<u8>>,
    /// The 20-byte platform node id — `SHA256(ed25519 pubkey)[..20]`, the
    /// Tenderdash/CometBFT convention (rust-dashcore #884), which the
    /// ProRegTx `platform_node_id` field carries and the pool matcher
    /// compares against. `Some` for [`ProviderKeyKind::PlatformNode`];
    /// `None` for [`ProviderKeyKind::Operator`] (whose on-chain field is
    /// the raw 48-byte BLS public key, not a hash).
    pub node_id: Option<[u8; 20]>,
    /// Raw private-key scalar (32 bytes), present only when the caller
    /// asked for it. BLS / Ed25519 keys have no WIF, so for those this is
    /// the only private form. Zeroized on drop.
    pub private_key: Option<Zeroizing<Vec<u8>>>,
    /// The key's P2PKH address. `Some` only for the secp256k1 families
    /// ([`ProviderKeyKind::Owner`] / [`ProviderKeyKind::Voting`]), whose
    /// keys appear on-chain as the ProRegTx owner / voting addresses;
    /// `None` for BLS / Ed25519, which have no address form.
    pub address: Option<String>,
    /// WIF encoding of `private_key`, on the same terms: `Some` only for
    /// the secp256k1 families and only when a private key was requested.
    /// Encoded here rather than by the caller so the network byte and
    /// compression flag have one home. Zeroized on drop.
    pub private_key_wif: Option<Zeroizing<String>>,
}

/// The operator seed↔xpub cross-check guard, factored out so it can be
/// unit-tested without a full [`PlatformWallet`].
///
/// Derives the operator secret at `index` from the raw `seed` via the
/// gate-free #881 entry point, then verifies its public key equals
/// `xpub_modern` (the modern serialization the account xpub produced).
/// The two #881 entry points derive independently and do **not** verify
/// against each other, so this refuses to return a mismatched
/// `(public, private)` pair — a stale / corrupt persisted xpub would
/// otherwise let one escape (signing would then produce a signature that
/// doesn't verify against the displayed public key).
///
/// Returns the secret's big-endian scalar bytes when `include_private`,
/// `None` otherwise; [`PlatformWalletError::KeyDerivation`] on mismatch or
/// derivation failure.
fn checked_operator_private_bytes_at(
    xpub_modern: &[u8],
    seed: &[u8],
    network: key_wallet::Network,
    index: u32,
    include_private: bool,
) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformWalletError> {
    let secret = BLSAccount::operator_private_key_at(seed, network, index).map_err(|e| {
        PlatformWalletError::KeyDerivation(format!(
            "failed to derive BLS operator key at index {index}: {e}"
        ))
    })?;
    if secret.public_key().to_bytes().as_slice() != xpub_modern {
        return Err(PlatformWalletError::KeyDerivation(format!(
            "BLS operator key at index {index}: seed-derived public key does not match the \
             account xpub derivation — the wallet seed and its stored operator account xpub \
             disagree; refusing to return a mismatched key"
        )));
    }
    Ok(include_private.then(|| Zeroizing::new(secret.to_be_bytes().to_vec())))
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
    /// The derivation delegates to key-wallet's gate-free provider-key
    /// entry points (rust-dashcore #881) so every per-index key is
    /// byte-identical to what `Wallet::from_mnemonic` account creation
    /// produces; it never feeds a secp256k1 child scalar into a
    /// BLS/Ed25519 master (the pre-#879 hybrid).
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

                // The operator public key at this index — the account
                // xpub's non-hardened legacy `ckd_pub` child, via the
                // gate-free #881 entry point (works for resident and
                // restored / watch-only accounts). Both serializations of
                // the same G1 point come off the returned extended key.
                let xpub = account.operator_public_key_at(index).map_err(|e| {
                    PlatformWalletError::KeyDerivation(format!(
                        "failed to derive BLS operator public key at index {index}: {e}"
                    ))
                })?;
                let public_key_bytes = xpub.to_bytes().to_vec();
                // Same G1 point in Dash legacy serialization (key-wallet
                // #879) — for the "BLS Public Key (Legacy)" display row.
                let legacy_public_key_bytes = Some(xpub.to_bytes_legacy().to_vec());

                let private_key = match &seed {
                    // Private reveal: the operator secret scalar from the
                    // raw seed (#881 entry point), gated by the always-on
                    // seed↔xpub cross-check (`checked_operator_private_bytes_at`).
                    Some(seed) => checked_operator_private_bytes_at(
                        &public_key_bytes,
                        seed.as_ref(),
                        network,
                        index,
                        include_private,
                    )?,
                    // Public-only: no seed, no private scalar.
                    None => None,
                };

                Ok(ProviderDerivedKey {
                    index,
                    public_key_bytes,
                    legacy_public_key_bytes,
                    node_id: None,
                    private_key,
                    // BLS keys have no address or WIF form.
                    address: None,
                    private_key_wif: None,
                })
            }
            ProviderKeyKind::PlatformNode => {
                // Existence check — a missing account is a caller error,
                // not a derivation failure (the seed-based entry point below
                // needs no account state).
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

                // Gate-free #881 entry point (the exact routine
                // `derive_platform_node_public_keys` uses, so the per-index
                // reveal can never diverge from the persisted batch): raw
                // seed → SLIP-10 master → DIP-3 account path → hardened
                // child `index'`. No account state involved.
                let signing_key = EdDSAAccount::platform_node_key_at(seed.as_ref(), network, index)
                    .map_err(|e| {
                        PlatformWalletError::KeyDerivation(format!(
                            "failed to derive Ed25519 platform-node key at index {index}: {e}"
                        ))
                    })?;
                let verifying_bytes: [u8; 32] = signing_key.verifying_key().to_bytes();
                let public_key_bytes = verifying_bytes.to_vec();

                // The 20-byte platform node id = SHA256(ed25519 pubkey)[..20]
                // — the Tenderdash/CometBFT convention (rust-dashcore #884)
                // the ProRegTx `platform_node_id` field carries, NOT a hash160.
                let node_id: [u8; 20] =
                    dashcore::PlatformNodeId::from_ed25519_public_key(&verifying_bytes)
                        .to_byte_array();

                let private_key =
                    include_private.then(|| Zeroizing::new(signing_key.to_bytes().to_vec()));

                Ok(ProviderDerivedKey {
                    index,
                    public_key_bytes,
                    // Ed25519 platform-node keys have no BLS legacy variant.
                    legacy_public_key_bytes: None,
                    node_id: Some(node_id),
                    private_key,
                    // Ed25519 platform-node keys have no address or WIF form.
                    address: None,
                    private_key_wif: None,
                })
            }
            ProviderKeyKind::Owner | ProviderKeyKind::Voting => {
                let account = state
                    .wallet()
                    .accounts
                    .account_of_type(account_type)
                    .ok_or_else(|| {
                        PlatformWalletError::AddressNotFound(format!(
                            "wallet has no secp256k1 {} account",
                            match kind {
                                ProviderKeyKind::Owner => "provider-owner-keys",
                                _ => "provider-voting-keys",
                            }
                        ))
                    })?;

                // Provider key accounts hold one pool with no
                // internal/external split, derived at a non-hardened child
                // index — so the public side comes off the account xpub and
                // needs no seed, exactly like the BLS operator family.
                let public_key = account
                    .derive_public_key_at(AddressPoolType::Absent, index, None)
                    .map_err(|e| {
                        PlatformWalletError::KeyDerivation(format!(
                            "failed to derive provider public key at index {index}: {e}"
                        ))
                    })?;
                let address = account
                    .derive_address_at(AddressPoolType::Absent, index, None)
                    .map_err(|e| {
                        PlatformWalletError::KeyDerivation(format!(
                            "failed to derive provider address at index {index}: {e}"
                        ))
                    })?;

                let (private_key, private_key_wif) = match &seed {
                    Some(seed) => {
                        // Derived inline rather than through
                        // `Account::derive_from_seed_private_key_at`, which
                        // gates on `is_watch_only` and so refuses exactly the
                        // external-signable wallets this seed argument exists
                        // to serve (see the module docs on the BLS / Ed25519
                        // families using the gate-free #881 entry points for
                        // the same reason; secp256k1 has no such entry point).
                        //
                        // Shape mirrors `BLSAccount::operator_private_key_at`:
                        // raw seed → master xpriv → the account's own DIP-3
                        // path → non-hardened child `index`. The account path
                        // is resolved from the account type, so it is applied
                        // exactly once.
                        let master =
                            ExtendedPrivKey::new_master(network, seed.as_ref()).map_err(|e| {
                                PlatformWalletError::KeyDerivation(format!(
                                    "failed to build master xpriv: {e}"
                                ))
                            })?;
                        let secp = dashcore::key::Secp256k1::new();
                        // The ACCOUNT's own path, not `account_type.derivation_path(network)`.
                        // The public side above comes off `account.account_xpub`, built from
                        // this path using the ACCOUNT's network; resolving it again from the
                        // type plus the wallet's network makes the coin type an independent
                        // input, and the two disagree the moment those networks differ.
                        // Taking it from the account is agreement by construction, so the
                        // cross-check below verifies the seed rather than the path.
                        let account_path = account.derivation_path().map_err(|e| {
                            PlatformWalletError::KeyDerivation(format!(
                                "failed to resolve the provider account path: {e}"
                            ))
                        })?;
                        let child = ChildNumber::from_normal_idx(index).map_err(|e| {
                            PlatformWalletError::KeyDerivation(format!(
                                "invalid provider key index {index}: {e}"
                            ))
                        })?;
                        let mut child_xpriv = master
                            .derive_priv(&secp, &account_path)
                            .and_then(|acct| {
                                let child_path: key_wallet::bip32::DerivationPath =
                                    vec![child].into();
                                acct.derive_priv(&secp, &child_path)
                            })
                            .map_err(|e| {
                                PlatformWalletError::KeyDerivation(format!(
                                    "failed to derive provider private key at index {index}: {e}"
                                ))
                            })?;
                        let mut private = child_xpriv.to_priv();
                        let derived_public = private.public_key(&secp);

                        // Produce every output while the scalar is still live,
                        // then erase it. `dashcore::PrivateKey` is `Copy` and
                        // has no `Drop`, so unlike the `Zeroizing` outputs below
                        // nothing scrubs it on the way out — the raw scalar
                        // would otherwise be left behind in this frame. Both
                        // copies are erased: the extended key it came from, and
                        // the `PrivateKey` itself.
                        let outputs = if derived_public == public_key {
                            Some((
                                Zeroizing::new(private.inner.secret_bytes().to_vec()),
                                Zeroizing::new(private.to_wif()),
                            ))
                        } else {
                            None
                        };
                        private.inner.non_secure_erase();
                        child_xpriv.private_key.non_secure_erase();

                        let Some((scalar, wif)) = outputs else {
                            return Err(PlatformWalletError::KeyDerivation(format!(
                                "provider key at index {index} is inconsistent: the seed-derived \
                                 private key's public key does not match the account xpub's"
                            )));
                        };
                        (Some(scalar), Some(wif))
                    }
                    None => (None, None),
                };

                Ok(ProviderDerivedKey {
                    index,
                    public_key_bytes: public_key.to_bytes(),
                    // secp256k1 keys have no BLS legacy variant.
                    legacy_public_key_bytes: None,
                    node_id: None,
                    private_key,
                    address: Some(address.to_string()),
                    private_key_wif,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::mnemonic::Mnemonic;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    use key_wallet::Network;

    // Canonical all-`abandon` BIP-39 test vector — deterministic, so the
    // derived key material below is a stable golden vector.
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    // A second, distinct BIP-39 vector (the standard `0x7f7f…` entropy
    // test mnemonic) whose seed does NOT correspond to `TEST_MNEMONIC`'s
    // account xpub — used for the operator cross-check negative path.
    const TEST_MNEMONIC_B: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";

    fn seed_bearing_wallet(network: Network) -> Wallet {
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC).expect("valid test mnemonic");
        Wallet::from_mnemonic(mnemonic, network, WalletAccountCreationOptions::Default)
            .expect("wallet construction")
    }

    fn second_seed_bearing_wallet(network: Network) -> Wallet {
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC_B)
            .expect("valid test mnemonic B");
        Wallet::from_mnemonic(mnemonic, network, WalletAccountCreationOptions::Default)
            .expect("wallet B construction")
    }

    /// The load-bearing invariants for platform-node keys, pinned so a
    /// derivation regression can't silently hand out wrong key material:
    /// `node_id == SHA256(pubkey)[..20]` (the Tenderdash convention,
    /// rust-dashcore #884) at every index, all indices distinct, the
    /// snapshot's per-index public key corresponds to the #881 seed entry
    /// point, and golden secret / node-id vectors pinned to key-wallet's
    /// `provider_key_derivation_tests.rs`.
    #[test]
    fn platform_node_keys_are_consistent_and_pinned() {
        let wallet = seed_bearing_wallet(Network::Mainnet);
        let seed = wallet.wallet_seed_bytes().expect("resident seed");

        let keys = derive_platform_node_public_keys(&wallet, Network::Mainnet, 20)
            .expect("platform-node derivation");

        assert_eq!(keys.len(), 20, "requested 20 keys");
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(k.index, i as u32, "index ordering");
            let expected_node_id: [u8; 20] =
                dashcore::PlatformNodeId::from_ed25519_public_key(&k.public_key).to_byte_array();
            assert_eq!(
                k.node_id, expected_node_id,
                "node_id must be SHA256(ed25519 pubkey)[..20] at index {i}"
            );

            // The snapshot's per-index public key must be the public key of
            // the #881 seed entry point at that index — the same routine
            // `derive_provider_key_at_index` uses for the per-index reveal,
            // so the persisted batch and the reveal can't diverge.
            let via_api = EdDSAAccount::platform_node_key_at(&seed, Network::Mainnet, i as u32)
                .expect("platform_node_key_at");
            assert_eq!(
                k.public_key,
                via_api.verifying_key().to_bytes(),
                "snapshot pubkey must equal platform_node_key_at at {i}"
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
        // DashSync/dashwallet-ios produce.
        let sk0 = EdDSAAccount::platform_node_key_at(&seed, Network::Mainnet, 0)
            .expect("platform_node_key_at");
        assert_eq!(
            hex::encode(sk0.to_bytes()),
            "5fa238b12be77347abf9b5957bd902d16c6aaca28d25c4267ffacbd7458dceb1",
            "mainnet platform-node index-0 secret golden (dashbls/SLIP-10 reference)"
        );
        // The snapshot's index-0 public key corresponds to that golden secret.
        assert_eq!(
            keys[0].public_key,
            sk0.verifying_key().to_bytes(),
            "snapshot index-0 pubkey must match the golden secret's public key"
        );
        // Golden pubkey + Tenderdash node-id (SHA256[..20]) for mainnet
        // index 0, pinned to key-wallet's post-#884
        // `provider_key_derivation_tests.rs` reference vectors.
        assert_eq!(
            hex::encode(keys[0].public_key),
            "3130c14339391cf26a68d86879e180ee9a16b660f5aa91f560f67c0abe8cf789",
            "mainnet platform-node index-0 pubkey golden"
        );
        assert_eq!(
            hex::encode(keys[0].node_id),
            "302f2615e6955cce8ed3cff81e8011bfd3a2991f",
            "mainnet platform-node index-0 node-id golden (Tenderdash SHA256[..20], #884)"
        );
    }

    /// Operator (BLS) keys pinned to the dashbls reference vectors from
    /// key-wallet `provider_key_derivation_tests.rs` — the values
    /// DashSync/dashwallet-ios produce — on both networks, via the #881
    /// gate-free entry points this module now calls. Also exercises the
    /// always-on seed↔xpub cross-check (`operator_private_key_at(..)`'s
    /// public key must equal `operator_public_key_at(..)`), the guard that
    /// refuses to hand out a mismatched (public, private) pair.
    #[test]
    fn operator_keys_match_dashbls_reference_and_cross_check() {
        // --- Mainnet (`m/9'/5'/3'/3'/0`) ---
        let wallet = seed_bearing_wallet(Network::Mainnet);
        let seed = wallet.wallet_seed_bytes().expect("resident seed");
        let account = wallet
            .accounts
            .bls_account_of_type(AccountType::ProviderOperatorKeys)
            .expect("operator account");

        // Public: the gate-free #881 entry point (works on this resident
        // account and on watch-only accounts alike).
        let xpub0 = account.operator_public_key_at(0).expect("operator public");
        assert_eq!(
            hex::encode(xpub0.to_bytes_legacy()),
            "078cad04aae29eb76171937eb7101452b401b026efbc27db840f130374e6a9ec8443d917277f8921e0ba6678a7709875",
            "operator key0 legacy pubkey golden (dashbls reference)"
        );
        assert_eq!(
            hex::encode(xpub0.to_bytes()),
            "878cad04aae29eb76171937eb7101452b401b026efbc27db840f130374e6a9ec8443d917277f8921e0ba6678a7709875",
            "operator key0 modern pubkey golden (dashbls reference)"
        );

        // Private: the gate-free #881 seed entry point (no account state).
        let sk0 = BLSAccount::operator_private_key_at(&seed, Network::Mainnet, 0)
            .expect("operator private");
        assert_eq!(
            hex::encode(sk0.to_be_bytes()),
            "11122e1ad656d0610ce0f80d40da874d67ea656a3e66ed371c915ec3a488a43a",
            "operator key0 secret golden (dashbls reference)"
        );
        // The always-on cross-check the adapter runs: the seed-derived
        // secret's public key must equal the account-xpub-derived public.
        assert_eq!(
            sk0.public_key().to_bytes(),
            xpub0.public_key.to_bytes(),
            "seed-derived operator pubkey must equal the account-xpub derivation"
        );

        // --- Testnet (`m/9'/1'/3'/3'/0`) ---
        let wallet_t = seed_bearing_wallet(Network::Testnet);
        let seed_t = wallet_t.wallet_seed_bytes().expect("resident seed");
        let account_t = wallet_t
            .accounts
            .bls_account_of_type(AccountType::ProviderOperatorKeys)
            .expect("operator account");
        let xpub0_t = account_t
            .operator_public_key_at(0)
            .expect("operator public");
        assert_eq!(
            hex::encode(xpub0_t.to_bytes_legacy()),
            "09d8beabae708de1638487f1aff44b38e8c07d9b09f22d76329d6c8ec01e2ad4d030b660bca40ddbd222373a72c5bcef",
            "testnet operator key0 legacy pubkey golden (dashbls reference)"
        );
        let sk0_t = BLSAccount::operator_private_key_at(&seed_t, Network::Testnet, 0)
            .expect("operator private");
        assert_eq!(
            hex::encode(sk0_t.to_be_bytes()),
            "3346dfd71627f9f31cad3ee66fe7b673c32cb077b2eb38c621d7e61c30e46dbd",
            "testnet operator key0 secret golden (dashbls reference)"
        );
        assert_eq!(
            sk0_t.public_key().to_bytes(),
            xpub0_t.public_key.to_bytes(),
            "testnet seed-derived operator pubkey must equal the account-xpub derivation"
        );
    }

    /// Negative path for the operator seed↔xpub cross-check that
    /// `derive_provider_key_at_index` runs on every private reveal (via
    /// `checked_operator_private_bytes_at`): an operator private key
    /// derived from a seed that does NOT correspond to the account xpub
    /// must be REJECTED with `KeyDerivation`, never returned as a
    /// mismatched (public, private) pair. Pins the guard so it can't be
    /// silently weakened.
    #[test]
    fn operator_cross_check_rejects_mismatched_seed() {
        // Account xpub from mnemonic A.
        let wallet_a = seed_bearing_wallet(Network::Mainnet);
        let account_a = wallet_a
            .accounts
            .bls_account_of_type(AccountType::ProviderOperatorKeys)
            .expect("operator account");
        let xpub_a_modern = account_a
            .operator_public_key_at(0)
            .expect("operator public")
            .to_bytes()
            .to_vec();
        let seed_a = wallet_a.wallet_seed_bytes().expect("seed A");

        // A distinct wallet whose seed does not match A's xpub.
        let wallet_b = second_seed_bearing_wallet(Network::Mainnet);
        let seed_b = wallet_b.wallet_seed_bytes().expect("seed B");
        assert_ne!(seed_a, seed_b, "the two fixtures must differ");

        // Mismatch: A's xpub, B's seed → the guard must reject.
        let err =
            checked_operator_private_bytes_at(&xpub_a_modern, &seed_b, Network::Mainnet, 0, true)
                .expect_err("a seed not matching the account xpub must be rejected");
        assert!(
            matches!(err, PlatformWalletError::KeyDerivation(_)),
            "cross-check mismatch must surface as KeyDerivation, got {err:?}"
        );

        // Control: A's own seed passes and yields the golden secret.
        let ok =
            checked_operator_private_bytes_at(&xpub_a_modern, &seed_a, Network::Mainnet, 0, true)
                .expect("the matching seed must pass the cross-check");
        assert_eq!(
            hex::encode(&*ok.expect("private scalar returned when include_private")),
            "11122e1ad656d0610ce0f80d40da874d67ea656a3e66ed371c915ec3a488a43a",
            "the matching seed returns the golden operator secret"
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

    /// Registration-side: populating the managed platform-node pool from a
    /// derived batch must land typed [`PublicKeyType::EdDSA`] rows in the
    /// account's `AbsentHardened` pool — the source the address-persistence
    /// pipeline snapshots and the masternode-ownership scan reads. Pins the
    /// pool shape (typed key + node-id-derived address + advancing
    /// watermark) independent of any FFI plumbing.
    #[cfg(feature = "eddsa")]
    #[test]
    fn populate_platform_node_pool_lands_typed_eddsa_rows() {
        use key_wallet::managed_account::address_pool::{AddressPoolType, PublicKeyType};
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

        let wallet = seed_bearing_wallet(Network::Mainnet);
        let keys = derive_platform_node_public_keys(&wallet, Network::Mainnet, 5)
            .expect("platform-node derivation");

        let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
        populate_platform_node_pool(&mut info, &keys, Network::Mainnet)
            .expect("populate must succeed");

        let account = info
            .accounts
            .provider_platform_keys
            .as_ref()
            .expect("managed platform-node account must exist");
        let pool = account
            .managed_account_type()
            .address_pools()
            .iter()
            .find(|p| p.pool_type == AddressPoolType::AbsentHardened)
            .cloned()
            .expect("AbsentHardened pool must exist");

        assert_eq!(pool.addresses.len(), keys.len(), "one row per derived key");
        for k in &keys {
            let entry = pool
                .addresses
                .get(&k.index)
                .expect("row present at derived index");
            match &entry.public_key {
                Some(PublicKeyType::EdDSA(bytes)) => assert_eq!(
                    bytes.as_slice(),
                    &k.public_key,
                    "typed Ed25519 key must be stored byte-for-byte"
                ),
                other => panic!("expected a typed EdDSA row, got {other:?}"),
            }
            // The row address is the P2PKH payload of the node id, so a
            // scan recomputing SHA256[..20] from the pubkey maps back to
            // this index.
            let expected_node_id =
                dashcore::PlatformNodeId::from_ed25519_public_key(&k.public_key).to_byte_array();
            assert_eq!(
                k.node_id, expected_node_id,
                "node id must be the Tenderdash SHA256[..20]"
            );
        }
        assert_eq!(
            pool.highest_generated,
            Some(keys.len() as u32 - 1),
            "highest_generated must advance to the last populated index"
        );
    }

    /// The secp256k1 (owner / voting) families must derive at the DIP-3
    /// account path applied EXACTLY once: `m/9'/coin'/3'/{2'|1'}/index`.
    ///
    /// This is the invariant the Swift key-wallet route silently violated —
    /// it pre-derived the account root and handed that in as the "master",
    /// so the account path was applied twice and every owner/voting key came
    /// from `m/9'/5'/3'/1'/9'/5'/3'/1'/index`. Nothing failed locally: the
    /// key was well-formed and deterministic, and only a counterparty
    /// holding the real key could tell (Platform rejected the masternode
    /// vote as having no voter identity, because the voter identity is
    /// derived from the signing key's own hash160).
    #[test]
    fn ecdsa_provider_keys_derive_at_the_dip3_path() {
        use key_wallet::bip32::{DerivationPath, ExtendedPrivKey};
        use std::str::FromStr;

        for (network, coin) in [(Network::Mainnet, "5'"), (Network::Testnet, "1'")] {
            let wallet = seed_bearing_wallet(network);
            let seed = wallet.wallet_seed_bytes().expect("resident seed");
            let master = ExtendedPrivKey::new_master(network, &seed).expect("master xpriv");
            let secp = dashcore::key::Secp256k1::new();

            for (kind, family) in [
                (ProviderKeyKind::Owner, "2'"),
                (ProviderKeyKind::Voting, "1'"),
            ] {
                let account = wallet
                    .accounts
                    .account_of_type(kind.account_type())
                    .expect("provider account");

                for index in [0u32, 1, 19] {
                    let derived = account
                        .derive_from_seed_private_key_at(&seed, index)
                        .expect("account-based derivation");

                    let path =
                        DerivationPath::from_str(&format!("m/9'/{coin}/3'/{family}/{index}"))
                            .expect("explicit DIP-3 path");
                    let expected = master.derive_priv(&secp, &path).expect("path derivation");

                    assert_eq!(
                        derived.inner.secret_bytes(),
                        expected.private_key.secret_bytes(),
                        "{kind:?} key at index {index} on {network:?} must come from \
                         m/9'/{coin}/3'/{family}/{index}"
                    );
                }
            }
        }
    }

    /// The doubled path must NOT be what we derive. Guards the specific
    /// regression rather than merely asserting the correct value, so a future
    /// change that reintroduces pre-derivation fails here explicitly.
    #[test]
    fn ecdsa_provider_keys_do_not_double_apply_the_account_path() {
        use key_wallet::bip32::{DerivationPath, ExtendedPrivKey};
        use std::str::FromStr;

        let wallet = seed_bearing_wallet(Network::Mainnet);
        let seed = wallet.wallet_seed_bytes().expect("resident seed");
        let account = wallet
            .accounts
            .account_of_type(ProviderKeyKind::Voting.account_type())
            .expect("voting account");

        let derived = account
            .derive_from_seed_private_key_at(&seed, 19)
            .expect("account-based derivation");

        let master = ExtendedPrivKey::new_master(Network::Mainnet, &seed).expect("master");
        let doubled = master
            .derive_priv(
                &dashcore::key::Secp256k1::new(),
                &DerivationPath::from_str("m/9'/5'/3'/1'/9'/5'/3'/1'/19").expect("doubled path"),
            )
            .expect("doubled derivation");

        assert_ne!(
            derived.inner.secret_bytes(),
            doubled.private_key.secret_bytes(),
            "the account derivation path is being applied twice again"
        );
    }

    /// A WATCH-ONLY account must still derive its secp256k1 private key from
    /// a supplied seed.
    ///
    /// This is the shape the iOS app actually runs: the wallet is
    /// external-signable, its seed lives in the Keychain, and the caller
    /// resolves it on demand. The first cut of this branch went through
    /// `Account::derive_from_seed_private_key_at`, which gates on
    /// `is_watch_only` and so refused exactly the wallets the seed argument
    /// exists to serve — "Watch-only wallet: private keys not available".
    ///
    /// The seed-bearing tests above passed the whole time, which is the point
    /// of this one: they never exercised the gate.
    #[test]
    fn ecdsa_provider_keys_derive_for_a_watch_only_account() {
        use key_wallet::bip32::{DerivationPath, ExtendedPrivKey};
        use std::str::FromStr;

        let wallet = seed_bearing_wallet(Network::Mainnet);
        let seed = wallet.wallet_seed_bytes().expect("resident seed");
        let secp = dashcore::key::Secp256k1::new();
        let master = ExtendedPrivKey::new_master(Network::Mainnet, &seed).expect("master");

        for (kind, family) in [
            (ProviderKeyKind::Owner, "2'"),
            (ProviderKeyKind::Voting, "1'"),
        ] {
            let account_type = kind.account_type();
            let watch_only = wallet
                .accounts
                .account_of_type(account_type)
                .expect("provider account")
                .to_watch_only();
            assert!(
                watch_only.is_watch_only,
                "precondition: account is watch-only"
            );

            // The gated wrapper refuses this account — pinned so a future
            // change back to it fails here rather than on a device.
            assert!(
                watch_only
                    .derive_from_seed_private_key_at(&seed, 19)
                    .is_err(),
                "the gated wrapper is expected to refuse a watch-only account"
            );

            // The path this module actually uses is gate-free and must agree
            // with the explicit DIP-3 path.
            let account_path = account_type
                .derivation_path(Network::Mainnet)
                .expect("account path");
            let child = ChildNumber::from_normal_idx(19).expect("child index");
            let child_path: DerivationPath = vec![child].into();
            let derived = master
                .derive_priv(&secp, &account_path)
                .and_then(|acct| acct.derive_priv(&secp, &child_path))
                .expect("gate-free derivation")
                .to_priv();

            let expected = master
                .derive_priv(
                    &secp,
                    &DerivationPath::from_str(&format!("m/9'/5'/3'/{family}/19"))
                        .expect("explicit path"),
                )
                .expect("path derivation");

            assert_eq!(
                derived.inner.secret_bytes(),
                expected.private_key.secret_bytes(),
                "{kind:?} watch-only derivation must match m/9'/5'/3'/{family}/19"
            );
        }
    }
}
