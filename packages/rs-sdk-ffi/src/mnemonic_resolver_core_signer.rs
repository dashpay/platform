//! Core-side ECDSA [`key_wallet::signer::Signer`] implementation that
//! sources its private keys via the
//! [`MnemonicResolverHandle`](crate::mnemonic_resolver::MnemonicResolverHandle)
//! callback.
//!
//! Same architectural intent as `dash_sdk_sign_with_mnemonic_resolver_and_path`
//! (in `platform-wallet-ffi`) — keep the Swift caller out of the
//! mnemonic-orchestration loop so the `swift-sdk/CLAUDE.md`
//! "no mnemonic round-tripping" rule is satisfied — but exposed as
//! a Rust-side `Signer` trait object so it can plug into key-wallet's
//! signer-driven builders
//! (`build_asset_lock_with_signer`, `TransactionBuilder::build_signed`)
//! and rs-sdk's `_with_signer` state-transition methods.
//!
//! # Lifecycle and unsafety contract
//!
//! - The wrapper stores the resolver pointer as a `usize` and only
//!   reconstitutes the typed pointer inside `sign_ecdsa` / `public_key`.
//!   That keeps the struct `Send + Sync` without an `unsafe impl` on
//!   the field type itself.
//! - The caller is responsible for keeping the resolver handle alive
//!   for the lifetime of every `MnemonicResolverCoreSigner` value that
//!   wraps it. The signer never destroys the resolver — the FFI entry
//!   point that built the signer is also the one that owns the
//!   resolver's lifetime; ownership boundaries are documented at the
//!   call site.
//! - The MnemonicResolverHandle's vtable is `Send + Sync` by contract
//!   (`MnemonicResolverHandle` itself carries `unsafe impl Send + Sync`).
//!
//! # No double-hashing
//!
//! The `Signer::sign_ecdsa` trait contract is explicit (see
//! `key-wallet/src/signer.rs:109-120`): the caller passes a
//! pre-computed 32-byte digest, and the signer signs it directly.
//! rs-dpp's `StateTransition::sign_with_signer` (which is what the
//! `_with_signer` SDK calls eventually invoke) takes care of the
//! `double_sha` pre-image; we just receive the 32 bytes and sign.
//!
//! # Zeroization
//!
//! Every intermediate that carries key material is wiped before the
//! method returns. Two mechanisms cover the different ownership
//! shapes:
//!
//! - **`Zeroizing` wrappers** scrub on `Drop` for the byte-buffer
//!   intermediates: the resolver mnemonic buffer, the BIP-39 seed,
//!   and the final derived 32-byte scalar.
//! - **Explicit `non_secure_erase` calls** scrub the
//!   [`secp256k1::SecretKey`] scalars inside the two intermediate
//!   [`ExtendedPrivKey`] values (master + derived). `ExtendedPrivKey`
//!   has no `Drop` / `Zeroize` impl in `key-wallet`, so falling out
//!   of scope alone would leave those scalars resident; the explicit
//!   wipe at the bottom of `derive_priv` closes the gap. Same
//!   defense is applied at the sign-site for the `SecretKey` copy
//!   `from_slice` creates. A proper fix is a `Zeroize` /
//!   `ZeroizeOnDrop` impl in `dashpay/rust-dashcore`'s
//!   `key-wallet/src/bip32.rs`; until that ships, the local wipes
//!   keep the no-residue invariant true.
//!
//! Combined, no private key bytes survive past the trait-method
//! boundary.

use std::ffi::c_void;
use std::os::raw::c_char;

use async_trait::async_trait;
use key_wallet::bip32::{DerivationPath, ExtendedPrivKey, ExtendedPubKey};
use key_wallet::dashcore::secp256k1::{self, Secp256k1};
use key_wallet::signer::{Signer, SignerMethod};
use key_wallet::Network;
use thiserror::Error;
use zeroize::Zeroizing;

use crate::mnemonic_resolver::{
    mnemonic_resolver_result, MnemonicResolverHandle, MNEMONIC_RESOLVER_BUFFER_CAPACITY,
};
use crate::signer_simple::parse_mnemonic_any_language;

