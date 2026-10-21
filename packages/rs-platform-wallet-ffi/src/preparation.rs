//! Synchronous request scopes transported into the wallet's async runtime.

use dash_sdk::PlatformWritePermit;
use std::cell::RefCell;
use std::ffi::c_void;

thread_local! {
    static PERMITS: RefCell<Vec<PlatformWritePermit>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn current_permits() -> Vec<PlatformWritePermit> {
    PERMITS.with(|permits| permits.borrow().clone())
}

pub(crate) fn with_permits<T>(
    permits: Vec<PlatformWritePermit>,
    operation: impl FnOnce() -> T,
) -> T {
    struct Restore(Vec<PlatformWritePermit>);
    impl Drop for Restore {
        fn drop(&mut self) {
            PERMITS.with(|permits| *permits.borrow_mut() = std::mem::take(&mut self.0));
        }
    }
    let previous = PERMITS.with(|current| current.replace(permits));
    let _restore = Restore(previous);
    operation()
}

/// Invoke a synchronous wallet FFI operation with its original preparation.
///
/// The callback is invoked exactly once before this function returns. It must
/// not unwind, suspend, or retain the context beyond this call. A null callback
/// is a no-op. The context may be null if the callback does not dereference it.
/// Nested scopes preserve every enclosing permit. Wallet runtime entry points
/// copy these permits into the actual async worker task; SDK write guards then
/// validate them against the SDK performing the operation. This does not
/// change the shared SDK policy, and reads remain available.
///
/// # Safety
///
/// A non-null callback must be callable with `context` for the entire call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_with_write_permit(
    instance_id: u64,
    generation: u64,
    protocol_version: u32,
    callback: Option<unsafe extern "C" fn(*mut c_void)>,
    context: *mut c_void,
) {
    let Some(callback) = callback else { return };
    let mut permits = current_permits();
    permits.push(PlatformWritePermit {
        instance_id,
        generation,
        protocol_version,
    });
    with_permits(permits, || unsafe { callback(context) });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn permit(generation: u64) -> PlatformWritePermit {
        PlatformWritePermit {
            instance_id: 1,
            generation,
            protocol_version: 1,
        }
    }

    #[test]
    fn should_restore_preparation_scope_after_panic_and_nested_callback() {
        unsafe extern "C" fn inspect(context: *mut c_void) {
            let output = unsafe { &mut *(context as *mut Vec<PlatformWritePermit>) };
            *output = current_permits();
        }
        let mut output = Vec::<PlatformWritePermit>::new();
        with_permits(vec![permit(1)], || {
            unsafe {
                platform_wallet_with_write_permit(
                    1,
                    2,
                    1,
                    Some(inspect),
                    &mut output as *mut _ as *mut c_void,
                );
            }
            assert_eq!(output, vec![permit(1), permit(2)]);
            assert_eq!(current_permits(), vec![permit(1)]);
        });
        assert!(current_permits().is_empty());
        let _ = std::panic::catch_unwind(|| {
            with_permits(vec![permit(3)], || panic!("scope restoration"));
        });
        assert!(current_permits().is_empty());
    }
}
