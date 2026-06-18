//! FFI bindings for the identity-registration-key preview helper on
//! the platform-wallet [`IdentityWallet`](platform_wallet::IdentityWallet).
//!
//! Exposes [`platform_wallet_preview_identity_registration_keys`]: a
//! pure-compute view that derives the first N MASTER identity-
//! authentication keypairs the wallet would probe during a discovery
//! scan, without performing any Platform RPCs or mutating any wallet
//! state.
//!
//! This is the read-only counterpart to
//! [`crate::identity_discovery::platform_wallet_discover_identities`]:
//! the discover scan probes Platform with derived pubkey hashes and
//! folds matches into the wallet; this preview surfaces *exactly*
//! those derived keys back to the caller (path + compressed public
//! key + WIF private key) so the UI can render "here are the keys we
//! scanned for" when a discover call comes back empty.
//!
//! Policy lives entirely on the Rust side:
//! - the gap-limit default ([`platform_wallet::IDENTITY_GAP_LIMIT`])
//!   when `count_or_neg1 < 0`,
//! - the ECDSA-only key type and `m/9'/coin'/5'/0'/0'/idx'/0'`
//!   path shape ([`platform_wallet::derive_identity_auth_keypair`]),
//! - the WIF version byte (selected by the wallet's network),
//! - the MASTER key index ([`platform_wallet::MASTER_KEY_INDEX`]).
//!
//! The Swift caller knows nothing about any of those — it just reads
//! the array back out.
//!
//! # Key source: chosen by wallet capability
//!
//! The derivation source is selected by the in-process wallet's shape,
//! NOT by whether a resolver handle was supplied — the resolver is a
//! *capability* the Rust side consults only when it can't derive
//! locally, not a command that forces the resolver path:
//!
//! - **In-process wallet holds resident private keys** (`WalletType::
//!   Mnemonic` / `Seed` / `ExtendedPrivKey` — NOT external-signable and
//!   NOT watch-only): derive every row from the resident wallet under a
//!   single read lock held for the loop's duration (the historical
//!   path). The resolver handle is never touched, which also skips a
//!   pointless iOS Keychain read. This keeps `createWallet(seed:)` /
//!   raw-seed wallets working even when no BIP-39 mnemonic was ever
//!   persisted to `WalletStorage`.
//! - **In-process wallet is external-signable / watch-only:** its seed
//!   lives in iOS Keychain, NOT in process, so the resident derive
//!   would fail with `External signable wallet has no private key`. In
//!   that case, if `mnemonic_resolver_handle` is non-null, resolve the
//!   wallet's mnemonic on demand via the Swift-owned
//!   [`MnemonicResolverHandle`] (keyed by the wallet handle's own
//!   `wallet_id`), build the master `ExtendedPrivKey`, and derive each
//!   row from that master via
//!   [`derive_ecdsa_identity_auth_keypair_from_master`] — the same
//!   derive the rescan-via-resolver and the registration paths use.
//!   The mnemonic / seed / master scalar live in `Zeroizing` buffers
//!   (the master's `private_key` is explicitly `non_secure_erase`d —
//!   `ExtendedPrivKey` has no `Drop`) and are scrubbed before this
//!   function returns. This is the path the iOS app takes. If the
//!   resolver is null for such a wallet, the call returns an error
//!   hinting that a mnemonic resolver handle is required for this
//!   wallet shape.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use dashcore::PrivateKey as DashPrivateKey;
use key_wallet::bip32::ExtendedPrivKey;
use key_wallet::Wallet;
use platform_wallet::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master;
use platform_wallet::{derive_identity_auth_keypair, IDENTITY_GAP_LIMIT, MASTER_KEY_INDEX};
use zeroize::Zeroizing;

use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::{resolve_master_from_resolver, zeroize_and_free_row};
use crate::types::Network;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use rs_sdk_ffi::MnemonicResolverHandle;

