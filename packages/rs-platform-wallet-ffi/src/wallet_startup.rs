//! FFI for the ordered wallet bring-up.
//!
//! One call that runs identity discovery → contact-request sync → the
//! signer-present contact-crypto drain, so the host can start Core SPV knowing
//! the DIP-15 contact addresses exist and will be in the first filter set. The
//! ordering policy itself lives in
//! [`platform_wallet::manager::startup`]; this is the marshalling shell.

use platform_wallet::manager::startup::{
    WalletStartupOptions, WalletStartupOutcome, WalletStartupStatus,
};
use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle, VTableSigner};
use std::time::Duration;

use crate::check_ptr;
use crate::dashpay::resolver_contact_crypto_provider;
use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::handle::{Handle, PLATFORM_WALLET_MANAGER_STORAGE};
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
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
}

impl From<WalletStartupStatus> for WalletStartupStatusFFI {
    fn from(status: WalletStartupStatus) -> Self {
        match status {
            WalletStartupStatus::Ready => Self::Ready,
            WalletStartupStatus::NoIdentity => Self::NoIdentity,
            WalletStartupStatus::PartialNoIdentity => Self::PartialNoIdentity,
            WalletStartupStatus::PartialAccountsPending => Self::PartialAccountsPending,
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
    /// Whether the inline contact-request pass ran.
    pub dashpay_sync_ran: bool,
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
/// Only an invalid manager handle or an unknown `wallet_id` produce an error;
/// an unreachable Platform, a failed sync pass and an unfinished drain are all
/// reported through `out_outcome.status`, because a host must be able to start
/// Core SPV regardless. See [`WalletStartupStatusFFI`].
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

    // Resolve the master xpriv once, up front. The helper holds the mnemonic
    // and seed in `Zeroizing` buffers and scrubs them before returning; the
    // master itself has no `Drop`, so it is erased explicitly below.
    let mut master = if mnemonic_resolver_handle.is_null() {
        None
    } else {
        match resolve_master_from_resolver(mnemonic_resolver_handle, &wid, network) {
            Ok(master) => Some(master),
            Err(e) => return e,
        }
    };

    let signer_addr = if identity_signer_handle.is_null() {
        0usize
    } else {
        identity_signer_handle as usize
    };
    let provider = resolver_contact_crypto_provider(mnemonic_resolver_handle, wid, network);

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
                    .start_wallet_subsystems(&wid, master.as_ref(), &provider, signer, opts)
                    .await
            })
        })
    });

    // Erase the master before any early return below.
    if let Some(master) = master.as_mut() {
        master.private_key.non_secure_erase();
    }

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