/// Failure modes for the
/// [`MnemonicResolverCoreSigner`](crate::mnemonic_resolver_core_signer::MnemonicResolverCoreSigner)
/// signer.
///
/// Replaces the earlier `type Error = String;` shape with discriminants
/// callers can pattern-match (FFI layer, retry policies). The
/// `ResolverFailed(i32)` variant carries the resolver's raw return code
/// for the unknown-error path so operators can inspect the value
/// without modifying this enum every time a new code is introduced.
///
/// All variants are `Display`-only — no key material ever appears in
/// the error payload (mnemonic / seed / derived scalar are all
/// zeroized before any error can leak out of the resolver flow).
#[derive(Debug, Error)]
pub enum MnemonicResolverSignerError {
    /// Resolver handle was null at signer-construction time and the
    /// caller subsequently invoked a signing method. Production callers
    /// always pass a non-null handle; this variant exists primarily so
    /// tests can exercise the null-safety contract documented on
    /// [`MnemonicResolverCoreSigner::new`].
    #[error("null mnemonic resolver handle")]
    NullHandle,

    /// The Swift-side resolver reported that no mnemonic is stored for
    /// the wallet_id this signer was constructed with. Translates the
    /// FFI `NOT_FOUND` return code.
    #[error("mnemonic not found in keychain for the given wallet_id")]
    NotFound,

    /// The resolver requested a longer output buffer than this signer
    /// allocates. Should be unreachable in practice — the buffer is
    /// sized for the maximum BIP-39 phrase across all supported word
    /// lists. Translates the FFI `BUFFER_TOO_SMALL` return code.
    #[error("mnemonic resolver buffer too small")]
    BufferTooSmall,

    /// Catch-all for unknown resolver return codes. The raw code is
    /// preserved so operators can grep the Swift bridge for the
    /// matching error path.
    #[error("mnemonic resolver failed with code {0}")]
    ResolverFailed(i32),

    /// The resolver returned data that wasn't valid UTF-8 over the
    /// declared length. Indicates a Swift-side encoding bug.
    #[error("invalid UTF-8 in resolved mnemonic")]
    InvalidUtf8,

    /// The resolver returned a buffer whose declared length is zero or
    /// exceeds the capacity. Indicates a Swift-side framing bug.
    #[error("resolver returned invalid mnemonic length {0}")]
    InvalidMnemonicLength(usize),

    /// The resolved string is not a valid BIP-39 mnemonic phrase
    /// (failed checksum or word-list lookup).
    #[error("invalid mnemonic phrase: {0}")]
    InvalidMnemonic(String),

    /// BIP-32 derivation failed — either the master key was
    /// non-conformant or the path produced an invalid child key. The
    /// inner message is the upstream `bip32` library's `Display`.
    #[error("BIP-32 derivation failed: {0}")]
    DerivationFailed(String),

    /// The derived 32-byte scalar is not a valid secp256k1 field
    /// element. Vanishingly improbable in production (the BIP-32
    /// derivation flow already filters non-conformant child keys), but
    /// surfaced explicitly so this case can't masquerade as a generic
    /// error.
    #[error("invalid private key scalar: {0}")]
    InvalidScalar(String),
}

/// `key_wallet::signer::Signer` implementation that derives ECDSA
/// secp256k1 keys from a wallet mnemonic, fetched via a Swift-owned
/// [`MnemonicResolverHandle`].
///
/// Every signing operation is atomic: resolve mnemonic → parse →
/// seed → derive at the requested path → sign (or compute pubkey) →
/// zero all intermediate buffers, in one `await`-free synchronous
/// step (the trait method itself is `async` for compatibility with
/// the wider [`Signer`] surface, but the body never yields).
pub struct MnemonicResolverCoreSigner {
    /// Resolver handle stored as `usize` so the wrapping struct can
    /// derive `Send + Sync` without an extra `unsafe impl` on the
    /// field. The pointer is reconstituted inside the methods that
    /// need it — see the unsafety contract on the module-level docs
    /// for the lifetime guarantee.
    resolver_addr: usize,
    /// Wallet id passed through to every resolver invocation so the
    /// Swift side can look up the right mnemonic. Same shape as
    /// `dash_sdk_sign_with_mnemonic_resolver_and_path`'s
    /// `wallet_id_bytes` parameter.
    wallet_id: [u8; 32],
    /// Network the derived `ExtendedPrivKey` is bound to. Captured
    /// at construction so the trait methods don't need an extra
    /// argument plumbed through key-wallet's `Signer` contract
    /// (which is intentionally network-agnostic).
    network: Network,
}