/// One identity-registration-key preview row.
///
/// All heap allocations (`derivation_path`, `public_key`,
/// `private_key_wif`) are owned by Rust until released by
/// [`platform_wallet_preview_identity_registration_keys_free`]. The
/// `public_key` buffer is exactly 33 bytes — a compressed
/// secp256k1 public key — so callers can read `public_key_len`
/// defensively without assuming the constant.
///
/// `private_key_bytes` is the raw 32-byte ECDSA scalar — needed by
/// the Swift side so it can persist the key into the iOS Keychain
/// before calling [`platform_wallet_register_identity_with_signer`]
/// (the Swift `KeychainSigner` then re-reads it during
/// state-transition signing). `private_key_wif` carries the same
/// material in the human-readable WIF form for the keychain
/// explorer / debugging UI.
#[repr(C)]
pub struct IdentityKeyPreviewFFI {
    /// Identity index (BIP-9 position under the identity branch).
    pub identity_index: u32,
    /// Null-terminated UTF-8 derivation path string, e.g.
    /// `"m/9'/1'/5'/0'/0'/0'/0'"`. Heap-allocated — released by the
    /// paired free function.
    pub derivation_path: *mut c_char,
    /// Compressed secp256k1 public key bytes. Always 33 bytes.
    pub public_key: *mut u8,
    /// Length of `public_key`. Always 33.
    pub public_key_len: usize,
    /// Null-terminated UTF-8 WIF (Wallet Import Format) string for
    /// the private key. Network-aware (mainnet vs testnet/devnet/
    /// regtest version byte) and compressed.
    pub private_key_wif: *mut c_char,
    /// Raw 32-byte ECDSA private-key scalar. Inline — no heap
    /// allocation — so the freed-rows path doesn't need to chase a
    /// pointer for it. Treat as sensitive material: the Swift side
    /// is expected to copy it straight into the iOS Keychain and
    /// drop the local reference.
    pub private_key_bytes: [u8; 32],
}

impl IdentityKeyPreviewFFI {
    /// All-null / zero row used by single-row callers
    /// (e.g. `dash_sdk_derive_identity_key_at_slot`) to pre-zero
    /// their `out_row` so a failed FFI call leaves the caller
    /// staring at known empty state instead of uninitialized
    /// memory.
    pub fn empty() -> Self {
        Self {
            identity_index: 0,
            derivation_path: ptr::null_mut(),
            public_key: ptr::null_mut(),
            public_key_len: 0,
            private_key_wif: ptr::null_mut(),
            private_key_bytes: [0u8; 32],
        }
    }
}

/// Heap-allocated array of [`IdentityKeyPreviewFFI`] rows. Release
/// the whole struct (rows + their owned strings + key buffers) by
/// handing it back to
/// [`platform_wallet_preview_identity_registration_keys_free`]. Safe
/// to free on a zero / null struct (no-op).
#[repr(C)]
pub struct IdentityKeyPreviewsFFI {
    /// Pointer to a contiguous `[IdentityKeyPreviewFFI; count]`
    /// buffer. Null when `count == 0`.
    pub items: *mut IdentityKeyPreviewFFI,
    /// Number of rows in `items`.
    pub count: usize,
}

impl IdentityKeyPreviewsFFI {
    fn empty() -> Self {
        Self {
            items: ptr::null_mut(),
            count: 0,
        }
    }
}

