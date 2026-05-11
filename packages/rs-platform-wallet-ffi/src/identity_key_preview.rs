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
        let boxed: Box<[IdentityKeyPreviewFFI]> = Box::from_raw(slice);
        // Reclaim each row's owned allocations before the rows
        // themselves get dropped (which by itself wouldn't free
        // them — they're raw pointers from `into_raw` /
        // `forget(Box::...)`).
        for row in boxed.iter() {
            release_row(row);
        }
        drop(boxed);
    }
    previews.items = std::ptr::null_mut();
    previews.count = 0;
}

/// Reclaim the heap allocations owned by a single preview row.
/// Used by both the public free function (post-success path) and
/// the internal cleanup helper that drops successfully-built rows
/// when a later derivation fails mid-loop.
unsafe fn release_row(row: &IdentityKeyPreviewFFI) {
    if !row.derivation_path.is_null() {
        unsafe {
            drop(CString::from_raw(row.derivation_path));
        }
    }
    if !row.private_key_wif.is_null() {
        unsafe {
            drop(CString::from_raw(row.private_key_wif));
        }
    }
    if !row.public_key.is_null() && row.public_key_len > 0 {
        unsafe {
            drop(Vec::from_raw_parts(
                row.public_key,
                row.public_key_len,
                row.public_key_len,
            ));
        }
    }
}

/// Release every row in a partially-built preview list and consume
/// the vec. Used by the build-loop cleanup branch.
fn free_rows(rows: Vec<IdentityKeyPreviewFFI>) {
    for row in &rows {
        // SAFETY: each row was just appended by the build loop
        // (CString::into_raw + Vec::into_raw). They have not been
        // exposed across the FFI boundary yet, so this is the
        // single, sole release.
        unsafe { release_row(row) };
    }
    drop(rows);
}
