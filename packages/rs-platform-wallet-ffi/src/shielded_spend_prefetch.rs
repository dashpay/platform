//! FFI entry point that prepares an upcoming shielded spend.
//!
//! Every shielded spend (transfer, unshield, withdraw, identity top-up /
//! create from the pool) must pick an anchor Platform has recorded, which
//! takes a round trip that downloads and proof-verifies Platform's recorded
//! anchor set. Hosts call
//! [`platform_wallet_manager_shielded_prefetch_spend_anchors`] when a send
//! screen opens so that round trip overlaps the user filling in the form
//! instead of delaying the send. Feature-gated behind `shielded`.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;

/// Prefetch Platform's recorded shielded-anchor set for the next spend on
/// `handle`'s network, in the background.
///
/// Returns as soon as the fetch is queued; it runs on the FFI runtime. A
/// shielded send started within the next ~30 s, before any shielded sync
/// grows the commitment tree, reuses the prefetched set and skips that round
/// trip; otherwise the send fetches for itself exactly as without this call.
/// Cheap to call repeatedly (a no-op while a usable set is cached), and a
/// failed fetch is only logged — it never affects a later send.
///
/// Returns `ErrorInvalidHandle` for an unknown `handle` and
/// `ErrorWalletOperation` when shielded support isn't configured on the
/// manager.
#[no_mangle]
pub extern "C" fn platform_wallet_manager_shielded_prefetch_spend_anchors(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let lookup = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.shielded_coordinator())
    });
    let coordinator = match lookup {
        None => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                format!("invalid manager handle: {handle}"),
            );
        }
        Some(None) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "shielded support not configured — call platform_wallet_manager_configure_shielded first",
            );
        }
        Some(Some(coordinator)) => coordinator,
    };
    runtime().spawn(async move {
        if let Err(e) = coordinator.prefetch_recorded_anchors().await {
            tracing::debug!(
                error = %e,
                "shielded spend-anchor prefetch failed; the next spend fetches for itself"
            );
        }
    });
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_reject_an_unknown_manager_handle() {
        let mut result = platform_wallet_manager_shielded_prefetch_spend_anchors(NULL_HANDLE);
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        unsafe { crate::platform_wallet_ffi_result_free(&mut result) };
    }
}
