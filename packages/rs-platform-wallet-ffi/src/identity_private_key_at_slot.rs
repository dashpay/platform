//! Wallet-handle-driven identity private-key derivation for a single
//! persisted key slot.
//!
//! Exposes [`platform_wallet_derive_identity_private_key_at_slot`]: given
//! a platform-wallet handle and the persisted derivation breadcrumb
//! `(identity_index, key_index)` that
//! [`crate::identity_persistence::IdentityKeyEntryFFI`] hands the client,
//! this derives the ready-to-persist 32-byte ECDSA private scalar and
//! its DIP-9 derivation path string, returning them in a paired-free
//! [`IdentityPrivateKeyFFI`] buffer.
//!
//! # Why this exists — the CLAUDE.md "one allowed exception" shape
//!
//! The persistence callback bridge hands the client each identity key as
//! an [`IdentityKeyEntryFFI`], which carries **only** the derivation
//! breadcrumb (`wallet_id`, `identity_index`, `key_index`) — never the
//! private half. The client is expected to re-derive the 32-byte scalar
//! locally and stash it in the platform's secure key store (iOS Keychain
//! / Android Keystore-wrapped storage), then record the store identifier
//! on the persisted row so the signer can re-read it during
//! state-transition signing.
//!
//! The naive way to do that — fetch the mnemonic from the store, run
//! `mnemonic → seed → path → key` on the client side, write the result
//! back — is exactly the pipeline the SDK doctrine forbids on the client
//! (`packages/swift-sdk/CLAUDE.md` "The one allowed exception: Keychain",
//! and `packages/kotlin-sdk/CLAUDE.md` "Forbidden in Kotlin"). The
//! doctrine's remedy is a single FFI entry point that performs the whole
//! derivation on the Rust side and returns bytes ready to persist. This
//! is that entry point — the single-key analogue of
//! [`crate::identity_key_preview::platform_wallet_preview_identity_registration_keys`].
//!
//! # Key source: chosen by wallet capability
//!
//! Like the preview helper, the derivation source is selected by the
//! in-process wallet's shape, NOT by whether a resolver handle was
//! supplied:
//!
//! - **Resident-key wallet** (`WalletType::Mnemonic` / `Seed` /
//!   `ExtendedPrivKey` — not external-signable, not watch-only): derive
//!   from the in-process wallet under a read lock. The resolver handle is
//!   never touched.
//! - **External-signable / watch-only wallet:** the seed is not in
//!   process, so the mnemonic is resolved on demand via the client-owned
//!   [`MnemonicResolverHandle`] (keyed by the wallet handle's own
//!   `wallet_id`), the master `ExtendedPrivKey` is built, and the key is
//!   derived from it. If the resolver is null for such a wallet the call
//!   errors with a hint that a resolver handle is required.
//!
//! All sensitive intermediates live in `Zeroizing` buffers; the resolved
//! master's scalar is explicitly `non_secure_erase`d before return as
//! belt-and-braces (the pinned key-wallet rev zeroizes `ExtendedPrivKey`
//! on `Drop`; the explicit erase is robust to an upstream regression).

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use key_wallet::bip32::ExtendedPrivKey;
use key_wallet::Wallet;
use platform_wallet::derive_identity_auth_keypair;
use platform_wallet::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master;
use zeroize::Zeroizing;

use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
use crate::types::Network;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use rs_sdk_ffi::MnemonicResolverHandle;

/// The derived private half of a single identity key, ready for the
/// client to persist into its secure key store.
///
/// `private_key_bytes` is the raw 32-byte ECDSA scalar — treat as
/// sensitive material: copy it straight into the platform key store and
/// release this buffer immediately via
/// [`platform_wallet_derive_identity_private_key_at_slot_free`], which
/// zeroizes the scalar in place.
///
/// `derivation_path` is the null-terminated DIP-9 path string
/// (`m/9'/coin'/5'/0'/0'/identity_index'/key_index'`) the client uses as
/// the store account identifier so the key can be located and rendered
/// later. Heap-allocated — reclaimed by the paired free.
#[repr(C)]
pub struct IdentityPrivateKeyFFI {
    /// Identity index the key was derived at (echoed back for the
    /// caller's convenience — matches the requested `identity_index`).
    pub identity_index: u32,
    /// Key index the key was derived at (echoed back — matches the
    /// requested `key_index`).
    pub key_index: u32,
    /// Raw 32-byte ECDSA private-key scalar. Inline — no heap
    /// allocation. Zeroized in place by the paired free.
    pub private_key_bytes: [u8; 32],
    /// Null-terminated UTF-8 DIP-9 derivation path string. Heap-
    /// allocated; reclaimed by the paired free.
    pub derivation_path: *mut c_char,
}

