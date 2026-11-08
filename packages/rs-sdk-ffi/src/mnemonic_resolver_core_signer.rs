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
//! method returns. Three mechanisms cover the different ownership
//! shapes:
//!
//! - **[`ExtendedPrivKey`] self-wipes on `Drop`.** The master and
//!   derived extended keys zero their secret material when they leave
//!   scope, on every exit path — success, `?`-early-return, and
//!   panic-unwind. The type is not `Copy`, so each move is a real move
//!   that leaves no stray bitwise duplicate behind.
//! - **`Zeroizing` wrappers** scrub the plain byte buffers that carry
//!   no `Drop` of their own: the resolver mnemonic buffer, the BIP-39
//!   seed, and the final derived 32-byte scalar.
//! - **Typed ECDSA secret keys** scrub signing copies on drop. The
//!   `WipingSecretKey` guard scrubs the raw secp256k1 copy used for ECDH.
//!
//! Combined, no private key bytes survive past the trait-method
//! boundary.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::os::raw::c_char;

use async_trait::async_trait;
use dash_sdk::dpp::data_contract::DataContract;
use dash_sdk::dpp::identity::{Identity, KeyID};
use dash_sdk::dpp::platform_value::BinaryData;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::secp256k1;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::platform::dashpay::{
    private_contact_request_outgoing_viewing_key, private_contact_requests_from_scan,
    seal_private_contact_request, PrivateContactRequest, PrivateContactRequestScan,
    PrivateContactRequestSender,
};
use dash_sdk::platform::private_document_entries::PrivateDocumentEntries;
use dash_sdk::platform::private_document_scan::{PrivateDocumentScan, PrivateDocumentScanKeys};
use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey, ExtendedPubKey};
use key_wallet::dashcore::ecdsa::{EcdsaError, EcdsaPublicKey, EcdsaSecretKey, EcdsaSignature};
use key_wallet::dip9::{
    DASHPAY_CONTACT_INFO_ENC_TO_USER_ID_CHILD, DASHPAY_CONTACT_INFO_PRIVATE_DATA_CHILD,
    FEATURE_PURPOSE, FEATURE_PURPOSE_DASHPAY_AUTO_ACCEPT, FEATURE_PURPOSE_IDENTITIES,
    FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_INVITATIONS,
};
use key_wallet::signer::{ExtendedPubKeySigner, Signer, SignerMethod};
use key_wallet::Network;
use platform_encryption::PrivateDocumentTarget;
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
    ///
    /// Renders with
    /// [`DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX`](crate::signer::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX)
    /// at position 0: this is the missing-key completion of the
    /// `Signer` surface, whose error is only `Display`, so the reserved
    /// machine marker at the START of the rendering is the one typed
    /// signal a caller may recognize (position-0 check, never a
    /// substring sniff — dashpay/platform#4183 review).
    /// `platform-wallet`'s message signing promotes it to its typed
    /// key-unavailable error, which the FFI maps to code 31
    /// (`ErrorSigningKeyUnavailable`).
    #[error(
        "{}mnemonic not found in keychain for the given wallet_id",
        crate::signer::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX
    )]
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

    /// Decoding the generated recoverable signature failed.
    #[error("ECDSA signature conversion failed: {0}")]
    SignatureConversion(#[source] EcdsaError),

    /// The SDK refused to read or seal a private contact request with the
    /// derived keys: a malformed request, a key the identity does not hold,
    /// or a contract without the private type.
    #[error("private contact request: {0}")]
    PrivateContactRequest(dash_sdk::Error),
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

    /// Resolve the mnemonic from the Swift-side callback, build the BIP-32
    /// master `ExtendedPrivKey`, and hand it *by reference* to `use_master`,
    /// returning whatever `use_master` produces. An operation that needs keys
    /// at several paths derives them all from this one resolve.
    ///
    /// This is the single entry-point for all private-key material in this
    /// signer. It handles the full stack: resolver FFI call → result-code
    /// mapping → UTF-8 + word-list validation → BIP-39 seed → master
    /// `ExtendedPrivKey`.
    ///
    /// # Zeroization contract
    ///
    /// The `master` extended key wipes its secret material when it leaves
    /// this scope, as does every child `use_master` derives from it:
    /// [`ExtendedPrivKey`] zeroizes on `Drop` and is not `Copy`, so each move
    /// is a real move that leaves no bitwise duplicate behind. The master key
    /// never crosses the call boundary (`use_master` only borrows it), so it
    /// cannot outlive the operation. `use_master` returns public material, a
    /// product of the keys, or a `Zeroizing` scalar copy the caller wipes on
    /// its own drop. The mnemonic and seed buffers are plain arrays and ride
    /// [`Zeroizing`] wrappers for the same guarantee.
    ///
    /// # Errors
    ///
    /// Propagates [`MnemonicResolverSignerError`] for every failure mode:
    /// null handle, resolver FFI errors, encoding/parse failures, BIP-32
    /// derivation errors, and whatever `use_master` returns.
    fn resolve_master<T>(
        &self,
        use_master: impl FnOnce(&ExtendedPrivKey) -> Result<T, MnemonicResolverSignerError>,
    ) -> Result<T, MnemonicResolverSignerError> {
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
        let seed: Zeroizing<[u8; 64]> = mnemonic.to_seed("");
        drop(mnemonic);

        let master = ExtendedPrivKey::new_master(self.network, seed.as_ref())
            .map_err(|e| MnemonicResolverSignerError::DerivationFailed(format!("master: {e}")))?;

        use_master(&master)
    }

    /// Resolve the mnemonic, derive the BIP-32 extended private key at
    /// `path`, and hand it *by reference* to `extract`, returning whatever
    /// `extract` produces. The derived key self-wipes on `Drop` when this
    /// returns; see [`Self::resolve_master`] for the full contract.
    fn resolve_and_derive<T>(
        &self,
        path: &DerivationPath,
        extract: impl FnOnce(&ExtendedPrivKey) -> T,
    ) -> Result<T, MnemonicResolverSignerError> {
        self.resolve_master(|master| {
            let derived = master
                .derive_priv(path)
                .map_err(|e| MnemonicResolverSignerError::DerivationFailed(format!("path: {e}")))?;
            Ok(extract(&derived))
        })
    }

    /// Resolve the mnemonic and derive the raw 32-byte scalar at `path`.
    ///
    /// Returns the scalar in a `Zeroizing` wrapper so the caller's last
    /// drop point wipes it. The intermediate `ExtendedPrivKey` values,
    /// mnemonic, and seed are wiped inside [`Self::resolve_and_derive`]
    /// before this returns.
    fn derive_priv(
        &self,
        path: &DerivationPath,
    ) -> Result<Zeroizing<[u8; 32]>, MnemonicResolverSignerError> {
        // `to_bytes()` copies the scalar out of the borrowed key; the
        // `ExtendedPrivKey` itself never leaves `resolve_and_derive`.
        self.resolve_and_derive(path, |derived| derived.private_key.to_bytes())
    }

    /// Export the raw auto-accept private scalar at `path` (DIP-15 QR
    /// auto-accept) — the **one deliberate raw-key export** from this signer
    /// (every other method returns only a derived product, never the scalar).
    /// The auto-accept key is a shareable, expiry-bounded bearer credential the
    /// owner embeds in a QR (`dapk`), so it must leave the signer.
    ///
    /// Scoped by defense-in-depth: `path` MUST be an auto-accept path
    /// (`m/9'/coin_type'/16'/expiry'`, 4 components with `9'` purpose + `16'`
    /// feature) — otherwise this errors, so it cannot be repurposed to
    /// exfiltrate a signing or identity key. Returns the 32-byte scalar
    /// `Zeroizing`-wrapped (the QR encoder copies it; the wrapper wipes the
    /// temporary on drop).
    pub fn export_auto_accept_private_key(
        &self,
        path: &DerivationPath,
    ) -> Result<Zeroizing<[u8; 32]>, MnemonicResolverSignerError> {
        let purpose9 = ChildNumber::from_hardened_idx(FEATURE_PURPOSE)
            .map_err(|e| MnemonicResolverSignerError::DerivationFailed(e.to_string()))?;
        let feature16 = ChildNumber::from_hardened_idx(FEATURE_PURPOSE_DASHPAY_AUTO_ACCEPT)
            .map_err(|e| MnemonicResolverSignerError::DerivationFailed(e.to_string()))?;
        let comps: &[ChildNumber] = path.as_ref();
        if comps.len() != 4 || comps[0] != purpose9 || comps[2] != feature16 {
            return Err(MnemonicResolverSignerError::DerivationFailed(
                "export_auto_accept_private_key: path is not an auto-accept path".to_string(),
            ));
        }
        self.derive_priv(path)
    }

    /// Export the raw DIP-13 invitation-funding private scalar at `path`
    /// (`m/9'/coin_type'/5'/3'/funding_index'`) — the second deliberate raw-key
    /// export from this signer. An invitation hands the one-time voucher key to
    /// the invitee so they can register their own identity from the funded asset
    /// lock, so this key must leave the signer (a scoped, documented bearer
    /// credential like the auto-accept `dapk`).
    ///
    /// Gated to the **exact** invitation sub-feature: `path` MUST have 5
    /// components with `9'` purpose, `5'` identity feature, and `3'` invitation
    /// sub-feature. This is deliberately stricter than checking the feature
    /// alone — feature `5'` is shared with identity authentication (`5'/0'`),
    /// registration funding (`5'/1'`), and top-up (`5'/2'`), so a looser gate
    /// could be repurposed to exfiltrate the user's own identity keys. Returns
    /// the 32-byte scalar `Zeroizing`-wrapped.
    pub fn export_invitation_private_key(
        &self,
        path: &DerivationPath,
    ) -> Result<Zeroizing<[u8; 32]>, MnemonicResolverSignerError> {
        let purpose9 = ChildNumber::from_hardened_idx(FEATURE_PURPOSE)
            .map_err(|e| MnemonicResolverSignerError::DerivationFailed(e.to_string()))?;
        let feature5 = ChildNumber::from_hardened_idx(FEATURE_PURPOSE_IDENTITIES)
            .map_err(|e| MnemonicResolverSignerError::DerivationFailed(e.to_string()))?;
        let subfeature3 =
            ChildNumber::from_hardened_idx(FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_INVITATIONS)
                .map_err(|e| MnemonicResolverSignerError::DerivationFailed(e.to_string()))?;
        let comps: &[ChildNumber] = path.as_ref();
        // Bind the fixed purpose / feature / sub-feature only. `coin_type`
        // (comps[1]) and `funding_index` (comps[4]) are deliberately left
        // unconstrained: the whole `9'/*/5'/3'/*` subtree is invitation-vouchers
        // only (the user's own keys live at sub-features `5'/0'`, `5'/1'`, `5'/2'`,
        // all excluded by `comps[3] == 3'`), so this gate cannot exfiltrate a
        // user key regardless of their hardening. Constraining them additionally
        // rejects real voucher paths whose coin_type is non-hardened.
        if comps.len() != 5
            || comps[0] != purpose9
            || comps[2] != feature5
            || comps[3] != subfeature3
        {
            return Err(MnemonicResolverSignerError::DerivationFailed(
                "export_invitation_private_key: path is not an invitation-funding path".to_string(),
            ));
        }
        self.derive_priv(path)
    }

    /// Compute the DIP-15 ECDH shared secret between our identity-encryption
    /// key (derived at `path`) and the contact's `peer_pubkey`, entirely
    /// in-process. The derived private scalar never leaves this function —
    /// only the ECDH *product* is returned (safe to use as the symmetric key
    /// for the caller's AES step; it is not the raw scalar).
    ///
    /// Reuses [`platform_encryption::derive_shared_key_ecdh`] — the single
    /// ECDH source (`SHA256((y&1|2) ‖ x)`) — so the result is byte-identical
    /// to the resident-seed path it replaces (pinned by a parity test). The
    /// borrowed `ExtendedPrivKey` never leaves [`Self::resolve_and_derive`]
    /// and self-wipes on `Drop`.
    ///
    /// Sync (the derivation is CPU-bound + the resolver call is synchronous);
    /// the [`EcdhProvider::ClientSide`] closure that consumes it wraps it in a
    /// future at the FFI seam.
    pub fn ecdh_shared_secret(
        &self,
        path: &DerivationPath,
        peer_pubkey: &secp256k1::PublicKey,
    ) -> Result<Zeroizing<[u8; 32]>, MnemonicResolverSignerError> {
        // Read the scalar by reference inside the closure; the borrowed
        // `ExtendedPrivKey` self-wipes on `Drop` when `resolve_and_derive`
        // returns. Only the ECDH product crosses the boundary.
        let secret = WipingSecretKey(
            secp256k1::SecretKey::from_secret_bytes(*self.derive_priv(path)?)
                .map_err(|e| MnemonicResolverSignerError::InvalidScalar(e.to_string()))?,
        );
        let shared = platform_encryption::derive_shared_key_ecdh(&secret.0, peer_pubkey);
        Ok(Zeroizing::new(shared))
    }

    /// DIP-15 `accountReference` for a contact-request send, computed entirely
    /// in-process. Derive the sender's ECDH private scalar at `path` and feed it
    /// (as the HMAC key) to [`platform_encryption::calculate_account_reference`]
    /// over the 69-byte compact xpub. This is the same scalar
    /// [`Self::ecdh_shared_secret`] uses; it never leaves the signer (the derived
    /// scalar is `Zeroizing`-scrubbed and the intermediate `ExtendedPrivKey`
    /// self-wipes on `Drop`), so the masked reference is produced without the
    /// resident seed.
    pub fn account_reference(
        &self,
        path: &DerivationPath,
        compact_xpub: &[u8],
        account_index: u32,
        version: u32,
    ) -> Result<u32, MnemonicResolverSignerError> {
        let secret = self.derive_priv(path)?;
        Ok(platform_encryption::calculate_account_reference(
            &secret,
            compact_xpub,
            account_index,
            version,
        ))
    }

    /// Inverse of [`Self::account_reference`]: recover `(version, account_index)`
    /// from a masked reference using the same in-process scalar. Used on re-send
    /// to read the previous rotation version without the resident seed.
    pub fn unmask_account_reference(
        &self,
        path: &DerivationPath,
        compact_xpub: &[u8],
        account_reference: u32,
    ) -> Result<(u32, u32), MnemonicResolverSignerError> {
        let secret = self.derive_priv(path)?;
        Ok(platform_encryption::unmask_account_reference(
            account_reference,
            &secret,
            compact_xpub,
        ))
    }

    /// Derive the 32-byte AES key for one DIP-15 contactInfo feature
    /// (`encToUserId` = 65536, `privateData` = 65537) at
    /// `root_path / feature' / derivation_index'`. The intermediate
    /// `ExtendedPrivKey` self-wipes on `Drop`; the returned key bytes are
    /// `Zeroizing`-wrapped.
    fn derive_contact_info_aes_key(
        &self,
        root_path: &DerivationPath,
        feature: u32,
        derivation_index: u32,
    ) -> Result<Zeroizing<[u8; 32]>, MnemonicResolverSignerError> {
        let path = root_path.clone().extend([
            ChildNumber::from_hardened_idx(feature).map_err(|e| {
                MnemonicResolverSignerError::DerivationFailed(format!("contactInfo feature: {e}"))
            })?,
            ChildNumber::from_hardened_idx(derivation_index).map_err(|e| {
                MnemonicResolverSignerError::DerivationFailed(format!("contactInfo index: {e}"))
            })?,
        ]);
        self.derive_priv(&path)
    }

    /// DIP-15 contactInfo **seal**: encrypt `contact_id` (`encToUserId`,
    /// AES-256-ECB) and `private_data_plaintext` (`privateData`, AES-256-CBC
    /// with `private_data_iv`) under the two hardened-child keys at `root_path`,
    /// entirely in-process. Reuses `platform_encryption` (the single AES
    /// source); the DIP-15 wire codec (length prefixes etc.) stays in the
    /// caller — this handles only the key derivation + AES.
    pub fn contact_info_seal(
        &self,
        root_path: &DerivationPath,
        derivation_index: u32,
        contact_id: &[u8; 32],
        private_data_plaintext: &[u8],
        private_data_iv: &[u8; 16],
    ) -> Result<ContactInfoSealed, MnemonicResolverSignerError> {
        let enc_key = self.derive_contact_info_aes_key(
            root_path,
            DASHPAY_CONTACT_INFO_ENC_TO_USER_ID_CHILD,
            derivation_index,
        )?;
        let priv_key = self.derive_contact_info_aes_key(
            root_path,
            DASHPAY_CONTACT_INFO_PRIVATE_DATA_CHILD,
            derivation_index,
        )?;
        Ok(ContactInfoSealed {
            enc_to_user_id: platform_encryption::encrypt_enc_to_user_id(&enc_key, contact_id),
            private_data: platform_encryption::encrypt_private_data(
                &priv_key,
                private_data_iv,
                private_data_plaintext,
            ),
        })
    }

    /// DIP-15 contactInfo **open**: inverse of [`Self::contact_info_seal`] —
    /// recover the contact id + private-data plaintext.
    pub fn contact_info_open(
        &self,
        root_path: &DerivationPath,
        derivation_index: u32,
        enc_to_user_id: &[u8; 32],
        private_data_blob: &[u8],
    ) -> Result<ContactInfoOpened, MnemonicResolverSignerError> {
        let enc_key = self.derive_contact_info_aes_key(
            root_path,
            DASHPAY_CONTACT_INFO_ENC_TO_USER_ID_CHILD,
            derivation_index,
        )?;
        let priv_key = self.derive_contact_info_aes_key(
            root_path,
            DASHPAY_CONTACT_INFO_PRIVATE_DATA_CHILD,
            derivation_index,
        )?;
        let private_data = platform_encryption::decrypt_private_data(&priv_key, private_data_blob)
            .map_err(|e| {
                MnemonicResolverSignerError::DerivationFailed(format!("contactInfo decrypt: {e}"))
            })?;
        Ok(ContactInfoOpened {
            contact_id: platform_encryption::decrypt_enc_to_user_id(&enc_key, enc_to_user_id),
            private_data,
        })
    }

    /// Sort `page`, read from `start_position` of the store of the private
    /// document type `document_type_name` of `data_contract_id`, with the
    /// wallet's keys ([`PrivateDocumentScanKeys::scan_page`]). `recipients`
    /// are the identities entries may be sealed to, each with the paths of
    /// its keys by key id; `senders` the identities that may have sealed
    /// entries, each with the path of an encryption key whose outgoing
    /// viewing key recovers what it sent. Every key comes from one resolve
    /// and is dropped here; only the sorted entries return.
    pub fn scan_private_document_page(
        &self,
        recipients: &[(Identifier, Vec<(KeyID, DerivationPath)>)],
        senders: &[(Identifier, DerivationPath)],
        data_contract_id: Identifier,
        document_type_name: &str,
        start_position: u64,
        page: &PrivateDocumentEntries,
    ) -> Result<PrivateDocumentScan, MnemonicResolverSignerError> {
        let target = PrivateDocumentTarget {
            contract_id: data_contract_id.to_buffer(),
            document_type_name,
        };
        self.resolve_master(|master| {
            let keys = private_document_scan_keys(master, recipients, senders)?;
            Ok(keys.scan_page(&target, start_position, page))
        })
    }

    /// Read the private contact requests of `scan`, a scan of DashPay's
    /// store made with the keys at `recipients`
    /// ([`private_contact_requests_from_scan`]): each received request's
    /// sender is checked with the recipient key that opened it, against the
    /// key `sender_identities` holds under the id it names. Only the
    /// recipient keys are derived, from one resolve, and dropped here.
    pub fn read_private_contact_requests(
        &self,
        recipients: &[(Identifier, Vec<(KeyID, DerivationPath)>)],
        dashpay_contract: &DataContract,
        scan: PrivateDocumentScan,
        sender_identities: &BTreeMap<Identifier, Option<Identity>>,
        platform_version: &PlatformVersion,
    ) -> Result<PrivateContactRequestScan, MnemonicResolverSignerError> {
        self.resolve_master(|master| {
            let keys = private_document_scan_keys(master, recipients, &[])?;
            private_contact_requests_from_scan(
                dashpay_contract,
                scan,
                &keys,
                sender_identities,
                platform_version,
            )
            .map_err(MnemonicResolverSignerError::PrivateContactRequest)
        })
    }

    /// Seal `request` from `sender`, whose encryption key
    /// `sender_encryption_key_id` is the one at `sender_encryption_key_path`,
    /// to the key `recipient_key_id` of `recipient`
    /// ([`seal_private_contact_request`]). `esk_seed` is 32 fresh random
    /// bytes, never reused. The encryption key, and the outgoing viewing key
    /// the SDK derives from it, stay here; only the sealed entry returns.
    #[allow(clippy::too_many_arguments)]
    pub fn seal_private_contact_request(
        &self,
        sender_encryption_key_path: &DerivationPath,
        dashpay_contract: &DataContract,
        sender: &Identity,
        sender_encryption_key_id: KeyID,
        recipient: &Identity,
        recipient_key_id: KeyID,
        request: &PrivateContactRequest,
        esk_seed: &[u8; 32],
    ) -> Result<BinaryData, MnemonicResolverSignerError> {
        let secret =
            self.resolve_master(|master| secret_key_at(master, sender_encryption_key_path))?;
        seal_private_contact_request(
            dashpay_contract,
            &PrivateContactRequestSender {
                identity: sender,
                encryption_key_id: sender_encryption_key_id,
                encryption_secret_key: &secret.0,
            },
            recipient,
            recipient_key_id,
            request,
            esk_seed,
        )
        .map_err(MnemonicResolverSignerError::PrivateContactRequest)
    }
}