// SAFETY: `usize` and `[u8; 32]` are both `Send + Sync`; `Network`
// is a plain `Copy` enum. The raw resolver pointer hidden inside
// `resolver_addr` is documented to be thread-safe by contract — the
// Swift-side vtable is either `@MainActor`-isolated or backed by a
// serial dispatch queue, mirroring how `MnemonicResolverHandle`
// itself carries `unsafe impl Send + Sync` in
// `crate::mnemonic_resolver`.
//
// We deliberately do *not* stash the pointer as `*mut
// MnemonicResolverHandle` directly: that field would be `!Send +
// !Sync`, forcing an `unsafe impl Send + Sync` on this struct that
// covers more than the resolver's actual thread-safety contract
// (raw pointer fields are unsoundly broad). The `usize` indirection
// is sound because it sheds the "pointer to T" type, leaving only
// a numeric handle whose dereference responsibility lives in each
// method body (next to its safety justification).
//
// No explicit `unsafe impl Send` / `unsafe impl Sync` needed — both
// auto-traits derive from the `usize + [u8; 32] + Network` field
// shape.

impl MnemonicResolverCoreSigner {
    /// Construct a new `MnemonicResolverCoreSigner`.
    ///
    /// # Safety
    /// - `handle` must come from
    ///   [`crate::mnemonic_resolver::dash_sdk_mnemonic_resolver_create`]
    ///   and must stay alive for the entire lifetime of every
    ///   `MnemonicResolverCoreSigner`
    ///   value that wraps it. The signer never destroys the handle —
    ///   ownership belongs to the FFI caller that built it.
    /// - `handle` may be null; methods will fail with a resolver
    ///   error if the resolver is dereferenced. Production callers
    ///   should pass non-null.
    pub unsafe fn new(
        handle: *mut MnemonicResolverHandle,
        wallet_id: [u8; 32],
        network: Network,
    ) -> Self {
        Self {
            resolver_addr: handle as usize,
            wallet_id,
            network,
        }
    }

    /// Resolve the mnemonic from the Swift-side callback and derive the
    /// BIP-32 [`ExtendedPrivKey`] at `path`, returning `(master, derived)`.
    ///
    /// Single source of truth for the resolve → parse → seed → master →
    /// `derive_priv` chain shared by [`Self::derive_priv`] (which reads the
    /// derived scalar) and the [`Signer::extended_public_key`] method (which
    /// computes the public xpub). All intermediate byte buffers (mnemonic,
    /// seed) are dropped — and zeroed via `Zeroizing` — before this method
    /// returns.
    ///
    /// # Zeroization contract
    ///
    /// The returned `master` / `derived` are each wrapped in [`WipingXprv`],
    /// whose `Drop` scrubs the inner `secp256k1::SecretKey` scalar (which
    /// [`ExtendedPrivKey`] does not wipe on its own). So both scalars are wiped
    /// on **every** exit path of the caller — normal return, `?` error
    /// propagation, and panic unwind — with no hand-placed `non_secure_erase`
    /// required. `master` is wrapped the instant it exists, so it is scrubbed
    /// even if the path derivation below fails.
    fn resolve_derived_xprv(
        &self,
        path: &DerivationPath,
    ) -> Result<(WipingXprv, WipingXprv), MnemonicResolverSignerError> {
        if self.resolver_addr == 0 {
            return Err(MnemonicResolverSignerError::NullHandle);
        }

        // ---- Resolve mnemonic into a Zeroizing buffer -----------------------
        let mut mnemonic_buf: Zeroizing<[u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]> =
            Zeroizing::new([0u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]);
        let mut mnemonic_len: usize = 0;

        // SAFETY: We re-cast from `usize` to `*mut MnemonicResolverHandle`
        // here. The caller of `new()` guaranteed the original pointer
        // outlives this signer (see the unsafety contract on
        // `Self::new`). `MnemonicResolverHandle`'s vtable + ctx are
        // thread-stable per the same module's `unsafe impl Send +
        // Sync` justification.
        let resolver = unsafe { &*(self.resolver_addr as *const MnemonicResolverHandle) };
        let vtable = unsafe { &*resolver.vtable };
        let rc = unsafe {
            (vtable.resolve)(
                resolver.ctx as *const c_void,
                self.wallet_id.as_ptr(),
                mnemonic_buf.as_mut_ptr() as *mut c_char,
                MNEMONIC_RESOLVER_BUFFER_CAPACITY,
                &mut mnemonic_len,
            )
        };
        match rc {
            x if x == mnemonic_resolver_result::SUCCESS => {}
            x if x == mnemonic_resolver_result::NOT_FOUND => {
                return Err(MnemonicResolverSignerError::NotFound);
            }
            x if x == mnemonic_resolver_result::BUFFER_TOO_SMALL => {
                return Err(MnemonicResolverSignerError::BufferTooSmall);
            }
            other => {
                return Err(MnemonicResolverSignerError::ResolverFailed(other));
            }
        }
        if mnemonic_len == 0 || mnemonic_len > MNEMONIC_RESOLVER_BUFFER_CAPACITY {
            return Err(MnemonicResolverSignerError::InvalidMnemonicLength(
                mnemonic_len,
            ));
        }

        // Parse mnemonic. UTF-8 validation runs on the prefix only —
        // we never construct an owned `String` (the resulting buffer
        // is dropped via Zeroizing).
        let mnemonic_str = std::str::from_utf8(&mnemonic_buf[..mnemonic_len])
            .map_err(|_| MnemonicResolverSignerError::InvalidUtf8)?;
        let mnemonic = parse_mnemonic_any_language(mnemonic_str)
            .map_err(|e| MnemonicResolverSignerError::InvalidMnemonic(e.to_string()))?;

        // ---- Derive seed and BIP-32 key at `path` ---------------------------
        let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(""));
        drop(mnemonic);

