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
//! - **The `WipingSecretKey` RAII guard** scrubs the raw
//!   [`secp256k1::SecretKey`] copies at the two sign sites, where the
//!   scalar comes back out of `SecretKey::from_slice`. `SecretKey` is an
//!   upstream secp256k1 type with no `Zeroize` impl (only
//!   `non_secure_erase()`), so it can't ride a `Zeroizing` wrapper; the
//!   guard wipes it on every exit path — normal return, `?`-early-return,
//!   and panic-unwind — closing the one leak window a bare inline erase
//!   would leave open between construction and the scrub.
//!
//! Combined, no private key bytes survive past the trait-method
//! boundary.

use std::ffi::c_void;
use std::os::raw::c_char;

use async_trait::async_trait;
use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey, ExtendedPubKey};
use key_wallet::dashcore::secp256k1::{self, Secp256k1};
use key_wallet::signer::{ExtendedPubKeySigner, Signer, SignerMethod};
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

    /// Resolve the mnemonic from the Swift-side callback, derive the BIP-32
    /// extended private key at `path`, and hand it *by reference* to
    /// `extract`, returning whatever `extract` produces.
    ///
    /// This is the single entry-point for all private-key material in this
    /// signer. It handles the full stack: resolver FFI call → result-code
    /// mapping → UTF-8 + word-list validation → BIP-39 seed → master
    /// `ExtendedPrivKey` → child `ExtendedPrivKey` at `path`.
    ///
    /// # Zeroization contract
    ///
    /// Both the `master` and `derived` extended keys wipe their secret
    /// material when they leave this scope — [`ExtendedPrivKey`] zeroizes on
    /// `Drop` and is not `Copy`, so each move is a real move that leaves no
    /// bitwise duplicate behind. The
    /// key never crosses the call boundary — `extract` only borrows it — so it
    /// cannot outlive the derivation. `extract` returns public material
    /// (`ExtendedPubKey`) or a `Zeroizing` scalar copy; the caller wipes the
    /// latter on its own drop. The mnemonic and seed buffers are plain arrays
    /// and ride [`Zeroizing`] wrappers for the same guarantee.
    ///
    /// # Errors
    ///
    /// Propagates [`MnemonicResolverSignerError`] for every failure mode:
    /// null handle, resolver FFI errors, encoding/parse failures, and BIP-32
    /// derivation errors.
    fn resolve_and_derive<T>(
        &self,
        path: &DerivationPath,
        extract: impl FnOnce(&ExtendedPrivKey) -> T,
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
        let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(""));
        drop(mnemonic);

        let secp = Secp256k1::new();
        let master = ExtendedPrivKey::new_master(self.network, seed.as_ref())
            .map_err(|e| MnemonicResolverSignerError::DerivationFailed(format!("master: {e}")))?;
        let derived = master
            .derive_priv(&secp, path)
            .map_err(|e| MnemonicResolverSignerError::DerivationFailed(format!("path: {e}")))?;

        Ok(extract(&derived))
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
        // `secret_bytes()` copies the scalar out of the borrowed key; the
        // `ExtendedPrivKey` itself never leaves `resolve_and_derive`.
        self.resolve_and_derive(path, |derived| {
            Zeroizing::new(derived.private_key.secret_bytes())
        })
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
        let purpose9 = ChildNumber::from_hardened_idx(9)
            .map_err(|e| MnemonicResolverSignerError::DerivationFailed(e.to_string()))?;
        let feature16 = ChildNumber::from_hardened_idx(16)
            .map_err(|e| MnemonicResolverSignerError::DerivationFailed(e.to_string()))?;
        let comps: &[ChildNumber] = path.as_ref();
        if comps.len() != 4 || comps[0] != purpose9 || comps[2] != feature16 {
            return Err(MnemonicResolverSignerError::DerivationFailed(
                "export_auto_accept_private_key: path is not an auto-accept path".to_string(),
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
        let shared = self.resolve_and_derive(path, |derived| {
            platform_encryption::derive_shared_key_ecdh(&derived.private_key, peer_pubkey)
        })?;
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
        let enc_key = self.derive_contact_info_aes_key(root_path, 65536, derivation_index)?;
        let priv_key = self.derive_contact_info_aes_key(root_path, 65537, derivation_index)?;
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
        let enc_key = self.derive_contact_info_aes_key(root_path, 65536, derivation_index)?;
        let priv_key = self.derive_contact_info_aes_key(root_path, 65537, derivation_index)?;
        let private_data = platform_encryption::decrypt_private_data(&priv_key, private_data_blob)
            .map_err(|e| {
                MnemonicResolverSignerError::DerivationFailed(format!("contactInfo decrypt: {e}"))
            })?;
        Ok(ContactInfoOpened {
            contact_id: platform_encryption::decrypt_enc_to_user_id(&enc_key, enc_to_user_id),
            private_data,
        })
    }
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

/// RAII guard that wipes a [`secp256k1::SecretKey`]'s scalar on `Drop`.
///
/// `SecretKey` is an upstream secp256k1 type with no `Zeroize` impl (only
/// `non_secure_erase()`), so it can't ride a `Zeroizing` wrapper. Wrapping the
/// `SecretKey::from_slice` copy here wipes it on every exit path — normal
/// return, `?`-early-return, and panic-unwind — closing the leak window a bare
/// inline `non_secure_erase()` would leave open between construction and the
/// manual scrub. This is the one key intermediate that upstream key-wallet's
/// `Zeroize`/`Drop` on `ExtendedPrivKey` cannot cover.
struct WipingSecretKey(secp256k1::SecretKey);

impl Drop for WipingSecretKey {
    fn drop(&mut self) {
        self.0.non_secure_erase();
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
        // legitimate field element. The `WipingSecretKey` guard scrubs this
        // separate copy on every exit path, including a panic between here and
        // the return — `Zeroizing<[u8;32]>` already covers `secret_bytes`.
        let secret = WipingSecretKey(
            secp256k1::SecretKey::from_slice(secret_bytes.as_ref())
                .map_err(|e| MnemonicResolverSignerError::InvalidScalar(e.to_string()))?,
        );
        let msg = secp256k1::Message::from_digest(sighash);
        let signature = secp.sign_ecdsa(&msg, &secret.0);
        let pubkey = secp256k1::PublicKey::from_secret_key(&secp, &secret.0);
        Ok((signature, pubkey))
    }

    async fn public_key(&self, path: &DerivationPath) -> Result<secp256k1::PublicKey, Self::Error> {
        let secret_bytes = self.derive_priv(path)?;
        let secp = Secp256k1::new();
        // `WipingSecretKey` scrubs this `from_slice` copy on every exit path,
        // including panic-unwind — `Zeroizing<[u8;32]>` covers `secret_bytes`.
        let secret = WipingSecretKey(
            secp256k1::SecretKey::from_slice(secret_bytes.as_ref())
                .map_err(|e| MnemonicResolverSignerError::InvalidScalar(e.to_string()))?,
        );
        let pubkey = secp256k1::PublicKey::from_secret_key(&secp, &secret.0);
        Ok(pubkey)
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
        let secp = Secp256k1::new();
        // `ExtendedPubKey` carries only public material (chain code + point);
        // the borrowed private key never leaves `resolve_and_derive`.
        self.resolve_and_derive(path, |derived| ExtendedPubKey::from_priv(&secp, derived))
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
        let secp = Secp256k1::new();
        let mnemonic = parse_mnemonic_any_language(ENGLISH_PHRASE).expect("valid phrase");
        let master = ExtendedPrivKey::new_master(Network::Testnet, &mnemonic.to_seed(""))
            .expect("master derivation");
        let derived = master.derive_priv(&secp, &path).expect("path derivation");
        let expected = ExtendedPubKey::from_priv(&secp, &derived);

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
        use key_wallet::mnemonic::{Language, Mnemonic};
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::wallet::Wallet;

        let path = test_path();

        // A fixed peer keypair (the contact's encryption key).
        let secp = Secp256k1::new();
        let peer_sk = secp256k1::SecretKey::from_slice(&[0x42u8; 32]).expect("peer secret key");
        let peer_pk = secp256k1::PublicKey::from_secret_key(&secp, &peer_sk);

        // Old route: resident-seed wallet from the same mnemonic → derive the
        // scalar at `path` → ECDH through the single crypto source.
        let mnemonic =
            Mnemonic::from_phrase(ENGLISH_PHRASE, Language::English).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet =
            Wallet::from_seed_bytes(seed, Network::Testnet, WalletAccountCreationOptions::None)
                .expect("seeded wallet");
        let xprv = wallet
            .derive_extended_private_key(&path)
            .expect("wallet derives the private key at path");
        let expected = platform_encryption::derive_shared_key_ecdh(&xprv.private_key, &peer_pk);

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
        use key_wallet::mnemonic::{Language, Mnemonic};
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::wallet::Wallet;

        let path = test_path();
        // A stand-in 69-byte compact xpub; the HMAC only consumes the bytes.
        let compact_xpub: [u8; 69] = std::array::from_fn(|i| i as u8);
        let account_index = 5u32;
        let version = 3u32;

        // Old route: resident-seed wallet from the same mnemonic → derive the
        // scalar at `path` → mask through the single accountReference source.
        let mnemonic =
            Mnemonic::from_phrase(ENGLISH_PHRASE, Language::English).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet =
            Wallet::from_seed_bytes(seed, Network::Testnet, WalletAccountCreationOptions::None)
                .expect("seeded wallet");
        let secret = wallet
            .derive_extended_private_key(&path)
            .expect("wallet derives the private key at path")
            .private_key
            .secret_bytes();
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
        use key_wallet::mnemonic::{Language, Mnemonic};
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
        let mnemonic =
            Mnemonic::from_phrase(ENGLISH_PHRASE, Language::English).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet =
            Wallet::from_seed_bytes(seed, Network::Testnet, WalletAccountCreationOptions::None)
                .expect("seeded wallet");
        let enc_key: [u8; 32] = {
            let path = root_path.clone().extend([
                ChildNumber::from_hardened_idx(65536).unwrap(),
                ChildNumber::from_hardened_idx(derivation_index).unwrap(),
            ]);
            wallet
                .derive_extended_private_key(&path)
                .expect("derive encToUserId key")
                .private_key
                .secret_bytes()
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