impl IdentityPrivateKeyFFI {
    /// All-null / zero row so a failed FFI call leaves the caller
    /// staring at known-empty state instead of uninitialized memory.
    pub fn empty() -> Self {
        Self {
            identity_index: 0,
            key_index: 0,
            private_key_bytes: [0u8; 32],
            derivation_path: ptr::null_mut(),
        }
    }
}

/// Derive the 32-byte ECDSA identity-authentication private key at
/// `(identity_index, key_index)` for the wallet behind `wallet_handle`,
/// ready for the client to persist into its secure key store.
///
/// The derivation source (resident wallet vs. resolver-provided
/// mnemonic) is chosen by the wallet's capability — see the module docs.
/// The network is read from the wallet handle, so the caller decides
/// nothing about coin type or path shape.
///
/// # Parameters
/// - `wallet_handle` — platform-wallet handle.
/// - `mnemonic_resolver_handle` — client-owned [`MnemonicResolverHandle`],
///   consulted **only** when the in-process wallet lacks resident private
///   keys. May be null for resident-key wallets.
/// - `identity_index` / `key_index` — the persisted derivation
///   breadcrumb from the [`IdentityKeyEntryFFI`] row.
/// - `out_key` — populated on success. Release with
///   [`platform_wallet_derive_identity_private_key_at_slot_free`]. On
///   error the struct is left at the empty zero state.
///
/// # Safety
/// `wallet_handle` must come from the platform-wallet handle registry.
/// `mnemonic_resolver_handle`, when non-null, must come from
/// [`rs_sdk_ffi::dash_sdk_mnemonic_resolver_create`] and remain valid for
/// the call. `out_key` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_derive_identity_private_key_at_slot(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    identity_index: u32,
    key_index: u32,
    out_key: *mut IdentityPrivateKeyFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_key);
    unsafe { *out_key = IdentityPrivateKeyFFI::empty() };

    // Per-slot derived material funnelled from either key source so the
    // row-building runs exactly once.
    struct SlotMaterial {
        path: String,
        private_key: Zeroizing<[u8; 32]>,
    }

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let network: Network = wallet.network();

        // Two-phase locking, mirroring the preview / discovery paths:
        //   1. A SHORT read-guard block scoped to the capability check
        //      only — read the wallet's shape, then DROP the guard.
        //   2. The wallet-manager read guard is NEVER held across the
        //      resolver callback (which re-enters the client and reads
        //      the secure store, potentially stalling on biometric
        //      unlock).
        //   3. Only the resident branch re-acquires the guard, for the
        //      single derive.
        (|| -> Result<SlotMaterial, PlatformWalletFFIResult> {
            // Phase 1 — short capability-check guard.
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

            // Resolver path: resolve the mnemonic once and build the
            // master xpriv up front (NO guard held). Self-pin on the
            // wallet handle's own `wallet_id`.
            let mut master_opt: Option<ExtendedPrivKey> = None;
            if !wallet_has_resident_keys {
                if mnemonic_resolver_handle.is_null() {
                    return Err(PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorWalletOperation,
                        "this wallet has no resident private keys (external-signable / \
                         watch-only); a mnemonic resolver handle is required to derive \
                         its identity private key",
                    ));
                }
                let wallet_id = wallet.wallet_id();
                // SAFETY: handle is non-null (checked) and the caller's
                // safety contract guarantees provenance.
                master_opt = Some(unsafe {
                    resolve_master_from_resolver(mnemonic_resolver_handle, &wallet_id, network)?
                });
            }

            // Phase 3 — derive from the borrowed key source inside a
            // block so the resident-path guard (and the master borrow)
            // release BEFORE we wipe the resolved master's scalar below.
            let derive_result = {
                let mut loop_guard = None;
                let material = match master_opt.as_ref() {
                    Some(master) => {
                        let derived = derive_ecdsa_identity_auth_keypair_from_master(
                            master,
                            network,
                            identity_index,
                            key_index,
                        );
                        derived.map(|d| SlotMaterial {
                            path: d.derivation_path.to_string(),
                            private_key: d.private_key,
                        })
                    }
                    None => {
                        let wm = loop_guard.insert(wallet.wallet_manager().blocking_read());
                        let key_wallet: &Wallet = match wm.get_wallet(&wallet.wallet_id()) {
                            Some(w) => w,
                            None => {
                                return Err(PlatformWalletFFIResult::err(
                                    PlatformWalletFFIResultCode::ErrorInvalidHandle,
                                    "Wallet not found in wallet manager",
                                ));
                            }
                        };
                        derive_identity_auth_keypair(key_wallet, network, identity_index, key_index)
                            .map(|(path, mut ext_priv, _public_key)| {
                                let private_key =
                                    Zeroizing::new(ext_priv.private_key.secret_bytes());
                                // Belt-and-braces: the pinned key-wallet
                                // rev zeroizes `ExtendedPrivKey` on Drop,
                                // but erase the resident-path intermediate
                                // explicitly anyway, matching the
                                // resolver-branch erase below (cheap, and
                                // robust to an upstream Drop regression).
                                ext_priv.private_key.non_secure_erase();
                                SlotMaterial {
                                    path: path.to_string(),
                                    private_key,
                                }
                            })
                    }
                };
                material.map_err(PlatformWalletFFIResult::from)
                // `loop_guard` + the master borrow drop here.
            };

            // Wipe the resolved master's scalar (no Drop on
            // `ExtendedPrivKey`). No-op on the resident path.
            if let Some(mut master) = master_opt {
                master.private_key.non_secure_erase();
            }

            derive_result
        })()
    });

    let result = unwrap_option_or_return!(option);
    let material = unwrap_result_or_return!(result);

    let path_cstring = unwrap_result_or_return!(CString::new(material.path));

    unsafe {
        *out_key = IdentityPrivateKeyFFI {
            identity_index,
            key_index,
            private_key_bytes: *material.private_key,
            derivation_path: path_cstring.into_raw(),
        };
    }
    PlatformWalletFFIResult::ok()
}