        let secp = Secp256k1::new();
        // Wrap `master` in its wiping guard the instant it exists, so its scalar
        // is scrubbed even if the path derivation below returns `Err`.
        let master = WipingXprv(
            ExtendedPrivKey::new_master(self.network, seed.as_ref()).map_err(|e| {
                MnemonicResolverSignerError::DerivationFailed(format!("master: {e}"))
            })?,
        );
        let derived = WipingXprv(
            master
                .key()
                .derive_priv(&secp, path)
                .map_err(|e| MnemonicResolverSignerError::DerivationFailed(format!("path: {e}")))?,
        );

        Ok((master, derived))
    }

    /// Resolve the mnemonic from the Swift-side callback, then
    /// derive the secp256k1 private key at `path`. Returns the raw
    /// 32-byte scalar in a `Zeroizing` wrapper so the caller's last
    /// drop point zeros it.
    ///
    /// All other intermediate buffers (mnemonic, seed) are dropped
    /// (and zeroed) before this method returns — only the final
    /// derived scalar leaks out, and even that is `Zeroizing`-wrapped.
    fn derive_priv(
        &self,
        path: &DerivationPath,
    ) -> Result<Zeroizing<[u8; 32]>, MnemonicResolverSignerError> {
        let (_master, derived) = self.resolve_derived_xprv(path)?;

        // `secret_bytes()` returns a plain `[u8; 32]`; wrap in `Zeroizing` so
        // the caller (and any panic-unwind path) wipes it on drop. The
        // `WipingXprv` guards scrub `master`/`derived`'s scalars when they drop
        // at the end of this scope, on every exit path.
        let bytes = Zeroizing::new(derived.key().private_key.secret_bytes());

        Ok(bytes)
    }
}

/// RAII guard that scrubs an [`ExtendedPrivKey`]'s secret scalar on drop.
///
/// `key_wallet::bip32::ExtendedPrivKey` has no `Drop` / `Zeroize` impl, so its
/// inner `secp256k1::SecretKey` would otherwise survive in memory on *every*
/// exit path — normal return, early `?` error propagation, and panic unwind.
/// Moving the wipe into `Drop` makes it run on all of them, instead of only
/// where a `non_secure_erase()` was hand-placed. (`non_secure_erase` is
/// secp256k1's best-effort scrub — the strongest tool until `ExtendedPrivKey`
/// gains a `ZeroizeOnDrop` upstream.) The sibling FFI at
/// `rs-platform-wallet-ffi/src/sign_with_mnemonic_resolver.rs` has the same
/// un-wiped-on-error gap and wants the same guard.
struct WipingXprv(ExtendedPrivKey);

impl WipingXprv {
    #[inline]
    fn key(&self) -> &ExtendedPrivKey {
        &self.0
    }
}

impl Drop for WipingXprv {
    fn drop(&mut self) {
        self.0.private_key.non_secure_erase();
    }
}

#[async_trait]
impl Signer for MnemonicResolverCoreSigner {
    type Error = MnemonicResolverSignerError;

    fn supported_methods(&self) -> &[SignerMethod] {
        // Only digest signing is supported — the resolver flow has
        // no facility for parsing or rendering full transactions.
        static METHODS: &[SignerMethod] = &[SignerMethod::Digest];
        METHODS
    }