/// Derive the first `count_or_neg1` MASTER identity-authentication
/// keypairs this wallet would probe during a discovery scan,
/// starting at identity index `start_index`.
///
/// The derivation source is chosen by the in-process wallet's
/// capability (see the module docs): resident-key wallets derive
/// locally and never touch the resolver; external-signable / watch-only
/// wallets consult the resolver. The resolver is only *needed* for the
/// latter shape.
///
/// # Parameters
/// - `wallet_handle` — platform-wallet handle.
/// - `mnemonic_resolver_handle` — Swift-owned
///   [`MnemonicResolverHandle`], consulted **only** when the in-process
///   wallet lacks resident private keys (external-signable / watch-only
///   — the iOS Keychain-backed `WalletType::ExternalSignable` shape
///   whose seed is not in process). For such a wallet, when non-null
///   the mnemonic is resolved on demand (keyed by the wallet handle's
///   own `wallet_id`), a master `ExtendedPrivKey` is built, and each row
///   is derived from that master; when null the call errors with a hint
///   that a resolver handle is required for this wallet shape. For a
///   wallet that holds resident private keys this argument is ignored
///   and the rows are derived from the in-process wallet (the
///   historical path).
/// - `start_index` — first identity index to derive.
/// - `count_or_neg1` — number of consecutive identity indices to
///   derive. Pass `< 0` to use the Rust default
///   ([`IDENTITY_GAP_LIMIT`], currently 5) so the preview matches
///   the scan window `discover()` walks.
/// - `out_previews` — populated on success with a heap-allocated
///   array. Release with
///   [`platform_wallet_preview_identity_registration_keys_free`]. On
///   error the struct is left at the empty zero state.
///
/// # Safety
/// `wallet_handle` must come from the platform-wallet handle
/// registry. `mnemonic_resolver_handle`, when non-null, must come
/// from [`rs_sdk_ffi::dash_sdk_mnemonic_resolver_create`] and remain
/// valid for the duration of the call. `out_previews` must be a
/// valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_preview_identity_registration_keys(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    start_index: u32,
    count_or_neg1: i32,
    out_previews: *mut IdentityKeyPreviewsFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_previews);

    // Pre-clear so partial failures don't leave the caller staring
    // at uninitialized memory.
    unsafe { *out_previews = IdentityKeyPreviewsFFI::empty() };

    let count: u32 = if count_or_neg1 < 0 {
        IDENTITY_GAP_LIMIT
    } else {
        count_or_neg1 as u32
    };

    if count == 0 {
        // Empty preview is a valid result — early-out before
        // touching the wallet manager.
        return PlatformWalletFFIResult::ok();
    }

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        // Resolve the network from the wallet handle. `wallet.network()`
        // returns `dashcore::Network`, the type both `new_master` and the
        // derive helpers use.
        let network: Network = wallet.network();

        // Per-row materials: (path string, secp256k1 public key bytes,
        // 32-byte private scalar). Both key sources funnel through this
        // so the row-building (WIF, pubkey buffer, zeroize-on-error) is
        // written exactly once.
        struct RowMaterial {
            path: String,
            public_key: [u8; 33],
            private_key: Zeroizing<[u8; 32]>,
        }

        // Build one heap-detached FFI row from already-derived material.
        // All fallible work runs before any `into_raw` / `mem::forget`
        // so an early `?` cleans up via Drop.
        let build_row = |identity_index: u32,
                         material: RowMaterial|
         -> Result<IdentityKeyPreviewFFI, PlatformWalletFFIResult> {
            let path_cstring = CString::new(material.path)?;

            // WIF: network-aware (mainnet → 0xCC, testnet/devnet/
            // regtest → 0xEF) and compressed. Same construction
            // `key_wallet::derive_private_key_as_wif` performs.
            let secret_key = dashcore::secp256k1::SecretKey::from_slice(
                material.private_key.as_ref(),
            )
            .map_err(|e| {
                PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    format!("SecretKey::from_slice failed: {e}"),
                )
            })?;
            let dash_private = DashPrivateKey {
                compressed: true,
                network,
                inner: secret_key,
            };
            let wif_cstring = CString::new(dash_private.to_wif())?;

            // Compressed secp256k1 pubkey is always 33 bytes.
            let mut pub_box: Box<[u8]> = material.public_key.to_vec().into_boxed_slice();
            let pub_ptr = pub_box.as_mut_ptr();
            let pub_len = pub_box.len();
            std::mem::forget(pub_box);

            Ok(IdentityKeyPreviewFFI {
                identity_index,
                derivation_path: path_cstring.into_raw(),
                public_key: pub_ptr,
                public_key_len: pub_len,
                private_key_wif: wif_cstring.into_raw(),
                private_key_bytes: *material.private_key,
            })
        };

        // Borrowed per-row key source. The two derivation paths are
        // symmetric in shape: both hand `derive_material` a borrowed
        // source and get back `RowMaterial`. The resident-wallet
        // variant borrows the `&Wallet` looked up under a read guard
        // re-acquired for the loop's duration (the derive is pure
        // compute, so holding it across the loop is fine — but the guard
        // is NEVER held across the Swift resolver callback, see below);
        // the master variant borrows the resolved master xpriv and needs
        // no guard at all.
        enum DeriveSource<'a> {
            /// In-process wallet holds resident private keys — derive
            /// each row directly from it (no per-row lock acquisition).
            Resident(&'a Wallet),
            /// External-signable / watch-only wallet — derive each row
            /// from the master xpriv resolved from the wallet's mnemonic.
            Master(&'a ExtendedPrivKey),
        }

        // Derive one row's material from the active borrowed key source.
        let derive_material = |identity_index: u32,
                               source: &DeriveSource|
         -> Result<RowMaterial, PlatformWalletFFIResult> {
            match source {
                DeriveSource::Master(master) => {
                    // External-signable / watch-only path: pure derive
                    // from the resolved master, identical to the
                    // registration / rescan-via-resolver derive.
                    let derived = derive_ecdsa_identity_auth_keypair_from_master(
                        master,
                        network,
                        identity_index,
                        MASTER_KEY_INDEX,
                    )?;
                    Ok(RowMaterial {
                        path: derived.derivation_path.to_string(),
                        public_key: derived.public_key,
                        private_key: derived.private_key,
                    })
                }
                DeriveSource::Resident(key_wallet) => {
                    // Resident-wallet path: derive from the in-process
                    // wallet. The read guard + `&Wallet` were re-acquired
                    // once before the loop (see below) so this is a
                    // pure secp256k1 pass with no per-row locking.
                    let (path, ext_priv, public_key) = derive_identity_auth_keypair(
                        key_wallet,
                        network,
                        identity_index,
                        MASTER_KEY_INDEX,
                    )?;
                    Ok(RowMaterial {
                        path: path.to_string(),
                        public_key: public_key.serialize(),
                        private_key: Zeroizing::new(ext_priv.private_key.secret_bytes()),
                    })
                }
            }
        };

        // Everything from here on can fail with a `PlatformWalletFFIResult`;
        // run it in a closure returning `Result<Vec<_>, _>`.
        //
        // Two-phase locking, mirroring the discovery path
        // (`platform_wallet_discover_identities`):
        //   1. A SHORT read-guard block scoped to the capability check
        //      only — read the wallet's shape, capture
        //      `wallet_has_resident_keys`, then DROP the guard.
        //   2. The wallet-manager read guard is NEVER held across the
        //      Swift resolver callback (`resolve_master_from_resolver`
        //      synchronously re-enters Swift and reads the iOS Keychain,
        //      which can stall on biometric unlock) — invariant called
        //      out in review.
        //   3. Only the resident branch re-acquires the guard, and only
        //      for the loop's duration (its `derive_material` borrows
        //      `&Wallet`). The master branch holds no guard past the
        //      capability check.
        let build_result = (|| -> Result<Vec<IdentityKeyPreviewFFI>, PlatformWalletFFIResult> {
            // Phase 1 — short capability-check guard. Read the wallet's
            // shape under a read-lock and DROP it before any resolver
            // interaction. Resident private keys (Mnemonic / Seed /
            // ExtendedPrivKey) → historical in-process derive; the
            // resolver is never touched (also skips a pointless iOS
            // Keychain read). Otherwise the master xpriv resolved from
            // the wallet's mnemonic is required.
            let wallet_has_resident_keys = {
                let wm = wallet.wallet_manager().blocking_read();
                match wm.get_wallet(&wallet.wallet_id()) {
                    Some(key_wallet) => {
                        !key_wallet.is_external_signable() && !key_wallet.is_watch_only()
                    }
                    None => {
                        return Err(PlatformWalletFFIResult::err(
                            PlatformWalletFFIResultCode::ErrorInvalidHandle,
                            "Wallet not found in wallet manager",
                        ));
                    }
                }
            };

            // For the resolver path, resolve the mnemonic once and build
            // the master xpriv up front (NO guard held — see the
            // two-phase note above); the per-row derive then reuses it.
            // Self-pin on the wallet handle's own `wallet_id` (same
            // rationale as `dash_sdk_derive_identity_key_at_slot_with_resolver`).
            //
            // `master_opt` outlives `source` below (which borrows it),
            // and its inner scalar is wiped just before this closure
            // returns — see the `non_secure_erase` at the bottom.
            let mut master_opt: Option<ExtendedPrivKey> = None;
            if !wallet_has_resident_keys {
                if mnemonic_resolver_handle.is_null() {
                    return Err(PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorWalletOperation,
                        "this wallet has no resident private keys (external-signable / \
                         watch-only); a mnemonic resolver handle is required to preview \
                         its identity-registration keys",
                    ));
                }
                let wallet_id = wallet.wallet_id();
                // SAFETY: handle is non-null (checked) and the caller's
                // safety contract guarantees it came from
                // `dash_sdk_mnemonic_resolver_create`.
                master_opt = Some(unsafe {
                    resolve_master_from_resolver(mnemonic_resolver_handle, &wallet_id, network)?
                });
            }

            // Phase 3 — derive + build every row from the borrowed key
            // source, inside a block so both the borrowed `source` and
            // the resident-path read guard release at the block's end —
            // BEFORE we wipe the resolved master's scalar below. The
            // master branch needs no guard. The resident branch
            // re-acquires the read guard and re-looks-up the `&Wallet`,
            // both living through the loop via a guard binding so the
            // borrow outlives `derive_material`'s calls.
            //
            // On any failure we free the rows appended so far and capture
            // the error — we must still wipe the master's scalar below,
            // so the loop result is captured rather than `?`-returned.
            let loop_result = {
                let mut loop_guard = None;
                let source = match master_opt.as_ref() {
                    Some(master) => DeriveSource::Master(master),
                    None => {
                        let wm = loop_guard.insert(wallet.wallet_manager().blocking_read());
                        let key_wallet = wm.get_wallet(&wallet.wallet_id()).ok_or_else(|| {
                            PlatformWalletFFIResult::err(
                                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                                "Wallet not found in wallet manager",
                            )
                        })?;
                        DeriveSource::Resident(key_wallet)
                    }
                };

                (|| -> Result<Vec<IdentityKeyPreviewFFI>, PlatformWalletFFIResult> {
                    let mut rows: Vec<IdentityKeyPreviewFFI> = Vec::with_capacity(count as usize);
                    for offset in 0..count {
                        // Saturating add: the discovery scan caps identity
                        // indices well below u32::MAX in practice; if a caller
                        // intentionally passes near-max values we simply repeat
                        // the cap rather than wrap.
                        let identity_index = start_index.saturating_add(offset);
                        let material = match derive_material(identity_index, &source) {
                            Ok(m) => m,
                            Err(e) => {
                                free_rows(rows);
                                return Err(e);
                            }
                        };
                        match build_row(identity_index, material) {
                            Ok(row) => rows.push(row),
                            Err(e) => {
                                // Free everything we've successfully appended
                                // so far — we never hand a partial array back.
                                // TODO: Implement Drop instead of manually drop so ? op is usable
                                free_rows(rows);
                                return Err(e);
                            }
                        }
                    }
                    Ok(rows)
                })()
                // `source` (and `loop_guard`, the resident-path read
                // guard) drop at this block's end, releasing the
                // wallet-manager read lock held across the loop and the
                // borrow into `master_opt` — so the master wipe below is
                // free to mutate it.
            };

            // TODO(upstream): `ExtendedPrivKey` has no `Drop` / `Zeroize`;
            // wipe the resolved master's inner secp256k1 scalar
            // explicitly. Same hygiene as the discovery resolver path.
            // No-op on the resident path (no master was resolved).
            if let Some(mut master) = master_opt {
                master.private_key.non_secure_erase();
            }
            loop_result
        })();

        build_result
    });
    let result = unwrap_option_or_return!(option);
    let rows = unwrap_result_or_return!(result);

    let mut boxed_items = rows.into_boxed_slice();
    let items_ptr = boxed_items.as_mut_ptr();
    let items_count = boxed_items.len();
    std::mem::forget(boxed_items);

    unsafe {
        *out_previews = IdentityKeyPreviewsFFI {
            items: items_ptr,
            count: items_count,
        };
    }
    PlatformWalletFFIResult::ok()
}

