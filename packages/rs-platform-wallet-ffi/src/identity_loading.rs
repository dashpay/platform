//! FFI bindings for loading a single identity at a known BIP-9 HD
//! identity index on the platform-wallet
//! [`IdentityWallet`](platform_wallet::IdentityWallet).
//!
//! Exposes [`platform_wallet_load_identity_at_index`] which drives
//! `IdentityWallet::load_identity_by_index` (or
//! `load_identity_by_index_from_master`): derives the MASTER
//! authentication key from the wallet's DIP-9 tree at the supplied
//! identity index, queries Platform for a registered identity bound to
//! that key hash (unique pubkey-hash lookup), and — if found — folds it
//! into the wallet's `IdentityManager`.
//!
//! Unlike [`crate::identity_discovery::platform_wallet_discover_identities`],
//! this probes exactly ONE identity index rather than gap-limit
//! scanning a range. The two share the same DIP-9 MASTER slot
//! (`MASTER_KEY_INDEX`), so a load and a discovery scan resolve the same
//! identity at the same index.
//!
//! # Key source: chosen by wallet capability
//!
//! The DIP-9 derivation needs the wallet's private key material. The
//! source is selected by the in-process wallet's shape, NOT by whether
//! a resolver handle was supplied — the resolver is a *capability* the
//! Rust side consults only when it can't derive locally, not a command
//! that forces the resolver path (same dispatch as
//! [`crate::identity_discovery::platform_wallet_discover_identities`]):
//!
//! - **In-process wallet holds resident private keys** (`WalletType::
//!   Mnemonic` / `Seed` / `ExtendedPrivKey` — NOT external-signable and
//!   NOT watch-only): drive the historical resident-wallet derive
//!   (`load_identity_by_index`). The resolver handle is never touched,
//!   which also skips a pointless iOS Keychain read. This keeps
//!   `createWallet(seed:)` / raw-seed wallets working even when no
//!   BIP-39 mnemonic was ever persisted to `WalletStorage`.
//! - **In-process wallet is external-signable / watch-only:** its seed
//!   lives in iOS Keychain, NOT in process, so the resident derive would
//!   fail with `External signable wallet has no private key`. In that
//!   case, if `mnemonic_resolver_handle` is non-null, resolve the
//!   wallet's mnemonic on demand via the Swift-owned
//!   [`MnemonicResolverHandle`] (its `resolve` callback reads the
//!   mnemonic from iOS Keychain keyed by the wallet handle's own
//!   `wallet_id`), build the master `ExtendedPrivKey`, and drive
//!   `load_identity_by_index_from_master`. The mnemonic / seed / master
//!   scalar all live in `Zeroizing` buffers (the master's `private_key`
//!   is explicitly `non_secure_erase`d — `ExtendedPrivKey` has no
//!   `Drop`) and are scrubbed before this function returns. This is the
//!   path the iOS app takes. If the resolver is null for such a wallet,
//!   the call returns an error hinting that a mnemonic resolver handle
//!   is required for this wallet shape.
//!
//! A discovered identity lands in the wallet's `IdentityManager` and is
//! forwarded to Swift via the existing persister callback
//! (`on_persist_identities_fn`), so no extra SwiftData wiring is
//! required for the result to appear in the UI; the 32-byte id is also
//! handed back through `out_identity_id` for the caller's convenience.

use dpp::identity::accessors::IdentityGettersV0;

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
use crate::runtime::block_on_worker;
use crate::types::Network;
use crate::{unwrap_option_or_return, unwrap_result_or_return};
use rs_sdk_ffi::MnemonicResolverHandle;

