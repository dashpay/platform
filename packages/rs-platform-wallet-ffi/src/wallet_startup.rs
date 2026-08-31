//! FFI for the ordered wallet bring-up.
//!
//! One call that runs identity discovery → contact-request sync → the
//! signer-present contact-crypto drain, so the host can start Core SPV knowing
//! the DIP-15 contact addresses exist and will be in the first filter set. The
//! ordering policy itself lives in
//! [`platform_wallet::manager::startup`]; this is the marshalling shell.

use platform_wallet::manager::startup::{
    ScanKeyError, WalletStartupOptions, WalletStartupOutcome, WalletStartupStatus,
};
use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle, VTableSigner};
use std::time::Duration;

use crate::check_ptr;
use crate::dashpay::resolver_contact_crypto_provider;
use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::handle::{Handle, PLATFORM_WALLET_MANAGER_STORAGE};
use crate::identity_keys_from_mnemonic::{
    resolve_master_from_resolver_classified, ResolveFailureKind,
};
use crate::runtime::run_on_big_stack_thread;

/// Discriminants for [`WalletStartupStatus`] across the boundary.
///
/// Values are part of the ABI — append, never renumber.
#[repr(u8)]
pub enum WalletStartupStatusFFI {
    Ready = 0,
    NoIdentity = 1,
    PartialNoIdentity = 2,
    PartialAccountsPending = 3,
    DiscoveryFailed = 4,
    SeedBindingUnverified = 5,
    IdentityScanIncomplete = 6,
}

impl From<WalletStartupStatus> for WalletStartupStatusFFI {
    fn from(status: WalletStartupStatus) -> Self {
        match status {
            WalletStartupStatus::Ready => Self::Ready,
            WalletStartupStatus::NoIdentity => Self::NoIdentity,
            WalletStartupStatus::PartialNoIdentity => Self::PartialNoIdentity,
            WalletStartupStatus::PartialAccountsPending => Self::PartialAccountsPending,
            WalletStartupStatus::DiscoveryFailed => Self::DiscoveryFailed,
            WalletStartupStatus::SeedBindingUnverified => Self::SeedBindingUnverified,
            WalletStartupStatus::IdentityScanIncomplete => Self::IdentityScanIncomplete,
        }
    }
}

/// Flat result of [`platform_wallet_manager_start_wallet_subsystems`].
#[repr(C)]
pub struct WalletStartupOutcomeFFI {
    /// A [`WalletStartupStatusFFI`] discriminant.
    pub status: u8,
    /// Whether `identity_id` carries a value.
    pub has_identity_id: bool,
    /// The wallet's identity, valid only when `has_identity_id`.
    pub identity_id: [u8; 32],
    /// Discovery scans performed; `0` when a local identity was already known.
    pub discovery_attempts: u32,
    /// Whether the inline contact-request pass ran **to completion**. `false`
    /// when it came back degraded — some identities' contact documents could
    /// not be read, so their account builds were never enqueued.
    pub dashpay_sync_ran: bool,
    /// The drain was skipped because the supplied contact-crypto provider does
    /// not resolve this wallet's seed. Nothing was derived or written.
    pub seed_binding_unverified: bool,
    /// The wallet's identity scan is on record as having left indices
    /// unanswered and this launch did not close the gap. Any identity reported
    /// here is real; it may not be the only one.
    pub identity_scan_incomplete: bool,
    /// Contact-crypto entries the drain completed.
    pub contact_accounts_drained: u32,
    /// Contact-account builds still queued on return.
    pub contact_accounts_pending: u32,
    /// Wall-clock duration of the whole sequence.
    pub elapsed_ms: u64,
}

impl From<WalletStartupOutcome> for WalletStartupOutcomeFFI {
    fn from(outcome: WalletStartupOutcome) -> Self {
        let (has_identity_id, identity_id) = match outcome.identity_id {
            Some(id) => (true, id.to_buffer()),
            None => (false, [0u8; 32]),
        };
        Self {
            status: WalletStartupStatusFFI::from(outcome.status) as u8,
            has_identity_id,
            identity_id,
            discovery_attempts: outcome.discovery_attempts,
            dashpay_sync_ran: outcome.dashpay_sync_ran,
            seed_binding_unverified: outcome.seed_binding_unverified,
            identity_scan_incomplete: outcome.identity_scan_incomplete,
            contact_accounts_drained: outcome.contact_accounts_drained as u32,
            contact_accounts_pending: outcome.contact_accounts_pending as u32,
            elapsed_ms: outcome.elapsed.as_millis() as u64,
        }
    }
}