    async fn sign_ecdsa(
        &self,
        path: &DerivationPath,
        sighash: [u8; 32],
    ) -> Result<(secp256k1::ecdsa::Signature, secp256k1::PublicKey), Self::Error> {
        let secret_bytes = self.derive_priv(path)?;
        let secp = Secp256k1::new();
        // `SecretKey::from_slice` validates the 32-byte scalar is a
        // legitimate field element.
        let mut secret = secp256k1::SecretKey::from_slice(secret_bytes.as_ref())
            .map_err(|e| MnemonicResolverSignerError::InvalidScalar(e.to_string()))?;
        let msg = secp256k1::Message::from_digest(sighash);
        let signature = secp.sign_ecdsa(&msg, &secret);
        let pubkey = secp256k1::PublicKey::from_secret_key(&secp, &secret);
        // Wipe the SecretKey-owned scalar before it drops. `Zeroizing<[u8;32]>`
        // covers `secret_bytes`; `SecretKey::from_slice` allocated a separate
        // 32-byte copy that needs its own wipe.
        secret.non_secure_erase();
        Ok((signature, pubkey))
    }

    async fn public_key(&self, path: &DerivationPath) -> Result<secp256k1::PublicKey, Self::Error> {
        let secret_bytes = self.derive_priv(path)?;
        let secp = Secp256k1::new();
        let mut secret = secp256k1::SecretKey::from_slice(secret_bytes.as_ref())
            .map_err(|e| MnemonicResolverSignerError::InvalidScalar(e.to_string()))?;
        let pubkey = secp256k1::PublicKey::from_secret_key(&secp, &secret);
        // Wipe the SecretKey-owned scalar before it drops. `Zeroizing<[u8;32]>`
        // covers `secret_bytes`; `SecretKey::from_slice` allocated a separate
        // 32-byte copy that needs its own wipe.
        secret.non_secure_erase();
        Ok(pubkey)
    }

    async fn extended_public_key(
        &self,
        path: &DerivationPath,
    ) -> Result<ExtendedPubKey, Self::Error> {
        let (_master, derived) = self.resolve_derived_xprv(path)?;
        let secp = Secp256k1::new();
        // The extended public key (point + chain code) carries no secret; it is
        // safe to return. The `WipingXprv` guards scrub the private halves when
        // they drop at the end of this scope, on every exit path.
        let xpub = ExtendedPubKey::from_priv(&secp, derived.key());

        Ok(xpub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mnemonic_resolver::{
        dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy,
    };
    use std::str::FromStr;

    /// English BIP-39 test vector (all-zero entropy).
    const ENGLISH_PHRASE: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    unsafe extern "C" fn english_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> i32 {
        let phrase = ENGLISH_PHRASE.as_bytes();
        if phrase.len() + 1 > out_capacity {
            return mnemonic_resolver_result::BUFFER_TOO_SMALL;
        }
        std::ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out_buf, phrase.len());
        *out_buf.add(phrase.len()) = 0;
        *out_len = phrase.len();
        mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn missing_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        _out_buf: *mut c_char,
        _out_capacity: usize,
        _out_len: *mut usize,
    ) -> i32 {
        mnemonic_resolver_result::NOT_FOUND
    }

    unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}

