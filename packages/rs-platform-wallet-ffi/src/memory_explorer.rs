//! FFI bindings for inspecting the in-memory state of a
//! [`PlatformWallet`](platform_wallet::PlatformWallet).
//!
//! Powers the iOS "Wallet Memory Explorer" view — a read-only dump of
//! what Rust currently holds for a loaded wallet (managed identity ids,
//! out-of-wallet/observed identity ids, gap-limit registration high
//! water mark, asset lock counts).
//!
//! Mirrors the per-wallet `info.identity_manager.*` and
//! `info.tracked_asset_locks` surface that existing FFI calls already
//! expose piecemeal — this module just gives Swift one-shot
//! enumerators so the explorer view can render a snapshot without
//! juggling multiple FFI handles.
//!
//! Token balance state is owned by
//! [`platform_wallet::IdentitySyncManager`] — query that via the
//! `platform_wallet_manager_identity_sync_state_*` family rather than
//! through this explorer.
//!
//! All entry points are read-only. Holding the wallet manager
//! `blocking_read` guard is fine on the FFI thread (matches the
//! pattern from [`crate::dashpay::platform_wallet_get_managed_identity`]).

use crate::error::*;
use crate::handle::*;
use crate::types::*;
use crate::{check_ptr, unwrap_option_or_return};
use dpp::identity::accessors::IdentityGettersV0;
use platform_wallet::wallet::identity::state::managed_identity::IdentityStatus;

/// Per-wallet snapshot returned by [`platform_wallet_get_in_memory_summary`].
#[repr(C)]
pub struct PlatformWalletMemorySummaryFFI {
    pub identities_count: usize,
    pub watched_count: usize,
    pub last_scanned_index: u32,
    pub tracked_asset_locks_count: usize,
}

/// Identity lifecycle status mirror.
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

/// List the ids of every identity the wallet currently manages.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_list_in_memory_identity_ids(
    wallet_handle: Handle,
    out_array: *mut IdentifierArray,
) -> PlatformWalletFFIResult {
    check_ptr!(out_array);
    // Sentinel first: the handle lookup and wallet-info fetch below are both
    // fallible, and `platform_wallet_identifier_array_free` reconstructs a
    // `Vec` from any non-null pointer/count pair — see
    // `IdentifierArray::empty`.
    unsafe { *out_array = IdentifierArray::empty() };

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wm = wallet.wallet_manager().blocking_read();
        let info = wm.get_wallet_info(&wallet.wallet_id())?;
        let ids: Vec<dpp::prelude::Identifier> = info
            .identity_manager
            .wallet_identities
            .values()
            .flat_map(|inner| inner.values().map(|m| m.identity.id()))
            .collect();
        Some(ids)
    });
    let inner = unwrap_option_or_return!(option);
    let ids = unwrap_option_or_return!(inner);
    unsafe { *out_array = IdentifierArray::new(ids) };
    PlatformWalletFFIResult::ok()
}

/// List the ids of every out-of-wallet / observed identity.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_list_in_memory_watched_identity_ids(
    wallet_handle: Handle,
    out_array: *mut IdentifierArray,
) -> PlatformWalletFFIResult {
    check_ptr!(out_array);
    // Sentinel first — see `IdentifierArray::empty`.
    unsafe { *out_array = IdentifierArray::empty() };

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wm = wallet.wallet_manager().blocking_read();
        let info = wm.get_wallet_info(&wallet.wallet_id())?;
        let ids: Vec<dpp::prelude::Identifier> = info
            .identity_manager
            .out_of_wallet_identities
            .keys()
            .copied()
            .collect();
        Some(ids)
    });
    let inner = unwrap_option_or_return!(option);
    let ids = unwrap_option_or_return!(inner);
    unsafe { *out_array = IdentifierArray::new(ids) };
    PlatformWalletFFIResult::ok()
}

/// Populate `out` with a snapshot of the wallet's in-memory state.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_in_memory_summary(
    wallet_handle: Handle,
    out: *mut PlatformWalletMemorySummaryFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out);

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wallet_id = wallet.wallet_id();
        let wm = wallet.wallet_manager().blocking_read();
        let info = wm.get_wallet_info(&wallet_id)?;
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
        Some(PlatformWalletMemorySummaryFFI {
            identities_count,
            watched_count,
            last_scanned_index,
            tracked_asset_locks_count,
        })
    });
    let inner = unwrap_option_or_return!(option);
    let summary = unwrap_option_or_return!(inner);
    unsafe { *out = summary };
    PlatformWalletFFIResult::ok()
}

/// Read the BIP-9 identity index recorded on a managed identity.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_identity_index(
    identity_handle: Handle,
    out_has_index: *mut bool,
    out_index: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(out_has_index);
    check_ptr!(out_index);

    let option =
        MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| identity.identity_index);
    let idx = unwrap_option_or_return!(option);
    match idx {
        Some(i) => unsafe {
            *out_has_index = true;
            *out_index = i;
        },
        None => unsafe {
            *out_has_index = false;
            *out_index = 0;
        },
    }
    PlatformWalletFFIResult::ok()
}

/// Read the lifecycle status of a managed identity.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_status(
    identity_handle: Handle,
    out_status: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_status);

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        IdentityStatusFFI::from(identity.status) as u8
    });
    *out_status = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}
