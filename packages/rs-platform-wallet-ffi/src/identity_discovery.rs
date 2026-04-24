//! FFI bindings for HD-gap-limit identity discovery on the
//! platform-wallet [`IdentityWallet`](platform_wallet::IdentityWallet).
//!
//! Exposes [`platform_wallet_discover_identities`] which drives
//! `IdentityWallet::discover`: derives consecutive MASTER
//! authentication keys from the wallet's DIP-9 tree, queries Platform
//! for a registered identity bound to each key hash (unique
//! pubkey-hash lookup), and stops after `gap_limit` consecutive
//! misses.
//!
//! Resume vs full rescan is controlled by `start_index_or_neg1`:
//!
//! - Pass `-1` (or any negative i64) to resume from the wallet's
//!   cached `last_scanned_index`.
//! - Pass `>= 0` to start scanning from that explicit identity index
//!   (typically `0` for a cold full rescan after a wallet import).
//!
//! Newly-discovered identities land in the wallet's `IdentityManager`
//! and are forwarded to Swift via the existing persister callback
//! (`on_persist_identities_fn`), so no extra SwiftData wiring is
//! required for the results to appear in the UI.

use std::ptr;

use platform_wallet::wallet::identity::network::IdentityDiscoveryOptions;

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;

/// Heap-allocated array of 32-byte identity ids returned by
/// [`platform_wallet_discover_identities`]. Release by handing the
/// entire struct back to
/// [`platform_wallet_discover_identities_free`].
#[repr(C)]
pub struct DiscoveredIdentityIdsFFI {
    /// Pointer to a contiguous `[[u8; 32]; count]` buffer. Null when
    /// `count == 0`.
    pub ids: *mut [u8; 32],
    /// Number of 32-byte identity ids in `ids`.
    pub count: usize,
}

impl DiscoveredIdentityIdsFFI {
    fn empty() -> Self {
        Self {
            ids: ptr::null_mut(),
            count: 0,
        }
    }
}

/// Discover identities registered for this wallet by scanning the
/// DIP-9 identity-authentication derivation tree and querying
/// Platform for each derived MASTER pubkey hash.
///
/// # Parameters
/// - `wallet_handle` — platform-wallet handle.
/// - `start_index_or_neg1` — `>= 0` starts from that explicit
///   identity index; `< 0` resumes from the wallet's cached
///   `last_scanned_index`.
/// - `gap_limit` — consecutive-miss threshold. Pass `0` to fall back
///   to the Rust default (`IDENTITY_GAP_LIMIT`, currently 5).
/// - `out_found` — populated on success with a heap-allocated array
///   of the newly-discovered identity ids. Release with
///   [`platform_wallet_discover_identities_free`]. On error the
///   struct is left at its empty-zero state.
/// - `out_error` — populated on failure with the usual
///   [`PlatformWalletFFIError`] detail.
///
/// # Returns
/// [`PlatformWalletFFIResult::Success`] on success (possibly with a
/// zero-length `out_found` — the scan completed but matched nothing
/// new), or an error variant.
///
/// # Safety
/// `wallet_handle` must come from the platform-wallet handle
/// registry. `out_found` must be a valid, writable pointer. All
/// other out pointers may be null.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_discover_identities(
    wallet_handle: Handle,
    start_index_or_neg1: i64,
    gap_limit: u32,
    out_found: *mut DiscoveredIdentityIdsFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_found.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_found is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // Pre-clear the out-array so partial failures don't leave the
    // caller staring at uninitialized memory.
    unsafe { *out_found = DiscoveredIdentityIdsFFI::empty() };

    let opts = IdentityDiscoveryOptions {
        start_index: if start_index_or_neg1 < 0 {
            None
        } else {
            // Clamp to u32 — callers passing something beyond
            // u32::MAX are already in pathological territory.
            Some(start_index_or_neg1.min(u32::MAX as i64) as u32)
        },
        gap_limit: if gap_limit == 0 {
            IdentityDiscoveryOptions::default().gap_limit
        } else {
            gap_limit
        },
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result = block_on_worker(async move { identity.discover(opts).await });
            match result {
                Ok(found) => {
                    if found.is_empty() {
                        // `out_found` is already the empty sentinel.
                        return PlatformWalletFFIResult::Success;
                    }

                    // Serialize discovered identity ids into a
                    // heap-allocated `[[u8; 32]]` buffer paired with
                    // its length, then hand ownership back to the
                    // caller via `out_found`.
                    use dpp::identity::accessors::IdentityGettersV0;

                    let ids: Vec<[u8; 32]> = found.iter().map(|i| *i.id().as_bytes()).collect();
                    let mut boxed = ids.into_boxed_slice();
                    let count = boxed.len();
                    let ptr = boxed.as_mut_ptr();
                    std::mem::forget(boxed);

                    unsafe {
                        *out_found = DiscoveredIdentityIdsFFI { ids: ptr, count };
                    }
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("discover_identities failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Release a [`DiscoveredIdentityIdsFFI`] previously populated by
/// [`platform_wallet_discover_identities`]. Safe to call on a
/// zero/null struct (no-op).
///
/// # Safety
/// `ids` must have been handed out by
/// [`platform_wallet_discover_identities`] and must not be freed
/// twice.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_discover_identities_free(found: DiscoveredIdentityIdsFFI) {
    if found.ids.is_null() || found.count == 0 {
        return;
    }
    unsafe {
        let slice = std::slice::from_raw_parts_mut(found.ids, found.count);
        drop(Box::from_raw(slice as *mut [[u8; 32]]));
    }
}