/// Load the identity registered for this wallet at a single BIP-9 HD
/// identity index by deriving the DIP-9 MASTER authentication key at
/// that index and querying Platform for its pubkey hash.
///
/// The derivation source is chosen by the in-process wallet's
/// capability (see the module docs): resident-key wallets probe via the
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
///   own `wallet_id`), a master `ExtendedPrivKey` is built, and the
///   probe hash is derived from that master; when null the call errors
///   with a hint that a resolver handle is required for this wallet
///   shape. For a wallet that holds resident private keys this argument
///   is ignored and the probe derives from the in-process wallet (the
///   historical path).
/// - `identity_index` — BIP-9 identity index to probe.
/// - `out_found` — set to `true` iff an identity was registered at this
///   index, `false` otherwise. Always written on success.
/// - `out_identity_id` — 32-byte buffer; written with the discovered
///   identity's id ONLY when `*out_found` is `true`. Left untouched
///   when no identity is found.
///
/// # Safety
/// `wallet_handle` must come from the platform-wallet handle registry.
/// `mnemonic_resolver_handle`, when non-null, must come from
/// [`rs_sdk_ffi::dash_sdk_mnemonic_resolver_create`] and remain valid
/// for the duration of the call. `out_found` must be a valid, writable
/// pointer. `out_identity_id` must be a valid, writable pointer to at
/// least 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_load_identity_at_index(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    identity_index: u32,
    out_found: *mut bool,
    out_identity_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_found);
    check_ptr!(out_identity_id);

    // Pre-clear the found flag so a partial failure doesn't leave the
    // caller staring at uninitialized memory. The 32-byte id buffer is
    // only written when `*out_found` is set, so callers must gate their
    // read on the flag.
    unsafe { *out_found = false };

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();

        // Select the derivation source by the in-process wallet's
        // capability (see the module docs), NOT by whether a resolver
        // was supplied. Read the wallet's shape under a short read-lock
        // and DROP the guard before `block_on_worker` — the load future
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
            return block_on_worker(async move {
                identity.load_identity_by_index(identity_index).await
            })
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
                 watch-only); a mnemonic resolver handle is required to load \
                 its identity",
            ));
        }

        // Resolver path: resolve the wallet's mnemonic → build master
        // xpriv → drive `load_identity_by_index_from_master`. Self-pin on
        // the wallet handle's own `wallet_id` (same rationale as
        // `platform_wallet_discover_identities`): a separate wallet_id
        // param would let a caller derive from one wallet's mnemonic
        // while loading against a different wallet's handle.
        let wallet_id = wallet.wallet_id();
        // `wallet.network()` returns `dashcore::Network`, which is the
        // same type `ExtendedPrivKey::new_master` and the load derive
        // use.
        let network: Network = wallet.network();

        // SAFETY: `mnemonic_resolver_handle` is non-null (checked above)
        // and the caller's safety contract guarantees it came from
        // `dash_sdk_mnemonic_resolver_create` and is valid for this call.
        // Resolves the mnemonic + builds the master in the same shared
        // helper the discovery and preview paths use so they can't drift;
        // the helper holds the mnemonic / seed in `Zeroizing` buffers and
        // scrubs them before returning. The master's inner scalar is
        // wiped by us below (`ExtendedPrivKey` has no `Drop`).
        let master = match unsafe {
            resolve_master_from_resolver(mnemonic_resolver_handle, &wallet_id, network)
        } {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        // Run the load against the resolved master. The master is MOVED
        // into the spawned future: `block_on_worker` polls on a worker
        // thread (the `'static` bound forbids borrowing our stack
        // `master`), so we hand ownership in and wipe it INSIDE the
        // future once `load_identity_by_index_from_master` is done
        // deriving.
        //
        // `ExtendedPrivKey` has no `Drop` / `Zeroize`, so the inner
        // secp256k1 scalar is scrubbed explicitly with
        // `non_secure_erase` — same hygiene as
        // `platform_wallet_discover_identities`. (`seed` / `mnemonic_buf`
        // are `Zeroizing` and already scrubbed inside
        // `resolve_master_from_resolver`.)
        block_on_worker(async move {
            let mut master = master;
            let load_result = identity
                .load_identity_by_index_from_master(identity_index, &master)
                .await;
            master.private_key.non_secure_erase();
            load_result
        })
        .map_err(PlatformWalletFFIResult::from)
    });
    let result = unwrap_option_or_return!(option);
    let found = unwrap_result_or_return!(result);

    match found {
        Some(identity) => {
            let id_bytes = *identity.id().as_bytes();
            unsafe {
                std::ptr::copy_nonoverlapping(id_bytes.as_ptr(), out_identity_id, 32);
                *out_found = true;
            }
            PlatformWalletFFIResult::ok()
        }
        None => {
            // `*out_found` is already `false` from the pre-clear above.
            PlatformWalletFFIResult::ok()
        }
    }
}