/// The secp256k1 secret key at `path` below `master`, wiped on drop. The
/// child `ExtendedPrivKey` self-wipes when this returns.
fn secret_key_at(
    master: &ExtendedPrivKey,
    path: &DerivationPath,
) -> Result<WipingSecretKey, MnemonicResolverSignerError> {
    let derived = master
        .derive_priv(path)
        .map_err(|e| MnemonicResolverSignerError::DerivationFailed(format!("path: {e}")))?;
    let secret = secp256k1::SecretKey::from_secret_bytes(*derived.private_key.to_bytes())
        .map_err(|e| MnemonicResolverSignerError::InvalidScalar(e.to_string()))?;
    Ok(WipingSecretKey(secret))
}

/// The scan keys of `recipients` and `senders`, derived below `master`: each
/// recipient's secret keys, and each sender's outgoing viewing key
/// ([`private_contact_request_outgoing_viewing_key`]). The copies made here
/// are wiped before this returns.
fn private_document_scan_keys(
    master: &ExtendedPrivKey,
    recipients: &[(Identifier, Vec<(KeyID, DerivationPath)>)],
    senders: &[(Identifier, DerivationPath)],
) -> Result<PrivateDocumentScanKeys, MnemonicResolverSignerError> {
    let mut keys = PrivateDocumentScanKeys::new();
    for (identity_id, paths) in recipients {
        let mut secret_keys = WipingSecretKeys(Vec::with_capacity(paths.len()));
        for (key_id, path) in paths {
            secret_keys
                .0
                .push((*key_id, secret_key_at(master, path)?.0));
        }
        keys = keys.with_recipient(*identity_id, &secret_keys.0);
    }
    for (identity_id, path) in senders {
        let secret = secret_key_at(master, path)?;
        let outgoing_viewing_key =
            Zeroizing::new(private_contact_request_outgoing_viewing_key(&secret.0));
        keys = keys.with_sender(*identity_id, *outgoing_viewing_key);
    }
    Ok(keys)
}