/// Release an [`IdentityKeyPreviewsFFI`] previously populated by
/// [`platform_wallet_preview_identity_registration_keys`]. Safe to
/// call on a zero / null struct or null outer pointer (no-op).
/// Each row's owned strings (`derivation_path`, `private_key_wif`)
/// and pubkey buffer are reclaimed.
///
/// Pointer-only signature: `IdentityKeyPreviewsFFI` is a 16-byte
/// aggregate at the AAPCS64 / Swift cliff so by-value isn't safe
/// across `@_silgen_name`. After the call the fields are reset so
/// a double-free no-ops.
///
/// # Safety
/// `previews.items` must have been handed out by
/// [`platform_wallet_preview_identity_registration_keys`] and must
/// not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_preview_identity_registration_keys_free(
    previews: *mut IdentityKeyPreviewsFFI,
) {
    if previews.is_null() {
        return;
    }
    let previews = unsafe { &mut *previews };
    if previews.items.is_null() || previews.count == 0 {
        return;
    }
    unsafe {
        let slice = std::slice::from_raw_parts_mut(previews.items, previews.count);
        let mut boxed: Box<[IdentityKeyPreviewFFI]> = Box::from_raw(slice);
        // Reclaim each row's owned allocations before the rows
        // themselves get dropped (which by itself wouldn't free
        // them — they're raw pointers from `into_raw` /
        // `forget(Box::...)`).
        for row in boxed.iter_mut() {
            release_row(row);
        }
        drop(boxed);
    }
    previews.items = std::ptr::null_mut();
    previews.count = 0;
}

