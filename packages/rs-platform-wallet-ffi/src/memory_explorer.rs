//! FFI bindings for inspecting the in-memory state of a
//! [`PlatformWallet`](platform_wallet::PlatformWallet).
//!
//! Powers the iOS "Wallet Memory Explorer" view — a read-only dump of
//! what Rust currently holds for a loaded wallet (managed identity ids,
//! out-of-wallet/observed identity ids, gap-limit registration high
//! water mark, asset lock and token-balance counts).
//!
//! Mirrors the per-wallet `info.identity_manager.*` and
//! `info.tracked_asset_locks` / `info.token_balances` surface that
//! existing FFI calls already expose piecemeal — this module just
//! gives Swift one-shot enumerators so the explorer view can render
//! a snapshot without juggling multiple FFI handles.
//!
//! All entry points are read-only. Holding the wallet manager
//! `blocking_read` guard is fine on the FFI thread (matches the
//! pattern from [`crate::dashpay::platform_wallet_get_managed_identity`]).

use crate::error::*;
use crate::handle::*;
use crate::types::*;
use dpp::identity::accessors::IdentityGettersV0;
use platform_wallet::wallet::identity::state::managed_identity::IdentityStatus;

/// Per-wallet snapshot returned by
/// [`platform_wallet_get_in_memory_summary`].
///
/// Caller-owned; written into a slot the caller already allocates so
/// no `_free` is required.
#[repr(C)]
pub struct PlatformWalletMemorySummaryFFI {
    /// Number of identities the wallet manages (signing-capable; lives
    /// in the wallet bucket).
    pub identities_count: usize,
    /// Number of out-of-wallet (read-only / observed) identities.
    pub watched_count: usize,
    /// One past the wallet's highest already-registered identity index,
    /// matching the resume position the gap-limit scanner uses next.
    /// Zero when the wallet has no managed identities yet.
    pub last_scanned_index: u32,
    /// Number of tracked asset locks the wallet currently holds in
    /// memory (`PlatformWalletInfo.tracked_asset_locks`).
    pub tracked_asset_locks_count: usize,
    /// Number of `(identity_id, token_id) -> amount` entries on the
    /// wallet (`PlatformWalletInfo.token_balances`).
    pub token_balances_count: usize,
}

/// Identity lifecycle status mirror.
///
/// Maps directly to `platform_wallet::wallet::identity::state::
/// managed_identity::IdentityStatus`. Exposed as a `#[repr(u8)]`
/// enum so Swift can read it as a plain integer return value.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityStatusFFI {
    Unknown = 0,
    PendingCreation = 1,
    Active = 2,
    FailedCreation = 3,
    NotFound = 4,
}

impl From<IdentityStatus> for IdentityStatusFFI {
    fn from(s: IdentityStatus) -> Self {
        match s {
            IdentityStatus::Unknown => IdentityStatusFFI::Unknown,
            IdentityStatus::PendingCreation => IdentityStatusFFI::PendingCreation,
            IdentityStatus::Active => IdentityStatusFFI::Active,
            IdentityStatus::FailedCreation => IdentityStatusFFI::FailedCreation,
            IdentityStatus::NotFound => IdentityStatusFFI::NotFound,
        }
    }
}

/// List the ids of every identity the wallet currently manages
/// (signing-capable identities in the wallet bucket).
///
/// Iterates `info.identity_manager.wallet_identities` values via
/// [`IdentifierArray`]. Release with
/// [`crate::platform_wallet_identifier_array_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_list_in_memory_identity_ids(
    wallet_handle: Handle,
    out_array: *mut IdentifierArray,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_array.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_array is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let wm = wallet.wallet_manager().blocking_read();
            let info = match wm.get_wallet_info(&wallet.wallet_id()) {
                Some(i) => i,
                None => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorInvalidHandle,
                                "Wallet info not found for wallet handle",
                            );
                        }
                    }
                    return PlatformWalletFFIResult::ErrorInvalidHandle;
                }
            };
            let ids: Vec<dpp::prelude::Identifier> = info
                .identity_manager
                .wallet_identities
                .values()
                .flat_map(|inner| inner.values().map(|m| m.identity.id()))
                .collect();
            unsafe { *out_array = IdentifierArray::new(ids) };
            PlatformWalletFFIResult::Success
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

