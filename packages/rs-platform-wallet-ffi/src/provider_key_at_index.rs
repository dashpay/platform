//! FFI binding for deriving a wallet's provider key-material key at a
//! single index — [`platform_wallet_provider_key_at_index`].
//!
//! Thin marshalling over
//! [`PlatformWallet::derive_provider_key_at_index`](platform_wallet::PlatformWallet::derive_provider_key_at_index):
//! the whole derivation decision (which curve, hardened vs non-hardened,
//! whether a seed is even needed) lives on the Rust library side. This
//! shim only resolves the mnemonic (for the external-signable /
//! watch-only wallets the iOS app holds, whose seed lives in the iOS
//! Keychain rather than the `WalletManager`) and marshals the resulting
//! hex strings across the C ABI.
//!
//! # Key source: chosen by wallet capability
//!
//! Mirrors the sibling
//! [`platform_wallet_address_private_key`](crate::platform_wallet_address_private_key):
//! a [`MnemonicResolverHandle`] is a *capability*, consulted only when
//! the in-process wallet lacks resident private keys **and** a seed is
//! actually required for the request.
//!
//! The curve asymmetry decides when the resolver is required:
//! - **Operator (BLS), public listing:** derives from the account xpub
//!   with no seed — the resolver is never touched, even for a watch-only
//!   wallet. `include_private` flips this to require a seed.
//! - **Platform node (Ed25519):** SLIP-10 is hardened-only, so *every*
//!   derivation (public or private) needs the seed. A watch-only wallet
//!   therefore requires the resolver even to list platform-node public
//!   keys; a null resolver handle for such a wallet errors.
//!
//! The two-phase locking rule from the address-key sibling applies here
//! too: the wallet-manager read guard is NEVER held across the Swift
//! resolver callback.

use std::ffi::CString;
use std::os::raw::c_char;

use key_wallet::bip32::ExtendedPrivKey;
use platform_wallet::{ProviderDerivedKey, ProviderKeyKind};
use zeroize::{Zeroize, Zeroizing};

use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
use crate::types::Network;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use rs_sdk_ffi::MnemonicResolverHandle;

/// Which provider key-material account to derive from. Matches the
/// account `type_tag`s the host already uses (10 = operator, 11 =
/// platform node) so the Swift side can pass the same discriminator it
/// renders with.
pub const PROVIDER_KEY_KIND_OPERATOR: u8 = 10;
/// See [`PROVIDER_KEY_KIND_OPERATOR`].
pub const PROVIDER_KEY_KIND_PLATFORM_NODE: u8 = 11;

/// One provider key derived at a single index, in the hex forms the host
/// renders.
///
/// All strings are heap-allocated and owned by Rust until released by
/// [`platform_wallet_provider_key_at_index_free`], which zeroizes the
/// private-key backing bytes before freeing.
#[repr(C)]
pub struct ProviderKeyAtIndexFFI {
    /// The key index that was derived (`#0..`).
    pub index: u32,
    /// Null-terminated lowercase hex of the raw curve public key — 96
    /// hex chars for a BLS-48 operator key, 64 for an Ed25519-32
    /// platform-node key. Null on the pre-cleared / freed state.
    pub public_key_hex: *mut c_char,
    /// Null-terminated lowercase hex of the 20-byte platform node id
    /// (40 chars) — `hash160` of the Ed25519 public key. Null for
    /// operator keys (no node id) and on the empty state.
    pub node_id_hex: *mut c_char,
    /// Null-terminated lowercase hex of the raw 32-byte private scalar
    /// (64 chars), populated only when `include_private` was set. BLS /
    /// Ed25519 keys have no WIF, so this is the only private form. Null
    /// otherwise; zeroized by the free function.
    pub private_key_hex: *mut c_char,
}

impl ProviderKeyAtIndexFFI {
    /// All-null row so a failed call leaves the caller looking at known
    /// empty state instead of uninitialized memory.
    fn empty() -> Self {
        Self {
            index: 0,
            public_key_hex: std::ptr::null_mut(),
            node_id_hex: std::ptr::null_mut(),
            private_key_hex: std::ptr::null_mut(),
        }
    }
}