    fn make_resolver(
        cb: crate::mnemonic_resolver::MnemonicResolveCallback,
    ) -> *mut MnemonicResolverHandle {
        unsafe { dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), cb, noop_destroy) }
    }

    fn test_path() -> DerivationPath {
        DerivationPath::from_str("m/9'/1'/5'/0'/0'/0'/0'").expect("valid path")
    }

    #[tokio::test]
    async fn sign_ecdsa_round_trips_and_verifies() {
        let resolver = make_resolver(english_resolve);
        let signer =
            unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

        let sighash = [0x42u8; 32];
        let (sig, pk) = signer
            .sign_ecdsa(&test_path(), sighash)
            .await
            .expect("signing succeeds");

        let secp = Secp256k1::new();
        let msg = secp256k1::Message::from_digest(sighash);
        secp.verify_ecdsa(&msg, &sig, &pk)
            .expect("signature must verify against returned pubkey");

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[tokio::test]
    async fn public_key_matches_sign_ecdsa_pubkey() {
        let resolver = make_resolver(english_resolve);
        let signer =
            unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

        let path = test_path();
        let pk_only = signer.public_key(&path).await.expect("public_key succeeds");
        let (_, pk_via_sign) = signer
            .sign_ecdsa(&path, [0u8; 32])
            .await
            .expect("signing succeeds");

        assert_eq!(
            pk_only, pk_via_sign,
            "public_key() and sign_ecdsa() must return the same pubkey for the same path"
        );

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[tokio::test]
    async fn extended_public_key_leaf_matches_public_key() {
        let resolver = make_resolver(english_resolve);
        let signer =
            unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

        let path = test_path();
        let xpub = signer
            .extended_public_key(&path)
            .await
            .expect("extended_public_key succeeds");
        let pk_only = signer.public_key(&path).await.expect("public_key succeeds");

        // The xpub's leaf point must be the same key `public_key()` derives
        // at the same path — they take different routes (xpub vs raw scalar)
        // to the same secp256k1 point.
        assert_eq!(
            xpub.public_key, pk_only,
            "extended_public_key().public_key must equal public_key() at the same path"
        );

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    /// Interop guard: the signer-based DashPay xpub route must be
    /// byte-identical to the resident-seed
    /// `Wallet::derive_extended_public_key` it replaces.
    ///
    /// The signer derives the contact-relationship extended public key at
    /// the DIP-15 receiving path `m/9'/coin'/15'/0'/<sender>/<recipient>`
    /// from the Keychain mnemonic; a `Wallet` built from the SAME mnemonic
    /// derives it the old way. If they ever diverge, every contact xpub the
    /// signer path produces would be unrecognizable to the resident-seed
    /// path (and to the reference clients), so this pins them equal.
    #[tokio::test]
    async fn extended_public_key_matches_wallet_derivation_for_dashpay_path() {
        use key_wallet::account::AccountType;
        use key_wallet::mnemonic::{Language, Mnemonic};
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::wallet::Wallet;

        // Two arbitrary 32-byte identity ids for the friendship path.
        let sender_id = [0x11u8; 32];
        let recipient_id = [0x22u8; 32];

        let path = AccountType::DashpayReceivingFunds {
            index: 0,
            user_identity_id: sender_id,
            friend_identity_id: recipient_id,
        }
        .derivation_path(Network::Testnet)
        .expect("DashPay receiving path");

        // Old route: resident-seed wallet from the same mnemonic.
        let mnemonic =
            Mnemonic::from_phrase(ENGLISH_PHRASE, Language::English).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet =
            Wallet::from_seed_bytes(seed, Network::Testnet, WalletAccountCreationOptions::None)
                .expect("seeded wallet");
        let expected = wallet
            .derive_extended_public_key(&path)
            .expect("wallet derives DashPay xpub");

        // New route: signer fed the same mnemonic via the resolver.
        let resolver = make_resolver(english_resolve);
        let signer =
            unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };
        let via_signer = signer
            .extended_public_key(&path)
            .await
            .expect("signer derives DashPay xpub");

        assert_eq!(
            via_signer, expected,
            "signer-based DashPay xpub must equal Wallet::derive_extended_public_key \
             for the same mnemonic and path"
        );

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[tokio::test]
    async fn missing_resolver_surfaces_not_found_error() {
        let resolver = make_resolver(missing_resolve);
        let signer =
            unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

        let err = signer
            .sign_ecdsa(&test_path(), [0u8; 32])
            .await
            .expect_err("must fail when resolver returns NOT_FOUND");
        // Pin the typed variant — String-contains is no longer the
        // contract. Surfaces FFI/test breakage at the structured-error
        // boundary rather than at a free-form Display payload.
        assert!(
            matches!(err, MnemonicResolverSignerError::NotFound),
            "error should be NotFound, got: {err:?}"
        );

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[tokio::test]
    async fn null_handle_surfaces_clean_error() {
        let signer = unsafe {
            MnemonicResolverCoreSigner::new(std::ptr::null_mut(), [0u8; 32], Network::Testnet)
        };
        let err = signer
            .sign_ecdsa(&test_path(), [0u8; 32])
            .await
            .expect_err("must fail with null handle");
        assert!(
            matches!(err, MnemonicResolverSignerError::NullHandle),
            "error should be NullHandle, got: {err:?}"
        );
    }

    #[test]
    fn advertises_digest_only() {
        let signer = unsafe {
            MnemonicResolverCoreSigner::new(std::ptr::null_mut(), [0u8; 32], Network::Testnet)
        };
        let methods = signer.supported_methods();
        assert_eq!(methods.len(), 1);
        assert!(matches!(methods[0], SignerMethod::Digest));
    }
}