/// Result of [`MnemonicResolverCoreSigner::contact_info_seal`].
pub struct ContactInfoSealed {
    /// `encToUserId` ciphertext (AES-256-ECB of the 32-byte contact id).
    pub enc_to_user_id: [u8; 32],
    /// `privateData` ciphertext (`iv ‖ AES-256-CBC`).
    pub private_data: Vec<u8>,
}

/// Result of [`MnemonicResolverCoreSigner::contact_info_open`].
pub struct ContactInfoOpened {
    /// The recovered 32-byte contact id.
    pub contact_id: [u8; 32],
    /// The recovered private-data plaintext.
    pub private_data: Vec<u8>,
}

/// Scrubs the raw secp256k1 scalar copy used by ECDH on every exit path.
struct WipingSecretKey(secp256k1::SecretKey);

impl Drop for WipingSecretKey {
    fn drop(&mut self) {
        self.0.non_secure_erase();
    }
}

/// Scrubs the secret keys of one scan recipient on every exit path.
struct WipingSecretKeys(Vec<(KeyID, secp256k1::SecretKey)>);

impl Drop for WipingSecretKeys {
    fn drop(&mut self) {
        for (_, secret_key) in &mut self.0 {
            secret_key.non_secure_erase();
        }
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
    ) -> Result<(EcdsaSignature, EcdsaPublicKey), Self::Error> {
        let secret_bytes = self.derive_priv(path)?;
        let secret = EcdsaSecretKey::from_bytes(&secret_bytes)
            .map_err(|e| MnemonicResolverSignerError::InvalidScalar(e.to_string()))?;
        // Recoverable signing preserves the non-grinding deterministic signature.
        let signature = secret
            .sign_recoverable(&sighash)
            .signature()
            .map_err(MnemonicResolverSignerError::SignatureConversion)?;
        Ok((signature, secret.public_key()))
    }