/// Derive this wallet's provider key of `kind` at `index` and return it
/// as hex (public key, optional node id, optional private key).
///
/// # Parameters
/// - `wallet_handle` — platform-wallet handle.
/// - `mnemonic_resolver_handle` — Swift-owned [`MnemonicResolverHandle`],
///   consulted only when a seed is required and the in-process wallet
///   lacks resident private keys (see the module docs). May be null for
///   an operator public listing on any wallet, or for a resident-key
///   wallet.
/// - `kind` — [`PROVIDER_KEY_KIND_OPERATOR`] (BLS, tag 10) or
///   [`PROVIDER_KEY_KIND_PLATFORM_NODE`] (Ed25519, tag 11).
/// - `index` — the key index to derive.
/// - `include_private` — also return the raw private scalar.
/// - `out` — populated on success. Release with
///   [`platform_wallet_provider_key_at_index_free`]. Left at the empty
///   zero state on error.
///
/// Returns a [`PlatformWalletFFIResult`]; `NotFound` for an unknown
/// handle, and the typed wallet-error code (with a descriptive message)
/// when the wallet has no account of that kind or derivation fails.
///
/// # Safety
/// `wallet_handle` must come from the platform-wallet handle registry.
/// `mnemonic_resolver_handle`, when non-null, must come from
/// [`rs_sdk_ffi::dash_sdk_mnemonic_resolver_create`] and remain valid
/// for the duration of the call. `out` must be a valid writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_provider_key_at_index(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    kind: u8,
    index: u32,
    include_private: bool,
    out: *mut ProviderKeyAtIndexFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out);
    unsafe { *out = ProviderKeyAtIndexFFI::empty() };

    let kind = match kind {
        PROVIDER_KEY_KIND_OPERATOR => ProviderKeyKind::Operator,
        PROVIDER_KEY_KIND_PLATFORM_NODE => ProviderKeyKind::PlatformNode,
        other => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!(
                    "unknown provider key kind {other} (expected {PROVIDER_KEY_KIND_OPERATOR} \
                     operator or {PROVIDER_KEY_KIND_PLATFORM_NODE} platform node)"
                ),
            );
        }
    };

    // A seed is needed for every private reveal, and for *all*
    // platform-node derivations (Ed25519/SLIP-10 is hardened-only, so
    // even the public key needs the private path). An operator public
    // listing needs nothing but the account xpub.
    let need_seed = include_private || matches!(kind, ProviderKeyKind::PlatformNode);

    let option = PLATFORM_WALLET_STORAGE.with_item(
        wallet_handle,
        |wallet| -> Result<ProviderDerivedKey, PlatformWalletFFIResult> {
            let network: Network = wallet.network();

            // Phase 1 — capability probe under a SHORT read guard,
            // dropped before any resolver interaction. Only relevant when
            // a seed is required at all.
            let is_resident = if need_seed {
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
            } else {
                // No seed needed — the resident/watch-only distinction is
                // irrelevant; the library derives from the account xpub.
                true
            };

            // Phase 2 — resolve the master xpriv for external-signable /
            // watch-only wallets that need a seed. NEVER under the guard
            // above: the resolver synchronously re-enters Swift and reads
            // the iOS Keychain (which can stall on biometric unlock).
            let mut master_opt: Option<ExtendedPrivKey> = None;
            if need_seed && !is_resident {
                if mnemonic_resolver_handle.is_null() {
                    return Err(PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorWalletOperation,
                        "this wallet has no resident private keys (external-signable / \
                         watch-only); a mnemonic resolver handle is required to derive \
                         this provider key",
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

            // Phase 3 — library derive (re-acquires the guard internally;
            // the resolver, if any, has already run).
            let result = wallet.derive_provider_key_at_index(
                kind,
                index,
                master_opt.as_ref(),
                include_private,
            );

            // Wipe the resolved master's inner scalar — `ExtendedPrivKey`
            // has no `Drop` / `Zeroize`. No-op on the seedless path.
            if let Some(mut master) = master_opt {
                master.private_key.non_secure_erase();
            }

            result.map_err(PlatformWalletFFIResult::from)
        },
    );
    let result = unwrap_option_or_return!(option);
    let derived = unwrap_result_or_return!(result);

    // Move the sensitive field out. `private_key` (Zeroizing) is wiped
    // when it drops at the end of this function; its hex byte buffer
    // moves directly into the returned CString (no residual plaintext
    // copy left on the heap) and is zeroized by `_free`.
    let ProviderDerivedKey {
        index,
        public_key_bytes,
        node_id,
        private_key,
    } = derived;

    let public_key_hex = unwrap_result_or_return!(CString::new(hex::encode(public_key_bytes)));
    let node_id_hex = match node_id {
        Some(id) => unwrap_result_or_return!(CString::new(hex::encode(id))).into_raw(),
        None => std::ptr::null_mut(),
    };
    // Secret: marshal through `secret_string_into_raw` so no un-zeroized
    // plaintext copy is stranded during CString NUL-termination (see its
    // docs). `public_key_hex` / `node_id_hex` above are public material,
    // so a plain `CString::new` is fine for them.
    let private_key_hex = match private_key {
        Some(pk) => unwrap_result_or_return!(crate::address_private_key::secret_string_into_raw(
            Zeroizing::new(hex::encode(&pk[..]))
        )),
        None => std::ptr::null_mut(),
    };

    unsafe {
        *out = ProviderKeyAtIndexFFI {
            index,
            public_key_hex: public_key_hex.into_raw(),
            node_id_hex,
            private_key_hex,
        };
    }
    PlatformWalletFFIResult::ok()
}

/// Release a [`ProviderKeyAtIndexFFI`] populated by
/// [`platform_wallet_provider_key_at_index`], zeroizing the sensitive
/// private-key backing bytes before freeing. Safe on a null outer
/// pointer or an already-freed / empty struct (no-op). Fields are nulled
/// after free so a second call is idempotent.
///
/// # Safety
/// `out`'s pointers must have been produced by
/// [`platform_wallet_provider_key_at_index`] and must not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_provider_key_at_index_free(
    out: *mut ProviderKeyAtIndexFFI,
) {
    if out.is_null() {
        return;
    }
    let out = unsafe { &mut *out };
    // Public key + node id are not secret — free without scrubbing.
    if !out.public_key_hex.is_null() {
        let _ = unsafe { CString::from_raw(out.public_key_hex) };
        out.public_key_hex = std::ptr::null_mut();
    }
    if !out.node_id_hex.is_null() {
        let _ = unsafe { CString::from_raw(out.node_id_hex) };
        out.node_id_hex = std::ptr::null_mut();
    }
    // The private-key hex is sensitive — zeroize its bytes before free.
    if !out.private_key_hex.is_null() {
        let mut bytes = unsafe { CString::from_raw(out.private_key_hex) }.into_bytes_with_nul();
        bytes.zeroize();
        out.private_key_hex = std::ptr::null_mut();
    }
}
