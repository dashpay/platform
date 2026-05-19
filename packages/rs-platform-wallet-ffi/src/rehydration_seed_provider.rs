//! `MnemonicResolverHandle` → `platform_wallet::SeedProvider` adapter.
//!
//! `load_from_persistor` now requires a runtime
//! [`SeedProvider`](platform_wallet::seed_provider::SeedProvider) to
//! re-derive each signing wallet. The iOS host already owns a
//! Swift-side mnemonic store reachable through the
//! [`MnemonicResolverHandle`] vtable (same mechanism used by the
//! on-demand signer in `sign_with_mnemonic_resolver.rs`), so wrapping
//! that resolver is the minimal correct seed source — no second
//! secret-plumbing path, no mnemonic round-tripping through Swift.
//!
//! The resolver yields a BIP-39 mnemonic for a `wallet_id`; this
//! adapter hands it to the manager as
//! [`WalletSecret::Mnemonic`](platform_wallet::seed_provider::WalletSecret),
//! borrowing it only across the wrapper and zeroizing the transient
//! buffer on drop. No secret byte is logged or placed in an error.

use std::os::raw::{c_char, c_void};

use platform_wallet::seed_provider::{
    SecretPhrase, SecretStoreErrorKind, SeedProvider, SeedUnavailable, WalletSecret,
};
use rs_sdk_ffi::{
    mnemonic_resolver_result, MnemonicResolverHandle, MNEMONIC_RESOLVER_BUFFER_CAPACITY,
};
use zeroize::Zeroizing;

/// Wraps a Swift-owned [`MnemonicResolverHandle`] as a
/// [`SeedProvider`]. Holds the raw handle pointer as a `usize` so the
/// adapter is `Send + Sync` — the Swift side promises both the vtable
/// and `ctx` are thread-stable for the resolver's lifetime (the same
/// contract the on-demand signer relies on), and the resolver must
/// outlive the `load_from_persistor` call it is passed into.
pub struct ResolverSeedProvider {
    resolver_addr: usize,
}

impl ResolverSeedProvider {
    /// # Safety
    /// `resolver` must be a valid pointer produced by
    /// `dash_sdk_mnemonic_resolver_create`, not yet destroyed, and it
    /// must outlive every `seed_for` call (i.e. the whole
    /// `load_from_persistor` invocation it is handed to).
    pub unsafe fn new(resolver: *const MnemonicResolverHandle) -> Self {
        Self {
            resolver_addr: resolver as usize,
        }
    }
}

impl SeedProvider for ResolverSeedProvider {
    fn seed_for(&self, wallet_id: [u8; 32]) -> Result<WalletSecret, SeedUnavailable> {
        if self.resolver_addr == 0 {
            return Err(SeedUnavailable::StoreUnavailable(
                SecretStoreErrorKind::BackendUnavailable,
            ));
        }

        let mut buf: Zeroizing<[u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]> =
            Zeroizing::new([0u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]);
        let mut out_len: usize = 0;

        // SAFETY: `resolver_addr` was a valid `*const
        // MnemonicResolverHandle` at construction; the caller's
        // unsafety contract guarantees it (and its thread-stable vtable
        // + ctx) outlive this call.
        let resolver = unsafe { &*(self.resolver_addr as *const MnemonicResolverHandle) };
        let vtable = unsafe { &*resolver.vtable };
        let rc = unsafe {
            (vtable.resolve)(
                resolver.ctx as *const c_void,
                wallet_id.as_ptr(),
                buf.as_mut_ptr() as *mut c_char,
                MNEMONIC_RESOLVER_BUFFER_CAPACITY,
                &mut out_len,
            )
        };
        match rc {
            x if x == mnemonic_resolver_result::SUCCESS => {}
            x if x == mnemonic_resolver_result::NOT_FOUND => {
                return Err(SeedUnavailable::Absent);
            }
            x if x == mnemonic_resolver_result::BUFFER_TOO_SMALL => {
                return Err(SeedUnavailable::StoreError(SecretStoreErrorKind::Other));
            }
            _ => {
                return Err(SeedUnavailable::StoreError(SecretStoreErrorKind::Other));
            }
        }
        if out_len == 0 || out_len > MNEMONIC_RESOLVER_BUFFER_CAPACITY {
            return Err(SeedUnavailable::StoreError(SecretStoreErrorKind::Other));
        }

        // The phrase is copied once into the zeroize-on-drop
        // `SecretPhrase`; `buf` is wiped on drop. No secret reaches a
        // log line or an error payload.
        let phrase = std::str::from_utf8(&buf[..out_len])
            .map_err(|_| SeedUnavailable::StoreError(SecretStoreErrorKind::Other))?;
        Ok(WalletSecret::Mnemonic(SecretPhrase::new(phrase)))
    }
}