/// Release an [`IdentityPrivateKeyFFI`] populated by
/// [`platform_wallet_derive_identity_private_key_at_slot`]. Zeroizes the
/// 32-byte scalar in place and reclaims the path string. Safe to call on
/// a null pointer or a zeroed struct (no-op / idempotent).
///
/// # Safety
/// `out_key.derivation_path`, if non-null, must have come from
/// `CString::into_raw` via the derive entry point and must not be freed
/// twice.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_derive_identity_private_key_at_slot_free(
    out_key: *mut IdentityPrivateKeyFFI,
) {
    if out_key.is_null() {
        return;
    }
    let key = unsafe { &mut *out_key };
    if !key.derivation_path.is_null() {
        let _ = unsafe { CString::from_raw(key.derivation_path) };
        key.derivation_path = ptr::null_mut();
    }
    // Volatile scrub the inline scalar — the optimizer cannot elide it.
    zeroize::Zeroize::zeroize(&mut key.private_key_bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a heap-detached row the way the production path does
    /// (CString::into_raw for the path, a real secret inline) so the
    /// free path is exercised on genuinely-owned allocations.
    fn make_owned_key(secret: [u8; 32]) -> IdentityPrivateKeyFFI {
        let path = CString::new("m/9'/1'/5'/0'/0'/0'/0'").unwrap();
        IdentityPrivateKeyFFI {
            identity_index: 0,
            key_index: 0,
            private_key_bytes: secret,
            derivation_path: path.into_raw(),
        }
    }

    /// The free path scrubs the inline scalar and nulls the path pointer,
    /// leaving the struct safe to release a second time.
    #[test]
    fn free_zeroizes_secret_and_is_idempotent() {
        let secret = [0x3Cu8; 32];
        let mut key = make_owned_key(secret);

        assert_eq!(key.private_key_bytes, secret);
        assert!(!key.derivation_path.is_null());

        // SAFETY: freshly-detached, sole release.
        unsafe { platform_wallet_derive_identity_private_key_at_slot_free(&mut key) };

        assert_eq!(
            key.private_key_bytes, [0u8; 32],
            "private_key_bytes must be zeroized after free"
        );
        assert!(key.derivation_path.is_null());

        // Second free on the reset struct must not double-free or panic.
        unsafe { platform_wallet_derive_identity_private_key_at_slot_free(&mut key) };
        assert_eq!(key.private_key_bytes, [0u8; 32]);

        // Null pointer is a safe no-op.
        unsafe { platform_wallet_derive_identity_private_key_at_slot_free(ptr::null_mut()) };
    }

    /// `empty()` is the all-zero starting state a failed call leaves
    /// behind.
    #[test]
    fn empty_is_all_zero() {
        let e = IdentityPrivateKeyFFI::empty();
        assert_eq!(e.identity_index, 0);
        assert_eq!(e.key_index, 0);
        assert_eq!(e.private_key_bytes, [0u8; 32]);
        assert!(e.derivation_path.is_null());
    }
}