/// Reclaim the heap allocations owned by a single preview row and zeroize the
/// sensitive private-key material it carried.
///
/// This is a thin delegation to [`zeroize_and_free_row`], the single canonical
/// zeroize-and-free implementation for [`IdentityKeyPreviewFFI`]. Keeping the
/// logic in exactly one place is the whole point of this change: a future change
/// to the row (a new sensitive `*mut c_char` field, moving `private_key_bytes`
/// behind a `Zeroizing` wrapper, etc.) then cannot silently leave one free path
/// scrubbing while another leaks. The longer-term endpoint noted at the
/// build-loop `TODO` is a `Drop` impl on the struct itself, which would make the
/// single zeroization site enforced by construction.
unsafe fn release_row(row: &mut IdentityKeyPreviewFFI) {
    unsafe { zeroize_and_free_row(row) }
}

/// Release every row in a partially-built preview list and consume
/// the vec. Used by the build-loop cleanup branch.
fn free_rows(mut rows: Vec<IdentityKeyPreviewFFI>) {
    for row in &mut rows {
        // SAFETY: each row was just appended by the build loop
        // (CString::into_raw + Vec::into_raw). They have not been
        // exposed across the FFI boundary yet, so this is the
        // single, sole release.
        unsafe { release_row(row) };
    }
    drop(rows);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a heap-detached row the same way the production build
    /// loop does (CString::into_raw + Vec leak), so `release_row` is
    /// exercised on genuinely-owned allocations rather than borrowed
    /// stack data.
    fn make_owned_row(secret: [u8; 32]) -> IdentityKeyPreviewFFI {
        let path = CString::new("m/9'/1'/5'/0'/0'/0'/0'").unwrap();
        let wif = CString::new("cQ_fake_wif_for_test_only_not_a_real_key").unwrap();
        let mut pub_box: Box<[u8]> = vec![0x02u8; 33].into_boxed_slice();
        let pub_ptr = pub_box.as_mut_ptr();
        let pub_len = pub_box.len();
        std::mem::forget(pub_box);

        IdentityKeyPreviewFFI {
            identity_index: 0,
            derivation_path: path.into_raw(),
            public_key: pub_ptr,
            public_key_len: pub_len,
            private_key_wif: wif.into_raw(),
            private_key_bytes: secret,
        }
    }

    /// `release_row` scrubs the inline 32-byte scalar in place and
    /// nulls every owned pointer, leaving the row safe to release a
    /// second time (double-free idempotency).
    #[test]
    fn release_row_zeroizes_secret_and_is_idempotent() {
        let secret = [0xABu8; 32];
        let mut row = make_owned_row(secret);

        // Sanity: the row starts out carrying the real secret and
        // owns its allocations.
        assert_eq!(row.private_key_bytes, secret);
        assert!(!row.derivation_path.is_null());
        assert!(!row.private_key_wif.is_null());
        assert!(!row.public_key.is_null());

        // SAFETY: `row` owns freshly-detached allocations and has not
        // crossed the FFI boundary, so this is the sole release.
        unsafe { release_row(&mut row) };

        // The raw scalar is wiped in place.
        assert_eq!(
            row.private_key_bytes, [0u8; 32],
            "private_key_bytes must be zeroized after release_row"
        );
        // Every owned pointer is nulled so a second release no-ops.
        assert!(row.derivation_path.is_null());
        assert!(row.private_key_wif.is_null());
        assert!(row.public_key.is_null());
        assert_eq!(row.public_key_len, 0);

        // Second release must not double-free or panic.
        unsafe { release_row(&mut row) };
        assert_eq!(row.private_key_bytes, [0u8; 32]);
    }

    /// The public preview → free round-trip wipes secrets and resets
    /// the outer struct so a second free is a no-op. We build the
    /// rows directly (the public derive path needs a live wallet
    /// handle) and drive them through the real `_free` entry point.
    #[test]
    fn public_free_zeroizes_and_resets() {
        let secret = [0x5Au8; 32];
        let rows = vec![make_owned_row(secret), make_owned_row(secret)];
        let mut boxed = rows.into_boxed_slice();
        let items_ptr = boxed.as_mut_ptr();
        let items_count = boxed.len();
        std::mem::forget(boxed);

        let mut previews = IdentityKeyPreviewsFFI {
            items: items_ptr,
            count: items_count,
        };

        // SAFETY: `previews.items` was detached above exactly as the
        // production derive path does; this is the sole free.
        unsafe { platform_wallet_preview_identity_registration_keys_free(&mut previews) };

        assert!(previews.items.is_null());
        assert_eq!(previews.count, 0);

        // Idempotent: a second free on the reset struct no-ops.
        unsafe { platform_wallet_preview_identity_registration_keys_free(&mut previews) };
        assert!(previews.items.is_null());
        assert_eq!(previews.count, 0);
    }

    /// `free_rows` (the mid-loop cleanup path) zeroizes secrets and
    /// must not panic on a partially-built list.
    #[test]
    fn free_rows_zeroizes_partial_list() {
        let rows = vec![make_owned_row([0x11u8; 32]), make_owned_row([0x22u8; 32])];
        // No assertion on the scalar after the fact — `free_rows`
        // consumes the vec — but it must release every owned
        // allocation without panicking or double-freeing.
        free_rows(rows);
    }
}