    async fn public_key(&self, path: &DerivationPath) -> Result<EcdsaPublicKey, Self::Error> {
        let secret_bytes = self.derive_priv(path)?;
        let secret = EcdsaSecretKey::from_bytes(&secret_bytes)
            .map_err(|e| MnemonicResolverSignerError::InvalidScalar(e.to_string()))?;
        Ok(secret.public_key())
    }
}

#[async_trait]
impl ExtendedPubKeySigner for MnemonicResolverCoreSigner {
    /// Derive the BIP-32 extended public key at `path`.
    ///
    /// Returns the full [`ExtendedPubKey`] (public point + chain code) so
    /// callers can perform non-hardened child derivation locally without
    /// additional round-trips to the resolver. All intermediate private-key
    /// material is zeroized before this method returns (see
    /// [`Self::resolve_and_derive`]); `ExtendedPubKey` carries only public
    /// information and requires no further wiping.
    async fn extended_public_key(
        &self,
        path: &DerivationPath,
    ) -> Result<ExtendedPubKey, Self::Error> {
        // `ExtendedPubKey` carries only public material (chain code + point);
        // the borrowed private key never leaves `resolve_and_derive`.
        self.resolve_and_derive(path, ExtendedPubKey::from_priv)
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

        let secret = WipingSecretKey(
            secp256k1::SecretKey::from_secret_bytes(
                *signer.derive_priv(&test_path()).expect("derived scalar"),
            )
            .expect("valid scalar"),
        );
        for byte in 0..32 {
            let sighash = [byte; 32];
            let (sig, pk) = signer
                .sign_ecdsa(&test_path(), sighash)
                .await
                .expect("signing succeeds");
            pk.verify(&sighash, sig)
                .expect("signature must verify against returned pubkey");
            let expected = secret
                .0
                .sign_ecdsa(secp256k1::Message::from_digest(sighash));
            assert_eq!(sig.to_bytes(), expected.serialize_compact());
            assert_eq!(pk.to_compressed(), secret.0.public_key().serialize());
        }

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    /// The derivation-path scope-gate is the ONLY thing stopping
    /// `export_auto_accept_private_key` from being a general raw-key
    /// exfiltration primitive — it must hand back a scalar ONLY on a DIP-15
    /// auto-accept path (`m/9'/coin'/16'/expiry'`) and reject every other path.
    #[test]
    fn export_auto_accept_private_key_gates_to_the_auto_accept_path() {
        let resolver = make_resolver(english_resolve);
        let signer =
            unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

        // A well-formed auto-accept path exports its 32-byte scalar.
        let auto_accept = DerivationPath::from_str("m/9'/1'/16'/123'").expect("valid path");
        let scalar = signer
            .export_auto_accept_private_key(&auto_accept)
            .expect("a well-formed auto-accept path exports its scalar");
        assert_ne!(*scalar, [0u8; 32], "exported scalar must be non-zero");

        // Every non-auto-accept path MUST be rejected — otherwise a caller
        // could exfiltrate an identity-auth or contactInfo signing key.
        for bad in [
            "m/9'/1'/5'/0'/0'/0'/0'", // identity-auth (feature 5', wrong length)
            "m/8'/1'/16'/0'",         // wrong purpose (comps[0] != 9')
            "m/9'/1'/15'/0'",         // wrong feature (comps[2] != 16')
            "m/9'/1'/16'",            // too short (len != 4)
            "m/9'/1'/16'/0'/0'",      // too long (len != 4)
        ] {
            let path = DerivationPath::from_str(bad).expect("valid path string");
            assert!(
                matches!(
                    signer.export_auto_accept_private_key(&path),
                    Err(MnemonicResolverSignerError::DerivationFailed(_))
                ),
                "non-auto-accept path {bad} must be rejected, not exported"
            );
        }

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    /// The invitation export gate binds the fixed `9'/*/5'/3'/*` shape (purpose,
    /// feature, sub-feature), because feature `5'` is shared with the user's own
    /// identity-auth / registration-funding / top-up keys — a gate that checked
    /// only the feature could be repurposed to exfiltrate those keys. coin_type
    /// and funding_index are intentionally unconstrained (the whole sub-feature-3'
    /// subtree is vouchers-only, and real voucher paths use a non-hardened coin).
    #[test]
    fn export_invitation_private_key_gates_to_the_invitation_path() {
        let resolver = make_resolver(english_resolve);
        let signer =
            unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

        // A well-formed invitation-funding path exports its 32-byte scalar.
        let invitation = DerivationPath::from_str("m/9'/1'/5'/3'/0'").expect("valid path");
        let scalar = signer
            .export_invitation_private_key(&invitation)
            .expect("a well-formed invitation path exports its scalar");
        assert_ne!(*scalar, [0u8; 32], "exported scalar must be non-zero");

        // Every non-invitation path MUST be rejected — especially the sibling
        // sub-features that share feature `5'` (auth/registration/top-up).
        for bad in [
            "m/9'/1'/5'/0'/0'/0'/0'", // identity authentication (sub-feature 0')
            "m/9'/1'/5'/1'/0'",       // registration funding (sub-feature 1')
            "m/9'/1'/5'/2'/0'",       // top-up funding (sub-feature 2')
            "m/9'/1'/16'/123'",       // auto-accept (feature 16', not 5')
            "m/8'/1'/5'/3'/0'",       // wrong purpose (comps[0] != 9')
            "m/9'/1'/5'/3'",          // too short (len != 5)
            "m/9'/1'/5'/3'/0'/0'",    // too long (len != 5)
        ] {
            let path = DerivationPath::from_str(bad).expect("valid path string");
            assert!(
                matches!(
                    signer.export_invitation_private_key(&path),
                    Err(MnemonicResolverSignerError::DerivationFailed(_))
                ),
                "non-invitation path {bad} must be rejected, not exported"
            );
        }

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

    #[tokio::test]
    async fn extended_public_key_matches_independent_derivation() {
        let resolver = make_resolver(english_resolve);
        let signer =
            unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

        let path = test_path();
        let xpub = signer
            .extended_public_key(&path)
            .await
            .expect("extended_public_key succeeds");

        // Independently derive the expected xpub straight from the known
        // BIP-39 vector — same network + path, no resolver in the loop.
        let mnemonic = parse_mnemonic_any_language(ENGLISH_PHRASE).expect("valid phrase");
        let master = ExtendedPrivKey::new_master(Network::Testnet, &*mnemonic.to_seed(""))
            .expect("master derivation");
        let derived = master.derive_priv(&path).expect("path derivation");
        let expected = ExtendedPubKey::from_priv(&derived);

        // Field-level checks run first so a silently-dropped BIP-32 metadatum
        // fails here with a precise message — not just the public point. The
        // final full-struct assert then catches the remaining fields
        // (parent_fingerprint, child_number). Ordering matters: a leading
        // full-struct `assert_eq!` would short-circuit and make these
        // per-field asserts unreachable (i.e. vacuous) on a metadata regression.
        assert_eq!(
            xpub.public_key, expected.public_key,
            "public key must match"
        );
        assert_eq!(
            xpub.chain_code, expected.chain_code,
            "chain code must match"
        );
        assert_eq!(xpub.depth, expected.depth, "depth must match");
        assert_eq!(xpub.network, expected.network, "network must match");
        assert_eq!(
            xpub, expected,
            "full xpub must match independent derivation"
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
        use key_wallet::mnemonic::Mnemonic;
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
        let mnemonic = Mnemonic::from_phrase(ENGLISH_PHRASE).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet =
            Wallet::from_seed_bytes(*seed, Network::Testnet, WalletAccountCreationOptions::None)
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

    /// Interop guard: the signer-based ECDH shared secret must be
    /// byte-identical to the resident-seed route it replaces.
    ///
    /// The signer derives our scalar at `path` from the Keychain mnemonic and
    /// ECDHs with a peer pubkey; a `Wallet` built from the SAME mnemonic
    /// derives the scalar the old way and ECDHs through the SAME single crypto
    /// source. If they diverged, every contact-request encrypt/decrypt the
    /// signer path produces would be unreadable by the reference clients, so
    /// this pins them equal.
    #[tokio::test]
    async fn ecdh_shared_secret_matches_wallet_derivation() {
        use key_wallet::mnemonic::Mnemonic;
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::wallet::Wallet;

        let path = test_path();

        // A fixed peer keypair (the contact's encryption key).
        let peer_sk =
            secp256k1::SecretKey::from_secret_bytes([0x42u8; 32]).expect("peer secret key");
        let peer_pk = secp256k1::PublicKey::from_secret_key(&peer_sk);

        // Old route: resident-seed wallet from the same mnemonic → derive the
        // scalar at `path` → ECDH through the single crypto source.
        let mnemonic = Mnemonic::from_phrase(ENGLISH_PHRASE).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet =
            Wallet::from_seed_bytes(*seed, Network::Testnet, WalletAccountCreationOptions::None)
                .expect("seeded wallet");
        let xprv = wallet
            .derive_extended_private_key(&path)
            .expect("wallet derives the private key at path");
        let expected = platform_encryption::derive_shared_key_ecdh(
            &secp256k1::SecretKey::from_secret_bytes(*xprv.private_key.to_bytes())
                .expect("derived scalar"),
            &peer_pk,
        );

        // New route: resolver-backed signer fed the same mnemonic.
        let resolver = make_resolver(english_resolve);
        let signer =
            unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };
        let actual = signer
            .ecdh_shared_secret(&path, &peer_pk)
            .expect("signer computes the ECDH shared secret");

        // Deref to a concrete `[u8; 32]` on both sides — `Zeroizing::as_ref`
        // is ambiguous here (dashcore adds an `AsRef<PushBytes>` for `[u8; 32]`).
        let actual_bytes: [u8; 32] = *actual;
        assert_eq!(
            actual_bytes, expected,
            "signer-based ECDH must equal the resident-seed ECDH for the same mnemonic and path"
        );

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    /// Interop guard for the seedless send path: the signer-computed
    /// `accountReference` must equal the resident-seed route, and round-trip
    /// back through the signer's unmask.
    ///
    /// The send flow masks `(version, account_index)` into the reference keyed
    /// by the sender's ECDH private scalar. If the signer derived a different
    /// scalar than the resident seed, a same-seed cross-wallet recovery would
    /// unmask to the wrong account (silent — there's no on-chain oracle), so
    /// this pins the signer route equal to `Wallet`'s and confirms the inverse.
    #[tokio::test]
    async fn account_reference_matches_wallet_derivation_and_round_trips() {
        use key_wallet::mnemonic::Mnemonic;
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::wallet::Wallet;

        let path = test_path();
        // A stand-in 69-byte compact xpub; the HMAC only consumes the bytes.
        let compact_xpub: [u8; 69] = std::array::from_fn(|i| i as u8);
        let account_index = 5u32;
        let version = 3u32;

        // Old route: resident-seed wallet from the same mnemonic → derive the
        // scalar at `path` → mask through the single accountReference source.
        let mnemonic = Mnemonic::from_phrase(ENGLISH_PHRASE).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet =
            Wallet::from_seed_bytes(*seed, Network::Testnet, WalletAccountCreationOptions::None)
                .expect("seeded wallet");
        let secret = wallet
            .derive_extended_private_key(&path)
            .expect("wallet derives the private key at path")
            .private_key
            .to_bytes();
        let expected = platform_encryption::calculate_account_reference(
            &secret,
            &compact_xpub,
            account_index,
            version,
        );

        // New route: resolver-backed signer fed the same mnemonic.
        let resolver = make_resolver(english_resolve);
        let signer =
            unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };
        let actual = signer
            .account_reference(&path, &compact_xpub, account_index, version)
            .expect("signer computes the account reference");
        assert_eq!(
            actual, expected,
            "signer accountReference must equal the resident-seed mask for the same mnemonic+path"
        );

        // And the signer's own inverse recovers the inputs.
        let (got_version, got_account) = signer
            .unmask_account_reference(&path, &compact_xpub, actual)
            .expect("signer unmasks the account reference");
        assert_eq!(
            got_version, version,
            "version round-trips through the signer"
        );
        assert_eq!(
            got_account, account_index,
            "account index round-trips through the signer"
        );

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    /// contactInfo seal/open round-trips, AND the signer's AES keys are
    /// byte-identical to a resident wallet's derivation at the same DIP-15
    /// contactInfo paths (`root / 65536' / idx'` and `root / 65537' / idx'`) —
    /// so contactInfo the signer seals is readable by the reference clients.
    #[tokio::test]
    async fn contact_info_seal_open_round_trips_and_matches_wallet_derivation() {
        use key_wallet::mnemonic::Mnemonic;
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::wallet::Wallet;

        let resolver = make_resolver(english_resolve);
        let signer =
            unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

        let root_path = test_path();
        let derivation_index = 0u32;
        let contact_id = [0x33u8; 32];
        let plaintext = b"hello private data".to_vec();
        let iv = [0x11u8; 16];

        // Seal, then open — must recover the inputs.
        let sealed = signer
            .contact_info_seal(&root_path, derivation_index, &contact_id, &plaintext, &iv)
            .expect("seal");
        let opened = signer
            .contact_info_open(
                &root_path,
                derivation_index,
                &sealed.enc_to_user_id,
                &sealed.private_data,
            )
            .expect("open");
        assert_eq!(
            opened.contact_id, contact_id,
            "open recovers the contact id"
        );
        assert_eq!(
            opened.private_data, plaintext,
            "open recovers the private data"
        );

        // Parity: encToUserId equals a resident wallet's derive+encrypt.
        let mnemonic = Mnemonic::from_phrase(ENGLISH_PHRASE).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet =
            Wallet::from_seed_bytes(*seed, Network::Testnet, WalletAccountCreationOptions::None)
                .expect("seeded wallet");
        let enc_key: Zeroizing<[u8; 32]> = {
            let path = root_path.clone().extend([
                ChildNumber::from_hardened_idx(65536).unwrap(),
                ChildNumber::from_hardened_idx(derivation_index).unwrap(),
            ]);
            wallet
                .derive_extended_private_key(&path)
                .expect("derive encToUserId key")
                .private_key
                .to_bytes()
        };
        let expected_enc = platform_encryption::encrypt_enc_to_user_id(&enc_key, &contact_id);
        assert_eq!(
            sealed.enc_to_user_id, expected_enc,
            "signer encToUserId must equal the resident-seed encryption at the same path"
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

    /// The producer half of the key-unavailable contract: `NotFound` renders
    /// with the reserved machine marker at position 0. Consumers of the
    /// `Signer` surface (whose error is only `Display`) recognize the missing
    /// key by exactly this start-of-rendering marker — a mid-string move
    /// would silently break the promotion to FFI code 31 without failing any
    /// structural match.
    #[test]
    fn not_found_renders_the_key_unavailable_marker_at_position_zero() {
        let rendered = MnemonicResolverSignerError::NotFound.to_string();
        assert!(
            rendered.starts_with(crate::signer::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX),
            "NotFound must stamp the reserved marker at position 0, got: {rendered:?}"
        );
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

    mod private_contact_requests {
        use super::*;
        use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
        use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dash_sdk::dpp::identity::{
            IdentityPublicKey, IdentityV0, KeyType, Purpose, SecurityLevel,
        };
        use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dash_sdk::platform::dashpay::PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME;
        use dash_sdk::platform::private_document_entries::PrivateDocumentEntry;
        use key_wallet::mnemonic::Mnemonic;
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::wallet::Wallet;
        use std::sync::atomic::{AtomicUsize, Ordering};

        const SENDER_ID: [u8; 32] = [0xA0; 32];
        const RECIPIENT_ID: [u8; 32] = [0xB0; 32];
        /// Two identities of another wallet, whose entry the scan passes over.
        const OUTSIDER_ID: [u8; 32] = [0xC0; 32];
        const CAROL_ID: [u8; 32] = [0xD0; 32];
        const SENDER_KEY_ID: KeyID = 1;
        const RECIPIENT_KEY_ID: KeyID = 2;
        const BLOCK_TIME_MS: u64 = 1_700_000_000_000;

        /// The sender's DashPay encryption key, in the test wallet.
        fn sender_key_path() -> DerivationPath {
            DerivationPath::from_str("m/9'/1'/5'/0'/0'/0'/1'").expect("valid path")
        }

        /// The recipient's DashPay decryption key, in the test wallet.
        fn recipient_key_path() -> DerivationPath {
            DerivationPath::from_str("m/9'/1'/5'/0'/0'/1'/2'").expect("valid path")
        }

        /// The secret key a resident wallet of `ENGLISH_PHRASE` holds at `path`.
        fn seed_secret_key(path: &DerivationPath) -> secp256k1::SecretKey {
            let seed = Mnemonic::from_phrase(ENGLISH_PHRASE)
                .expect("valid mnemonic")
                .to_seed("");
            let wallet = Wallet::from_seed_bytes(
                *seed,
                Network::Testnet,
                WalletAccountCreationOptions::None,
            )
            .expect("seeded wallet");
            let xprv = wallet
                .derive_extended_private_key(path)
                .expect("wallet derives the private key at path");
            secp256k1::SecretKey::from_secret_bytes(*xprv.private_key.to_bytes())
                .expect("a valid scalar")
        }

        fn outsider_secret_key() -> secp256k1::SecretKey {
            secp256k1::SecretKey::from_secret_bytes([0x41; 32]).expect("a valid scalar")
        }

        fn key(
            id: KeyID,
            purpose: Purpose,
            secret_key: &secp256k1::SecretKey,
        ) -> IdentityPublicKey {
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id,
                purpose,
                security_level: SecurityLevel::MEDIUM,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(
                    secp256k1::PublicKey::from_secret_key(secret_key)
                        .serialize()
                        .to_vec(),
                ),
                disabled_at: None,
            })
        }

        fn identity(
            id: [u8; 32],
            key_id: KeyID,
            purpose: Purpose,
            secret_key: &secp256k1::SecretKey,
        ) -> Identity {
            let key = key(key_id, purpose, secret_key);
            Identity::V0(IdentityV0 {
                id: Identifier::from(id),
                public_keys: BTreeMap::from([(key_id, key)]),
                balance: 0,
                revision: 0,
            })
        }

        fn sender() -> Identity {
            identity(
                SENDER_ID,
                SENDER_KEY_ID,
                Purpose::ENCRYPTION,
                &seed_secret_key(&sender_key_path()),
            )
        }

        fn recipient() -> Identity {
            identity(
                RECIPIENT_ID,
                RECIPIENT_KEY_ID,
                Purpose::DECRYPTION,
                &seed_secret_key(&recipient_key_path()),
            )
        }

        fn outsider() -> Identity {
            identity(
                OUTSIDER_ID,
                SENDER_KEY_ID,
                Purpose::ENCRYPTION,
                &outsider_secret_key(),
            )
        }

        fn carol() -> Identity {
            identity(
                CAROL_ID,
                RECIPIENT_KEY_ID,
                Purpose::DECRYPTION,
                &secp256k1::SecretKey::from_secret_bytes([0x42; 32]).expect("a valid scalar"),
            )
        }

        fn dashpay() -> DataContract {
            load_system_data_contract(SystemDataContract::Dashpay, PlatformVersion::latest())
                .expect("DashPay loads at the latest version")
        }

        fn target(contract: &DataContract) -> PrivateDocumentTarget<'static> {
            PrivateDocumentTarget {
                contract_id: contract.id().to_buffer(),
                document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            }
        }

        fn request() -> PrivateContactRequest {
            PrivateContactRequest {
                account_reference: 7,
                extended_public_key: [0x33; platform_encryption::COMPACT_XPUB_LEN],
                account_label: Some("Alice".to_string()),
                auto_accept_proof: None,
                shielded_address: Some([0x55; 43]),
            }
        }

        /// `request` sealed by `sender` with `secret_key`, the key a resident
        /// wallet holds, to the decryption key of `recipient`.
        fn seed_sealed(
            contract: &DataContract,
            sender: &Identity,
            secret_key: &secp256k1::SecretKey,
            recipient: &Identity,
            esk_seed: u8,
        ) -> BinaryData {
            seal_private_contact_request(
                contract,
                &PrivateContactRequestSender {
                    identity: sender,
                    encryption_key_id: SENDER_KEY_ID,
                    encryption_secret_key: secret_key,
                },
                recipient,
                RECIPIENT_KEY_ID,
                &request(),
                &[esk_seed; 32],
            )
            .expect("the request seals")
        }

        /// The test entry: the sender's request to the recipient.
        fn sent_to_recipient(contract: &DataContract, esk_seed: u8) -> BinaryData {
            seed_sealed(
                contract,
                &sender(),
                &seed_secret_key(&sender_key_path()),
                &recipient(),
                esk_seed,
            )
        }

        fn page(entries: Vec<BinaryData>) -> PrivateDocumentEntries {
            PrivateDocumentEntries {
                total_count: entries.len() as u64,
                entries: entries
                    .into_iter()
                    .zip(0u64..)
                    .map(|(data, position)| PrivateDocumentEntry {
                        position,
                        block_height: 10 + position,
                        block_time_ms: BLOCK_TIME_MS,
                        data: data.to_vec(),
                    })
                    .collect(),
            }
        }

        /// The wallet's recipient key paths: the recipient's decryption key.
        fn recipient_paths() -> Vec<(Identifier, Vec<(KeyID, DerivationPath)>)> {
            vec![(
                Identifier::from(RECIPIENT_ID),
                vec![(RECIPIENT_KEY_ID, recipient_key_path())],
            )]
        }

        /// The wallet's sender key paths: the sender's encryption key.
        fn sender_paths() -> Vec<(Identifier, DerivationPath)> {
            vec![(Identifier::from(SENDER_ID), sender_key_path())]
        }

        /// The scan keys a resident wallet of the same seed builds.
        fn seed_scan_keys() -> PrivateDocumentScanKeys {
            PrivateDocumentScanKeys::new()
                .with_recipient(
                    Identifier::from(RECIPIENT_ID),
                    &[(RECIPIENT_KEY_ID, seed_secret_key(&recipient_key_path()))],
                )
                .with_sender(
                    Identifier::from(SENDER_ID),
                    private_contact_request_outgoing_viewing_key(&seed_secret_key(
                        &sender_key_path(),
                    )),
                )
        }

        unsafe extern "C" fn counting_resolve(
            ctx: *const c_void,
            wallet_id_bytes: *const u8,
            out_buf: *mut c_char,
            out_capacity: usize,
            out_len: *mut usize,
        ) -> i32 {
            (*(ctx as *const AtomicUsize)).fetch_add(1, Ordering::SeqCst);
            english_resolve(ctx, wallet_id_bytes, out_buf, out_capacity, out_len)
        }

        /// The entry the signer seals is byte for byte the one a resident
        /// wallet seals with the key at the same path, so a request sent
        /// through the Keychain opens and authenticates for every client.
        #[test]
        fn should_seal_the_entry_a_seed_derived_sender_key_seals() {
            let contract = dashpay();
            let resolver = make_resolver(english_resolve);
            let signer =
                unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

            let sealed = signer
                .seal_private_contact_request(
                    &sender_key_path(),
                    &contract,
                    &sender(),
                    SENDER_KEY_ID,
                    &recipient(),
                    RECIPIENT_KEY_ID,
                    &request(),
                    &[7; 32],
                )
                .expect("the signer seals the request");
            assert_eq!(sealed, sent_to_recipient(&contract, 7));

            unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
        }

        /// A page scanned and read through the signer sorts and reads as with
        /// a resident wallet's scan keys: the request to the recipient, with
        /// its sender checked, and the same request recovered as sent by the
        /// sender's outgoing viewing key, while another wallet's entry is
        /// passed over.
        #[test]
        fn should_scan_and_read_a_page_as_seed_derived_scan_keys_do() {
            let contract = dashpay();
            let target = target(&contract);
            let page = page(vec![
                seed_sealed(&contract, &outsider(), &outsider_secret_key(), &carol(), 1),
                sent_to_recipient(&contract, 2),
            ]);
            let resolver = make_resolver(english_resolve);
            let signer =
                unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

            let scan = signer
                .scan_private_document_page(
                    &recipient_paths(),
                    &sender_paths(),
                    contract.id(),
                    PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                    0,
                    &page,
                )
                .expect("the signer scans the page");
            let seed_keys = seed_scan_keys();
            let expected_scan = seed_keys.scan_page(&target, 0, &page);
            assert_eq!(scan, expected_scan);
            assert_eq!(scan.received.len(), 1, "the request to the recipient");
            assert_eq!(scan.sent.len(), 1, "the request the sender sent");
            assert!(scan.rejected.is_empty());
            assert!(scan.is_complete());

            let sender_identities = BTreeMap::from([(Identifier::from(SENDER_ID), Some(sender()))]);
            let read = signer
                .read_private_contact_requests(
                    &recipient_paths(),
                    &contract,
                    scan,
                    &sender_identities,
                    PlatformVersion::latest(),
                )
                .expect("the signer reads the requests");
            let expected_read = private_contact_requests_from_scan(
                &contract,
                expected_scan,
                &seed_keys,
                &sender_identities,
                PlatformVersion::latest(),
            )
            .expect("the resident wallet reads the requests");
            assert_eq!(read, expected_read);
            assert_eq!(read.received.len(), 1);
            assert_eq!(read.received[0].sender_id, sender().id());
            assert_eq!(read.received[0].sender_key_id, SENDER_KEY_ID);
            assert_eq!(read.received[0].recipient_key_id, RECIPIENT_KEY_ID);
            assert_eq!(read.received[0].request, request());
            assert_eq!(read.sent.len(), 1);
            assert_eq!(read.sent[0].recipient_id, recipient().id());
            assert!(read.rejected.is_empty());

            unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
        }

        /// Every key a scan or a read needs comes from one Keychain read, not
        /// one per key.
        #[test]
        fn should_resolve_the_mnemonic_once_per_scan_and_once_per_read() {
            let contract = dashpay();
            let page = page(vec![sent_to_recipient(&contract, 3)]);
            let resolves = AtomicUsize::new(0);
            let resolver = unsafe {
                dash_sdk_mnemonic_resolver_create(
                    &resolves as *const AtomicUsize as *mut c_void,
                    counting_resolve,
                    noop_destroy,
                )
            };
            let signer =
                unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };
            // Two keys for the recipient and one sender: three derivations.
            let recipients = vec![(
                Identifier::from(RECIPIENT_ID),
                vec![
                    (RECIPIENT_KEY_ID, recipient_key_path()),
                    (
                        3,
                        DerivationPath::from_str("m/9'/1'/5'/0'/0'/1'/3'").expect("valid path"),
                    ),
                ],
            )];

            let scan = signer
                .scan_private_document_page(
                    &recipients,
                    &sender_paths(),
                    contract.id(),
                    PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                    0,
                    &page,
                )
                .expect("the signer scans the page");
            assert_eq!(resolves.load(Ordering::SeqCst), 1);

            let sender_identities = BTreeMap::from([(Identifier::from(SENDER_ID), Some(sender()))]);
            signer
                .read_private_contact_requests(
                    &recipients,
                    &contract,
                    scan,
                    &sender_identities,
                    PlatformVersion::latest(),
                )
                .expect("the signer reads the requests");
            assert_eq!(resolves.load(Ordering::SeqCst), 2);

            unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
        }

        /// A path that is not the sender's encryption key derives a key the
        /// SDK refuses to seal with, and the refusal reaches the caller typed.
        #[test]
        fn should_refuse_to_seal_with_a_key_the_sender_does_not_hold() {
            let contract = dashpay();
            let resolver = make_resolver(english_resolve);
            let signer =
                unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

            let err = signer
                .seal_private_contact_request(
                    &recipient_key_path(),
                    &contract,
                    &sender(),
                    SENDER_KEY_ID,
                    &recipient(),
                    RECIPIENT_KEY_ID,
                    &request(),
                    &[7; 32],
                )
                .expect_err("the key at that path is not the sender's");
            assert!(
                matches!(err, MnemonicResolverSignerError::PrivateContactRequest(_)),
                "expected the SDK's refusal, got: {err:?}"
            );

            unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
        }

        #[test]
        fn should_report_a_missing_mnemonic_from_a_scan() {
            let contract = dashpay();
            let resolver = make_resolver(missing_resolve);
            let signer =
                unsafe { MnemonicResolverCoreSigner::new(resolver, [0u8; 32], Network::Testnet) };

            let err = signer
                .scan_private_document_page(
                    &recipient_paths(),
                    &sender_paths(),
                    contract.id(),
                    PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                    0,
                    &page(Vec::new()),
                )
                .expect_err("no mnemonic, no keys");
            assert!(
                matches!(err, MnemonicResolverSignerError::NotFound),
                "error should be NotFound, got: {err:?}"
            );

            unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
        }
    }
}