/// Bring one wallet's DashPay state up in dependency order.
///
/// Blocks until the sequence finishes or its budget expires, then writes the
/// outcome to `out_outcome`. Intended to be called once per wallet load,
/// immediately before starting Core SPV.
///
/// Errors are limited to the request itself: an invalid manager handle, an
/// unknown `wallet_id`, or a `budget_secs` so large its deadline is not
/// representable. Everything about the run — an unreachable Platform, a failed
/// sync pass, an unfinished drain — is reported through `out_outcome.status`,
/// because a host must be able to start Core SPV regardless. See
/// [`WalletStartupStatusFFI`].
///
/// # Arguments
///
/// * `wallet_id` — 32 bytes.
/// * `mnemonic_resolver_handle` — nullable. Required for a Keychain-backed
///   external-signable wallet, whose seed does not live in the wallet manager;
///   null means the wallet holds resident keys and derives in-process.
/// * `identity_signer_handle` — nullable; null skips the DIP-15 auto-accept
///   pass, matching `platform_wallet_drain_pending_contact_crypto`.
/// * `budget_secs` — `0` means the crate default (20s). Deliberately not
///   "unbounded": this call gates Core SPV, so it must always terminate.
/// * `gap_limit` — `0` means the crate default.
///
/// # Safety
///
/// `wallet_id` must point to 32 readable bytes and `out_outcome` to a writable
/// [`WalletStartupOutcomeFFI`]. Both handles, when non-null, must be valid for
/// the duration of this call — the callee borrows and never retains them.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_start_wallet_subsystems(
    manager_handle: Handle,
    wallet_id: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    identity_signer_handle: *mut SignerHandle,
    budget_secs: u64,
    gap_limit: u32,
    out_outcome: *mut WalletStartupOutcomeFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_outcome);

    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);

    // The network is needed before any key material can be built, and the
    // caller gave us a manager handle rather than a wallet handle.
    let Some(network_opt) = PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(manager_handle, |m| m.wallet_network_blocking(&wid))
    else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Manager handle invalid".to_string(),
        );
    };
    let Some(network) = network_opt else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "Wallet not found".to_string(),
        );
    };

    // The master xpriv is resolved lazily, by the library, on the branch that
    // actually scans — this crate no longer predicts when that is. Erasure is
    // the library's too: it resolved the key, so it clears it.
    //
    // The handle is carried as a `usize` so the closure stays `Send + Sync`;
    // it is valid for the duration of this call, which is the only time the
    // closure can run. A null resolver means the host is telling us the wallet
    // holds resident keys (watch-only / in-process derive), so no resolver is
    // supplied at all — distinct from one that is supplied and fails, which
    // the library classifies as retryable or terminal per `ScanKeyError`.
    let resolver_addr = mnemonic_resolver_handle as usize;
    let resolve_scan_key = move || {
        resolve_master_from_resolver_classified(
            resolver_addr as *mut MnemonicResolverHandle,
            &wid,
            network,
        )
        .map_err(|failure| {
            tracing::warn!(
                wallet_id = %hex::encode(wid),
                code = ?failure.result.code,
                kind = ?failure.kind,
                "startup: could not resolve the wallet mnemonic for the identity scan"
            );
            // Carried as an outcome the library classifies, not as an FFI
            // throw: the documented contract is that only handle and wallet-id
            // problems throw, and a locked device is neither. The permanence
            // travels with it — a phrase that is not BIP-39 must not be
            // reported as something the next launch might fix.
            let detail = format!("mnemonic resolver failed ({:?})", failure.result.code);
            match failure.kind {
                ResolveFailureKind::Unavailable => ScanKeyError::Unavailable(detail),
                ResolveFailureKind::Permanent => ScanKeyError::Invalid(detail),
            }
        })
    };
    let scan_key = (!mnemonic_resolver_handle.is_null())
        .then_some(&resolve_scan_key as &(dyn Fn() -> Result<_, _> + Send + Sync));

    let signer_addr = if identity_signer_handle.is_null() {
        0usize
    } else {
        identity_signer_handle as usize
    };
    // The provider is resolver-backed, so without a resolver every crypto
    // operation would fail with `NullHandle` and the drain would report zero
    // while leaving the queue untouched. Pass `None` instead: the sequence then
    // skips the drain and reports the real pending count.
    let provider = (!mnemonic_resolver_handle.is_null())
        .then(|| resolver_contact_crypto_provider(mnemonic_resolver_handle, wid, network));

    let opts = WalletStartupOptions {
        budget: if budget_secs == 0 {
            WalletStartupOptions::default().budget
        } else {
            Duration::from_secs(budget_secs)
        },
        gap_limit: (gap_limit > 0).then_some(gap_limit),
    };

    // `run_on_big_stack_thread`, not `block_on_worker`: the manager is only
    // reachable as a `&PlatformWalletManager` borrowed from the handle store,
    // so the future cannot satisfy `block_on_worker`'s `'static` bound. The
    // scoped thread also supplies the 8 MB stack the GroveDB proof
    // verification inside discovery needs.
    let result = run_on_big_stack_thread(|| {
        PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
            crate::runtime::runtime().block_on(async {
                let signer =
                    (signer_addr != 0).then(|| unsafe { &*(signer_addr as *const VTableSigner) });
                manager
                    .start_wallet_subsystems(&wid, scan_key, provider.as_ref(), signer, opts)
                    .await
            })
        })
    });

    let outcome = match result {
        Ok(Some(Ok(outcome))) => outcome,
        Ok(Some(Err(e))) => return PlatformWalletFFIResult::from(e),
        Ok(None) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                "Manager handle invalid".to_string(),
            )
        }
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("failed to spawn the startup thread: {e}"),
            )
        }
    };

    *out_outcome = WalletStartupOutcomeFFI::from(outcome);
    PlatformWalletFFIResult::ok()
}
