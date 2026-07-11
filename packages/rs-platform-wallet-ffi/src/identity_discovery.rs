//! FFI bindings for HD-gap-limit identity discovery on the
//! platform-wallet [`IdentityWallet`](platform_wallet::IdentityWallet).
//!
//! Exposes [`platform_wallet_discover_identities`] which drives
//! `IdentityWallet::discover` (or `discover_from_master`): derives
//! consecutive MASTER authentication keys from the wallet's DIP-9 tree,
//! queries Platform for a registered identity bound to each key hash
//! (unique pubkey-hash lookup), and stops after `gap_limit` consecutive
//! misses.
//!
//! Resume vs full rescan is controlled by `start_index_or_neg1`:
//!
//! - Pass `-1` (or any negative i64) to resume from the wallet's
//!   cached `last_scanned_index`.
//! - Pass `>= 0` to start scanning from that explicit identity index
//!   (typically `0` for a cold full rescan after a wallet import).
//!
//! # Key source: chosen by wallet capability
//!
//! The DIP-9 derivation needs the wallet's private key material. The
//! source is selected by the in-process wallet's shape, NOT by whether
//! a resolver handle was supplied — the resolver is a *capability* the
//! Rust side consults only when it can't derive locally, not a command
//! that forces the resolver path:
//!
//! - **In-process wallet holds resident private keys** (`WalletType::
//!   Mnemonic` / `Seed` / `ExtendedPrivKey` — NOT external-signable and
//!   NOT watch-only): drive the historical resident-wallet derive
//!   (`discover`). The resolver handle is never touched, which also
//!   skips a pointless iOS Keychain read. This keeps `createWallet(seed:)`
//!   / raw-seed wallets working even when no BIP-39 mnemonic was ever
//!   persisted to `WalletStorage`.
//! - **In-process wallet is external-signable / watch-only:** its seed
//!   lives in iOS Keychain, NOT in process, so the resident derive would
//!   fail with `External signable wallet has no private key`. In that
//!   case, if `mnemonic_resolver_handle` is non-null, resolve the
//!   wallet's mnemonic on demand via the Swift-owned
//!   [`MnemonicResolverHandle`] (its `resolve` callback reads the
//!   mnemonic from iOS Keychain keyed by the wallet handle's own
//!   `wallet_id`), build the master `ExtendedPrivKey`, and drive
//!   `discover_from_master`. The mnemonic / seed / master scalar all
//!   live in `Zeroizing` buffers (the master's `private_key` is
//!   explicitly `non_secure_erase`d — `ExtendedPrivKey` has no `Drop`)
//!   and are scrubbed before this function returns. This is the path
//!   the iOS app takes. If the resolver is null for such a wallet, the
//!   call returns an error hinting that a mnemonic resolver handle is
//!   required for this wallet shape.
//!
//! Newly-discovered identities land in the wallet's `IdentityManager`
//! and are forwarded to Swift via the existing persister callback
//! (`on_persist_identities_fn`), so no extra SwiftData wiring is
//! required for the results to appear in the UI.

use std::ptr;

