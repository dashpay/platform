//! Test-only fixtures shared by the crate's unit tests.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr::NonNull;
use std::sync::Arc;

use platform_wallet::PlatformWalletManager;

use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::event_handler::{EventHandlerCallbacks, FFIEventHandler};
use crate::persistence::{FFIPersister, PersistenceCallbacks};
use crate::runtime::runtime;
use crate::xpub_render::platform_wallet_free_string;

/// A manager over a mock SDK with no-op persistence and event callbacks:
/// registrable in the handle storage, never touching the network.
pub(crate) fn mock_manager() -> PlatformWalletManager<FFIPersister> {
    unsafe extern "C" fn begin(_: *mut std::ffi::c_void, _: *const u8) -> i32 {
        0
    }
    unsafe extern "C" fn end(_: *mut std::ffi::c_void, _: *const u8, _: bool) -> i32 {
        0
    }
    let persister = FFIPersister::new(PersistenceCallbacks {
        on_changeset_begin_fn: Some(begin),
        on_changeset_end_fn: Some(end),
        ..Default::default()
    });
    let events = FFIEventHandler::new(
        EventHandlerCallbacks {
            context: std::ptr::null_mut(),
            on_wallet_event_fn: None,
            on_error_fn: None,
            on_platform_address_sync_completed_fn: None,
            on_shielded_sync_completed_fn: None,
            on_shielded_sync_progress_fn: None,
            on_shielded_tree_progress_fn: None,
            release_fn: None,
        },
        None,
    );
    let _runtime_guard = runtime().enter();
    PlatformWalletManager::new(
        Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(dashcore::Network::Testnet)
                .build()
                .expect("mock sdk"),
        ),
        Arc::new(persister),
        Arc::new(events),
    )
}

/// Runs `call` with a dangling out pointer and returns the C string it wrote,
/// or `None` for a null one, or the failure code. A failure must leave the
/// pointer null.
pub(crate) fn out_string(
    call: impl FnOnce(*mut *mut c_char) -> PlatformWalletFFIResult,
) -> Result<Option<String>, PlatformWalletFFIResultCode> {
    let mut out: *mut c_char = NonNull::dangling().as_ptr();
    let r = call(&mut out);
    if r.code != PlatformWalletFFIResultCode::Success {
        assert!(out.is_null());
        return Err(r.code);
    }
    if out.is_null() {
        return Ok(None);
    }
    let string = unsafe { CStr::from_ptr(out) }.to_str().unwrap().to_owned();
    unsafe { platform_wallet_free_string(out) };
    Ok(Some(string))
}
