//! FFI binding for revealing the private key of a single core
//! (Layer-1) address — [`platform_wallet_address_private_key`].
//!
//! Thin marshalling over
//! [`PlatformWallet::derive_core_address_private_key`](platform_wallet::PlatformWallet::derive_core_address_private_key):
//! the whole address-lookup-and-derive decision lives on the Rust
//! library side. This shim only resolves the mnemonic (for the
//! external-signable / watch-only wallets the iOS app holds, whose seed
//! lives in the iOS Keychain rather than the `WalletManager`) and
//! marshals the resulting hex + WIF strings across the C ABI.
//!
//! # Key source: chosen by wallet capability
//!
//! Mirrors the sibling
//! [`platform_wallet_preview_identity_registration_keys`](crate::platform_wallet_preview_identity_registration_keys):
//! a [`MnemonicResolverHandle`] is a *capability*, consulted only when
//! the in-process wallet lacks resident private keys.
//!
//! - **Resident-key wallet** (created from a raw seed): the resolver is
//!   never touched and the key is derived from the in-process wallet.
//! - **External-signable / watch-only wallet** (the iOS shape): the
//!   mnemonic is resolved on demand (keyed by the wallet handle's own
//!   `wallet_id`), a master [`ExtendedPrivKey`] is built, the address's
//!   key is derived from it, and the master's scalar is wiped before
//!   returning. If the resolver handle is null for such a wallet the
//!   call errors.
//!
//! The two-phase locking rule from the identity preview applies here
//! too: the wallet-manager read guard is NEVER held across the Swift
//! resolver callback.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use key_wallet::bip32::ExtendedPrivKey;
use platform_wallet::CoreAddressPrivateKey;
use zeroize::{Zeroize, Zeroizing};

use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
use crate::types::Network;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use rs_sdk_ffi::MnemonicResolverHandle;

/// The private key for one core address, in the two forms the host
/// renders: lowercase hex (64 chars) and network-aware compressed WIF.
///
/// Both strings are heap-allocated and owned by Rust until released by
/// [`platform_wallet_address_private_key_free`], which zeroizes their
/// backing bytes before freeing.
#[repr(C)]
pub struct AddressPrivateKeyFFI {
    /// Null-terminated lowercase hex of the raw 32-byte secp256k1
    /// scalar (64 hex chars). Null on the pre-cleared / freed state.
    pub private_key_hex: *mut c_char,
    /// Null-terminated WIF (Wallet Import Format) string — network-aware
    /// (mainnet vs testnet/devnet/regtest version byte) and compressed.
    pub private_key_wif: *mut c_char,
}

impl AddressPrivateKeyFFI {
    /// All-null row so a failed call leaves the caller looking at known
    /// empty state instead of uninitialized memory.
    fn empty() -> Self {
        Self {
            private_key_hex: std::ptr::null_mut(),
            private_key_wif: std::ptr::null_mut(),
        }
    }
}

