//! C callback that reports the outpoint of a newly built asset lock.

use std::os::raw::c_void;

use dashcore::OutPoint;
use platform_wallet::wallet::asset_lock::AssetLockBuiltObserver;

use crate::asset_lock_persistence::outpoint_to_bytes;

/// Receives the outpoint of an asset lock that a funding call has just
/// built, before the lock's row is persisted or the lock is broadcast.
///
/// `out_point_36` points to 36 bytes (32-byte raw txid followed by the
/// 4-byte little-endian vout, the layout of `AssetLockEntryFFI.out_point`)
/// and is valid only for the duration of the call. The callback runs
/// synchronously on the funding call's worker thread, at most once per
/// call, and must return promptly. It runs on the SDK's runtime thread, so
/// it must not call any wallet or SDK function: a blocking FFI call from
/// there panics the runtime and aborts the process.
/// Record the outpoint before returning (durably, if it has to survive a
/// restart): the lock's row is persisted right after. The funding call's
/// result then tells what happened to the lock:
/// `ErrorTransactionBroadcastRejected` (26) means it was removed again; any
/// other failure leaves it tracked and resumable by this outpoint (a resume
/// may find it already consumed). Both hold for this session; the row store
/// and removal behind them are best-effort, so after a restart a failed
/// store can leave no row for a broadcast lock, or the row of a removed one.
/// That holds for the exports that take this callback today: shielded
/// funding and platform-address funding from the wallet balance or an
/// account drain. The pooled wallet-drain export passes no callback; its
/// lock is recovered by operation id instead, and it would need its own
/// contract here before taking one. Exports without this callback (identity
/// registration and top-up, the generic funded proof, the wallet drain)
/// create rows nobody reported.
/// `NULL` means no callback.
pub type AssetLockBuiltFn =
    Option<unsafe extern "C" fn(context: *mut c_void, out_point_36: *const u8)>;

/// A caller-supplied [`AssetLockBuiltFn`] with its opaque context.
///
/// The context is held as `usize` so the funding future that captures it
/// stays `Send + 'static`; the caller's contract keeps it valid for the
/// whole (blocking) FFI call.
#[derive(Clone, Copy)]
pub(crate) struct AssetLockBuiltCallback {
    func: unsafe extern "C" fn(context: *mut c_void, out_point_36: *const u8),
    context: usize,
}

impl AssetLockBuiltCallback {
    /// `None` when the caller passed no callback.
    pub(crate) fn new(func: AssetLockBuiltFn, context: *mut c_void) -> Option<Self> {
        func.map(|func| Self {
            func,
            context: context as usize,
        })
    }

    /// The callback as the observer the funding paths take.
    pub(crate) fn into_observer(self) -> impl Fn(OutPoint) + Send + Sync {
        move |out_point| self.notify(out_point)
    }

    fn notify(&self, out_point: OutPoint) {
        let bytes = outpoint_to_bytes(&out_point);
        // SAFETY: the caller guarantees `func` and `context` stay valid for
        // the duration of the FFI call this callback belongs to, and `bytes`
        // outlives the call below.
        unsafe { (self.func)(self.context as *mut c_void, bytes.as_ptr()) }
    }
}

/// `on_built` as the optional observer reference the funding paths take.
pub(crate) fn observer_ref<F>(on_built: &Option<F>) -> Option<&AssetLockBuiltObserver<'_>>
where
    F: Fn(OutPoint) + Send + Sync,
{
    on_built
        .as_ref()
        .map(|notify| notify as &AssetLockBuiltObserver<'_>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::hashes::Hash;
    use dashcore::Txid;
    use std::sync::Mutex;

    /// Appends the reported outpoint to the `Mutex<Vec<[u8; 36]>>` the
    /// context points to.
    unsafe extern "C" fn record(context: *mut c_void, out_point_36: *const u8) {
        let received = &*(context as *const Mutex<Vec<[u8; 36]>>);
        let mut bytes = [0u8; 36];
        std::ptr::copy_nonoverlapping(out_point_36, bytes.as_mut_ptr(), 36);
        received.lock().unwrap().push(bytes);
    }

    #[test]
    fn should_yield_no_callback_for_an_absent_function() {
        assert!(AssetLockBuiltCallback::new(None, std::ptr::null_mut()).is_none());
    }

    #[test]
    fn should_pass_the_context_and_the_entry_layout_outpoint() {
        let received = Mutex::new(Vec::new());
        let out_point = OutPoint::new(Txid::from_byte_array([7; 32]), 3);
        let callback = AssetLockBuiltCallback::new(
            Some(record),
            &received as *const Mutex<Vec<[u8; 36]>> as *mut c_void,
        )
        .expect("callback present");

        let on_built = Some(callback.into_observer());
        observer_ref(&on_built).expect("observer present")(out_point);

        assert_eq!(
            received.into_inner().unwrap(),
            vec![outpoint_to_bytes(&out_point)]
        );
    }
}