use platform_wallet::wallet::identity::network::IdentityDiscoveryOptions;

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
use crate::runtime::block_on_worker;
use crate::types::Network;
use crate::{unwrap_option_or_return, unwrap_result_or_return};
use rs_sdk_ffi::MnemonicResolverHandle;

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
/// The derivation source is chosen by the in-process wallet's
/// capability (see the module docs): resident-key wallets scan via the
/// in-process derive and never touch the resolver; external-signable /
/// watch-only wallets consult the resolver. The resolver is only
/// *needed* for the latter shape.
///
/// # Parameters
/// - `wallet_handle` — platform-wallet handle.
/// - `mnemonic_resolver_handle` — Swift-owned
///   [`MnemonicResolverHandle`], consulted **only** when the in-process
///   wallet lacks resident private keys (external-signable / watch-only
///   — the iOS Keychain-backed `WalletType::ExternalSignable` shape
///   whose seed is not in process). For such a wallet, when non-null
///   the mnemonic is resolved on demand (keyed by the wallet handle's
///   own `wallet_id`), a master `ExtendedPrivKey` is built, and the scan
///   derives each probe hash from that master; when null the call errors
///   with a hint that a resolver handle is required for this wallet
///   shape. For a wallet that holds resident private keys this argument
///   is ignored and the scan derives from the in-process wallet (the
///   historical path).
/// - `start_index_or_neg1` — `>= 0` starts from that explicit
///   identity index; `< 0` resumes from the wallet's cached
///   `last_scanned_index`.
/// - `gap_limit` — consecutive-miss threshold. Pass `0` to fall back
///   to the Rust default (`IDENTITY_GAP_LIMIT`, currently 5).
/// - `out_found` — populated on success with a heap-allocated array
///   of the newly-discovered identity ids. Release with
///   [`platform_wallet_discover_identities_free`]. On error the
///   struct is left at its empty-zero state.
///   [`PlatformWalletFFIError`] detail.
///
/// # Safety
/// `wallet_handle` must come from the platform-wallet handle
/// registry. `mnemonic_resolver_handle`, when non-null, must come
/// from [`rs_sdk_ffi::dash_sdk_mnemonic_resolver_create`] and remain
/// valid for the duration of the call. `out_found` must be a valid,
/// writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_discover_identities(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    start_index_or_neg1: i64,
    gap_limit: u32,
    out_found: *mut DiscoveredIdentityIdsFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_found);

    // Pre-clear the out-array so partial failures don't leave the
    // caller staring at uninitialized memory.
    unsafe { *out_found = DiscoveredIdentityIdsFFI::empty() };

    let opts = IdentityDiscoveryOptions {
        start_index: if start_index_or_neg1 < 0 {
            None
        } else {
            Some(start_index_or_neg1.min(u32::MAX as i64) as u32)
        },
        gap_limit: if gap_limit == 0 {
            IdentityDiscoveryOptions::default().gap_limit
        } else {
            gap_limit
        },
    };

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();

        // Select the derivation source by the in-process wallet's
        // capability (see the module docs), NOT by whether a resolver
        // was supplied. Read the wallet's shape under a short read-lock
        // and DROP the guard before `block_on_worker` — the scan future
        // is `Send + 'static`, so the guard must not be held across it.
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

        if wallet_has_resident_keys {
            // Resident private keys (Mnemonic / Seed / ExtendedPrivKey) →
            // historical in-process derive. The resolver is never touched
            // (also skips a pointless iOS Keychain read), so raw-seed /
            // mnemonic wallets keep working even when no mnemonic was ever
            // persisted to `WalletStorage`.
            return block_on_worker(async move { identity.discover(opts).await })
                .map_err(PlatformWalletFFIResult::from);
        }

        // External-signable / watch-only wallet: its seed lives in iOS
        // Keychain, not in process, so the resident derive would fail with
        // `External signable wallet has no private key`. A resolver is
        // required here.
        if mnemonic_resolver_handle.is_null() {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "this wallet has no resident private keys (external-signable / \
                 watch-only); a mnemonic resolver handle is required to scan for \
                 its identities",
            ));
        }

        // Resolver path: resolve the wallet's mnemonic → build master
        // xpriv → drive `discover_from_master`. Self-pin on the wallet
        // handle's own `wallet_id` (same rationale as
        // `dash_sdk_derive_identity_key_at_slot_with_resolver`): a
        // separate wallet_id param would let a caller derive from one
        // wallet's mnemonic while scanning a different wallet's handle.
        let wallet_id = wallet.wallet_id();
        // `wallet.network()` returns `dashcore::Network`, which is the
        // same type `ExtendedPrivKey::new_master` and the discovery scan
        // derive with.
        let network: Network = wallet.network();

        // SAFETY: `mnemonic_resolver_handle` is non-null (checked above)
        // and the caller's safety contract guarantees it came from
        // `dash_sdk_mnemonic_resolver_create` and is valid for this call.
        // Resolves the mnemonic + builds the master in one shared helper
        // so the discovery and preview paths can't drift; the helper
        // holds the mnemonic / seed in `Zeroizing` buffers and scrubs
        // them before returning. The master's inner scalar is wiped by
        // us below (`ExtendedPrivKey` has no `Drop`).
        let master = match unsafe {
            resolve_master_from_resolver(mnemonic_resolver_handle, &wallet_id, network)
        } {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        // Run the scan against the resolved master. The master is MOVED
        // into the spawned future: `block_on_worker` polls on a worker
        // thread (the `'static` bound forbids borrowing our stack
        // `master`), so we hand ownership in and wipe it INSIDE the
        // future once `discover_from_master` is done deriving.
        //
        // `ExtendedPrivKey` has no `Drop` / `Zeroize`, so the inner
        // secp256k1 scalar is scrubbed explicitly with
        // `non_secure_erase` — same hygiene as
        // `dash_sdk_sign_with_mnemonic_resolver_and_path` and
        // `mnemonic_resolver_core_signer`. (`seed` / `mnemonic_buf` are
        // `Zeroizing` and already scrubbed inside
        // `resolve_master_from_resolver`.)
        block_on_worker(async move {
            let mut master = master;
            let scan_result = identity.discover_from_master(opts, &master).await;
            master.private_key.non_secure_erase();
            scan_result
        })
        .map_err(PlatformWalletFFIResult::from)
    });
    let result = unwrap_option_or_return!(option);
    let found = unwrap_result_or_return!(result);
    if found.is_empty() {
        return PlatformWalletFFIResult::ok();
    }

    use dpp::identity::accessors::IdentityGettersV0;
    let ids: Vec<[u8; 32]> = found.iter().map(|i| *i.id().as_bytes()).collect();
    let mut boxed = ids.into_boxed_slice();
    let count = boxed.len();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    unsafe {
        *out_found = DiscoveredIdentityIdsFFI { ids: ptr, count };
    }
    PlatformWalletFFIResult::ok()
}

/// Release a [`DiscoveredIdentityIdsFFI`] previously populated by
/// [`platform_wallet_discover_identities`]. Safe to call on a
/// zero/null struct or a null outer pointer (no-op).
///
/// Pointer-only signature: `DiscoveredIdentityIdsFFI` is a 16-byte
/// aggregate at the AAPCS64 / Swift cliff, so by-value isn't safe
/// across `@_silgen_name`. Caller hands ownership back via
/// `&mut found`; on return the buffer is freed and the fields are
/// reset so a double-free no-ops.
///
/// # Safety
/// `ids` must have been handed out by
/// [`platform_wallet_discover_identities`] and must not be freed
/// twice.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_discover_identities_free(
    found: *mut DiscoveredIdentityIdsFFI,
) {
    if found.is_null() {
        return;
    }
    let found = unsafe { &mut *found };
    if found.ids.is_null() || found.count == 0 {
        return;
    }
    unsafe {
        let slice = std::slice::from_raw_parts_mut(found.ids, found.count);
        drop(Box::from_raw(slice as *mut [[u8; 32]]));
    }
    found.ids = std::ptr::null_mut();
    found.count = 0;
}
