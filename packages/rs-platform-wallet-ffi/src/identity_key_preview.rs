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

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use dashcore::PrivateKey as DashPrivateKey;
use platform_wallet::{derive_identity_auth_keypair, IDENTITY_GAP_LIMIT, MASTER_KEY_INDEX};

use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::zeroize_and_free_row;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

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
/// # Parameters
/// - `wallet_handle` — platform-wallet handle.
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
/// registry. `out_previews` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_preview_identity_registration_keys(
    wallet_handle: Handle,
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
        // Synchronous read of the wallet manager — FFI callers come
        // in on non-tokio threads.
        let wm = wallet.wallet_manager().blocking_read();
        let key_wallet = wm.get_wallet(&wallet.wallet_id()).ok_or_else(|| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                "Wallet not found in wallet manager",
            )
        })?;
        let network = key_wallet.network;

        // Build a single row. All fallible work runs first; raw-
        // pointer detachment (`into_raw`, `mem::forget`) happens at
        // the very end so an early `?` cleans up via Drop.
        let build_row =
            |identity_index: u32| -> Result<IdentityKeyPreviewFFI, PlatformWalletFFIResult> {
                let (path, ext_priv, public_key) = derive_identity_auth_keypair(
                    key_wallet,
                    network,
                    identity_index,
                    MASTER_KEY_INDEX,
                )?;

                let path_cstring = CString::new(path.to_string())?;

                // WIF: network-aware (mainnet → 0xCC, testnet/devnet/
                // regtest → 0xEF) and compressed. Same construction
                // `key_wallet::derive_private_key_as_wif` performs.
                let dash_private = DashPrivateKey {
                    compressed: true,
                    network,
                    inner: ext_priv.private_key,
                };
                let wif_cstring = CString::new(dash_private.to_wif())?;

                // Compressed secp256k1 pubkey is always 33 bytes.
                let pub_bytes: [u8; 33] = public_key.serialize();
                let mut pub_box: Box<[u8]> = pub_bytes.to_vec().into_boxed_slice();
                let pub_ptr = pub_box.as_mut_ptr();
                let pub_len = pub_box.len();
                std::mem::forget(pub_box);

                Ok(IdentityKeyPreviewFFI {
                    identity_index,
                    derivation_path: path_cstring.into_raw(),
                    public_key: pub_ptr,
                    public_key_len: pub_len,
                    private_key_wif: wif_cstring.into_raw(),
                    private_key_bytes: ext_priv.private_key.secret_bytes(),
                })
            };

        let mut rows: Vec<IdentityKeyPreviewFFI> = Vec::with_capacity(count as usize);
        for offset in 0..count {
            // Saturating add: the discovery scan caps identity
            // indices well below u32::MAX in practice; if a caller
            // intentionally passes near-max values we simply repeat
            // the cap rather than wrap.
            let identity_index = start_index.saturating_add(offset);
            match build_row(identity_index) {
                Ok(row) => rows.push(row),
                Err(e) => {
                    // Free everything we've successfully appended so
                    // far — we never hand a partial array back.
                    // TODO: Implement Drop instead of manually drop so ? op is usable
                    free_rows(rows);
                    return Err(e);
                }
            }
        }
        Ok(rows)
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