/// Derive the private key for `address_cstr`, one of this wallet's
/// tracked core addresses, and return it as hex + WIF.
///
/// # Parameters
/// - `wallet_handle` — platform-wallet handle.
/// - `mnemonic_resolver_handle` — Swift-owned [`MnemonicResolverHandle`],
///   consulted only when the in-process wallet lacks resident private
///   keys (see the module docs). May be null for resident-key wallets.
/// - `address_cstr` — the address string to reveal the key for.
/// - `out` — populated on success. Release with
///   [`platform_wallet_address_private_key_free`]. Left at the empty
///   zero state on error.
///
/// Returns a [`PlatformWalletFFIResult`]; `NotFound` for an unknown
/// handle, and the typed wallet-error code (with a descriptive message)
/// when the address is not tracked by the wallet or derivation fails.
///
/// # Safety
/// `wallet_handle` must come from the platform-wallet handle registry.
/// `mnemonic_resolver_handle`, when non-null, must come from
/// [`rs_sdk_ffi::dash_sdk_mnemonic_resolver_create`] and remain valid
/// for the duration of the call. `address_cstr` must be a valid
/// NUL-terminated UTF-8 C-string, and `out` a valid writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_address_private_key(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    address_cstr: *const c_char,
    out: *mut AddressPrivateKeyFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out);
    unsafe { *out = AddressPrivateKeyFFI::empty() };
    check_ptr!(address_cstr);

    let address_str = unwrap_result_or_return!(unsafe { CStr::from_ptr(address_cstr) }.to_str());

    let option = PLATFORM_WALLET_STORAGE.with_item(
        wallet_handle,
        |wallet| -> Result<CoreAddressPrivateKey, PlatformWalletFFIResult> {
            let network: Network = wallet.network();

            // Phase 1 — capability probe under a SHORT read guard,
            // dropped before any resolver interaction.
            let is_resident = {
                let wm = wallet.wallet_manager().blocking_read();
                match wm.get_wallet(&wallet.wallet_id()) {
                    Some(key_wallet) => {
                        !key_wallet.is_external_signable() && !key_wallet.is_watch_only()
                    }
                    None => {
                        return Err(PlatformWalletFFIResult::err(
                            PlatformWalletFFIResultCode::ErrorInvalidHandle,
                            "wallet not found in wallet manager",
                        ));
                    }
                }
            };

            // Phase 2 — resolve the master xpriv for external-signable /
            // watch-only wallets. NEVER under the guard above: the
            // resolver synchronously re-enters Swift and reads the iOS
            // Keychain (which can stall on biometric unlock).
            let mut master_opt: Option<ExtendedPrivKey> = None;
            if !is_resident {
                if mnemonic_resolver_handle.is_null() {
                    return Err(PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorWalletOperation,
                        "this wallet has no resident private keys (external-signable / \
                         watch-only); a mnemonic resolver handle is required to reveal \
                         an address private key",
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

            // Phase 3 — library lookup + derive (re-acquires the guard
            // internally; the resolver, if any, has already run).
            let result = wallet.derive_core_address_private_key(address_str, master_opt.as_ref());

            // Wipe the resolved master's inner scalar — `ExtendedPrivKey`
            // has no `Drop` / `Zeroize`. No-op on the resident path.
            if let Some(mut master) = master_opt {
                master.private_key.non_secure_erase();
            }

            result.map_err(PlatformWalletFFIResult::from)
        },
    );
    let result = unwrap_option_or_return!(option);
    let secret = unwrap_result_or_return!(result);

    // Move the sensitive fields out and marshal them through
    // `secret_string_into_raw`, which scrubs every transient plaintext
    // copy (see its docs — a naive `CString::new` over the
    // zero-spare-capacity hex / WIF buffers would realloc and free the
    // original un-zeroized). `private_key` (Zeroizing) is wiped when it
    // drops at the end of this function; the returned buffers are
    // zeroized by `_free`.
    let CoreAddressPrivateKey {
        private_key, wif, ..
    } = secret;
    let hex_c = unwrap_result_or_return!(secret_string_into_raw(Zeroizing::new(hex::encode(
        &private_key[..]
    ))));
    let wif_c = unwrap_result_or_return!(secret_string_into_raw(Zeroizing::new(wif)));

    unsafe {
        *out = AddressPrivateKeyFFI {
            private_key_hex: hex_c,
            private_key_wif: wif_c,
        };
    }
    PlatformWalletFFIResult::ok()
}

/// Move a secret string onto the heap as a NUL-terminated C-string
/// without leaving an un-zeroized plaintext copy behind.
///
/// `CString::new` must append a NUL terminator; handed a
/// zero-spare-capacity buffer — which both `hex::encode` and
/// `PrivateKey::to_wif` produce (`len == capacity`) — its internal
/// `reserve_exact(1)` reallocates and frees the original allocation
/// **without** zeroizing it, stranding the plaintext secret in reclaimed
/// heap. We copy into a buffer pre-sized with room for the NUL so
/// `CString::new` cannot realloc, and wipe the `Zeroizing` source on
/// drop. The only surviving plaintext is then the returned buffer, which
/// the matching `_free` zeroizes.
pub(crate) fn secret_string_into_raw(
    secret: Zeroizing<String>,
) -> Result<*mut c_char, std::ffi::NulError> {
    let mut buf = Vec::with_capacity(secret.len() + 1);
    buf.extend_from_slice(secret.as_bytes());
    // `secret` wiped here on drop; `buf` (len N, capacity ≥ N+1) moves
    // into the CString with no realloc, so no plaintext copy is stranded.
    drop(secret);
    Ok(CString::new(buf)?.into_raw())
}

/// Release an [`AddressPrivateKeyFFI`] populated by
/// [`platform_wallet_address_private_key`], zeroizing the sensitive hex
/// and WIF backing bytes before freeing. Safe on a null outer pointer or
/// an already-freed / empty struct (no-op). Fields are nulled after free
/// so a second call is idempotent.
///
/// # Safety
/// `out`'s pointers must have been produced by
/// [`platform_wallet_address_private_key`] and must not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_address_private_key_free(out: *mut AddressPrivateKeyFFI) {
    if out.is_null() {
        return;
    }
    let out = unsafe { &mut *out };
    if !out.private_key_hex.is_null() {
        let mut bytes = unsafe { CString::from_raw(out.private_key_hex) }.into_bytes_with_nul();
        bytes.zeroize();
        out.private_key_hex = std::ptr::null_mut();
    }
    if !out.private_key_wif.is_null() {
        let mut bytes = unsafe { CString::from_raw(out.private_key_wif) }.into_bytes_with_nul();
        bytes.zeroize();
        out.private_key_wif = std::ptr::null_mut();
    }
}
