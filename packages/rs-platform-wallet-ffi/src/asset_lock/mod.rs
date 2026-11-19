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
use platform_wallet::PlatformWalletError;

/// Stamp `failure` — an asset-lock call whose wallet step succeeded but whose
/// output could not be handed back (the proof did not encode, the path held an
/// interior NUL) — with the class the wallet gives a failure while its lock is
/// tracked and unconsumed: [`PaymentDelivery::LockHeld`] on `out_point`, not
/// retryable. The call built or resumed that lock, so the host resumes it
/// rather than funding a new one. Code and message stay `failure`'s.
///
/// [`PaymentDelivery::LockHeld`]: platform_wallet::PaymentDelivery::LockHeld
pub(crate) fn lock_held_handoff_failure(
    out_point: dashcore::OutPoint,
    failure: PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    let held = PlatformWalletError::PaymentAlreadyDispatched {
        held_lock: Some(out_point),
        source: Box::new(PlatformWalletError::AssetLockTransaction(
            "the asset lock's proof could not be handed back".into(),
        )),
    };
    failure.with_payment_class(payment_class(&held))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{PlatformWalletFFIPaymentDelivery, PlatformWalletFFIResultCode};

    /// A proof the call built (or resumed) but could not hand back keeps the
    /// failure's own code and names the tracked lock as held, not retryable:
    /// the host resumes it instead of funding another lock.
    #[test]
    fn should_name_the_held_lock_when_the_result_cannot_be_handed_back() {
        let out_point = dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array([9; 32]),
            vout: 4,
        };
        let nul = std::ffi::CString::new("a\0b").expect_err("interior NUL");
        let mut result = lock_held_handoff_failure(out_point, nul.into());

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUtf8Conversion
        );
        assert_eq!(
            result.payment_delivery,
            PlatformWalletFFIPaymentDelivery::LockHeld
        );
        assert_eq!(result.held_asset_lock.txid, [9; 32]);
        assert_eq!(result.held_asset_lock.vout, 4);
        assert!(!result.retryable);
        unsafe { crate::error::platform_wallet_ffi_result_free(&mut result) };
    }
}
