//! FFI bindings for AssetLockManager operations.
//!
//! Mirrors the structure of `platform_wallet::wallet::asset_lock`.

mod build;
mod built_callback;
mod manager;
mod sync;

pub use build::*;
pub use built_callback::AssetLockBuiltFn;
pub(crate) use built_callback::{observer_ref, AssetLockBuiltCallback};
pub use manager::*;
pub use sync::*;

use crate::error::{payment_class, PlatformWalletFFIResult};
use crate::handle::{Handle, ASSET_LOCK_MANAGER_STORAGE};
use crate::runtime::runtime;
use platform_wallet::wallet::asset_lock::orchestration::handoff_failure_source;
use platform_wallet::PlatformWalletError;

/// Stamp `failure` — an asset-lock call whose wallet step succeeded but whose
/// output could not be handed back (the proof did not encode, the path held an
/// interior NUL) — with the class the wallet gives that failure
/// ([`AssetLockManager::handoff_failure`]): `MaybeSent`, naming no lock (the
/// tracked asset-lock list offers it), never retryable. A handle destroyed
/// meanwhile gives the same class. Code and message stay `failure`'s.
///
/// [`AssetLockManager::handoff_failure`]: platform_wallet::AssetLockManager::handoff_failure
pub(crate) fn lock_handoff_failure(
    handle: Handle,
    out_point: dashcore::OutPoint,
    failure: PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    let class = ASSET_LOCK_MANAGER_STORAGE
        .with_item(handle, |manager| {
            runtime().block_on(manager.handoff_failure(out_point))
        })
        .unwrap_or_else(handoff_failure_source);
    stamp_handoff_failure(&class, failure)
}

fn stamp_handoff_failure(
    class: &PlatformWalletError,
    failure: PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    failure.with_payment_class(payment_class(class))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{PlatformWalletFFIPaymentDelivery, PlatformWalletFFIResultCode};

    /// A proof the call built (or resumed) but could not hand back keeps the
    /// failure's own code and carries the wallet's class for it: `MaybeSent`,
    /// naming no lock, not retryable, so the host never funds another lock
    /// and finds this one in the tracked asset-lock list.
    #[test]
    fn should_keep_the_code_and_name_no_lock_when_the_result_cannot_be_handed_back() {
        let nul = std::ffi::CString::new("a\0b").expect_err("interior NUL");
        let mut result = stamp_handoff_failure(&handoff_failure_source(), nul.into());

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUtf8Conversion
        );
        assert_eq!(
            result.payment_delivery,
            PlatformWalletFFIPaymentDelivery::MaybeSent
        );
        assert_eq!(result.held_asset_lock.txid, [0; 32]);
        assert!(!result.retryable);
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };
    }

    /// With no manager to consult (its handle is gone) the failure is
    /// `MaybeSent`, never `NotSent`, and names no lock.
    #[test]
    fn should_class_a_handoff_failure_without_a_manager_as_maybe_sent() {
        let out_point = dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array([9; 32]),
            vout: 4,
        };
        let nul = std::ffi::CString::new("a\0b").expect_err("interior NUL");
        let mut result = lock_handoff_failure(Handle::MAX, out_point, nul.into());

        assert_eq!(
            result.payment_delivery,
            PlatformWalletFFIPaymentDelivery::MaybeSent
        );
        assert_eq!(result.held_asset_lock.txid, [0; 32]);
        assert!(!result.retryable);
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };
    }
}