/// List the ids of every out-of-wallet / observed identity the wallet
/// currently holds.
///
/// Returns the keys of `info.identity_manager.out_of_wallet_identities`
/// via [`IdentifierArray`]. Release with
/// [`crate::platform_wallet_identifier_array_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_list_in_memory_watched_identity_ids(
    wallet_handle: Handle,
    out_array: *mut IdentifierArray,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_array.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_array is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let wm = wallet.wallet_manager().blocking_read();
            let info = match wm.get_wallet_info(&wallet.wallet_id()) {
                Some(i) => i,
                None => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorInvalidHandle,
                                "Wallet info not found for wallet handle",
                            );
                        }
                    }
                    return PlatformWalletFFIResult::ErrorInvalidHandle;
                }
            };
            let ids: Vec<dpp::prelude::Identifier> = info
                .identity_manager
                .out_of_wallet_identities
                .keys()
                .copied()
                .collect();
            unsafe { *out_array = IdentifierArray::new(ids) };
            PlatformWalletFFIResult::Success
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

/// Populate `out` with a snapshot of the wallet's in-memory state.
///
/// Caller-owned out-pointer (struct is > 16 bytes — keeps us inside
/// the AAPCS64 / Swift-ABI safe lane). On error the slot is left
/// untouched.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_in_memory_summary(
    wallet_handle: Handle,
    out: *mut PlatformWalletMemorySummaryFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let wallet_id = wallet.wallet_id();
            let wm = wallet.wallet_manager().blocking_read();
            let info = match wm.get_wallet_info(&wallet_id) {
                Some(i) => i,
                None => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorInvalidHandle,
                                "Wallet info not found for wallet handle",
                            );
                        }
                    }
                    return PlatformWalletFFIResult::ErrorInvalidHandle;
                }
            };
            // `identities_count` follows the new bucket layout —
            // sum the inner-map lengths under the wallet bucket.
            let identities_count: usize = info
                .identity_manager
                .wallet_identities
                .values()
                .map(|m| m.len())
                .sum();
            let watched_count = info.identity_manager.out_of_wallet_identities.len();
            // Resume position the gap-limit scanner would use next —
            // one past the highest already-registered slot for this
            // wallet, or 0 when nothing has been registered yet.
            let last_scanned_index = info
                .identity_manager
                .highest_registration_index(&wallet_id)
                .map_or(0u32, |i| i + 1);
            let tracked_asset_locks_count = info.tracked_asset_locks.len();
            let token_balances_count = info.token_balances.len();

            unsafe {
                *out = PlatformWalletMemorySummaryFFI {
                    identities_count,
                    watched_count,
                    last_scanned_index,
                    tracked_asset_locks_count,
                    token_balances_count,
                };
            }
            PlatformWalletFFIResult::Success
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

/// Read the BIP-9 identity index recorded on a managed identity.
///
/// Mirrors `ManagedIdentity.identity_index` — the position in
/// `m/9'/coin'/5'/0'/key_type'/identity_index'/key_id'` used to
/// derive this identity's keys. Useful for the explorer view to
/// surface "which HD slot owns this id".
///
/// Out-of-wallet (observed) identities have no derivation context:
/// when the underlying `Option<u32>` is `None`, this writes
/// `*out_has_index = false` and `*out_index = 0` and still returns
/// `Success` — both states are valid, not errors.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_identity_index(
    identity_handle: Handle,
    out_has_index: *mut bool,
    out_index: *mut u32,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_has_index.is_null() || out_index.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_has_index or out_index is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            match identity.identity_index {
                Some(idx) => unsafe {
                    *out_has_index = true;
                    *out_index = idx;
                },
                None => unsafe {
                    *out_has_index = false;
                    *out_index = 0;
                },
            }
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Read the lifecycle status of a managed identity.
///
/// `out_status` receives an [`IdentityStatusFFI`] discriminant. The
/// `Default` for the underlying enum is `Unknown`, so the value is
/// always meaningful even for freshly-created identities.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_status(
    identity_handle: Handle,
    out_status: *mut u8,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_status.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_status is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            let status: IdentityStatusFFI = identity.status.into();
            unsafe { *out_status = status as u8 };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}
