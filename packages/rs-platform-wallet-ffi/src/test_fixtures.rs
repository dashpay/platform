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
    mock_manager_with(mock_persister())
}

fn mock_persister() -> FFIPersister {
    unsafe extern "C" fn begin(_: *mut std::ffi::c_void, _: *const u8) -> i32 {
        0
    }
    unsafe extern "C" fn end(_: *mut std::ffi::c_void, _: *const u8, _: bool) -> i32 {
        0
    }
    FFIPersister::new(PersistenceCallbacks {
        on_changeset_begin_fn: Some(begin),
        on_changeset_end_fn: Some(end),
        ..Default::default()
    })
}

/// [`mock_manager`] over `persister`.
pub(crate) fn mock_manager_with(persister: FFIPersister) -> PlatformWalletManager<FFIPersister> {
    mock_manager_with_persister_and_events(
        persister,
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
    )
}

/// [`mock_manager`] delivering its events to `callbacks`.
pub(crate) fn mock_manager_with_events(
    callbacks: EventHandlerCallbacks,
) -> PlatformWalletManager<FFIPersister> {
    mock_manager_with_persister_and_events(mock_persister(), callbacks)
}

fn mock_manager_with_persister_and_events(
    persister: FFIPersister,
    callbacks: EventHandlerCallbacks,
) -> PlatformWalletManager<FFIPersister> {
    let events = FFIEventHandler::new(callbacks, None);
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

// Viewing-key callbacks that store nothing and load nothing: enough for a
// persister to declare `SHIELDED_VIEWING_KEYS`, which a seed-backed shielded
// bind requires.
#[cfg(feature = "shielded")]
pub(crate) unsafe extern "C" fn noop_persist_viewing_keys(
    _ctx: *mut std::ffi::c_void,
    _wallet_id: *const u8,
    _entries: *const crate::shielded_persistence::ShieldedViewingKeyFFI,
    _count: usize,
) -> i32 {
    0
}
#[cfg(feature = "shielded")]
pub(crate) unsafe extern "C" fn noop_load_viewing_keys(
    _ctx: *mut std::ffi::c_void,
    out_entries: *mut *const crate::shielded_persistence::ShieldedViewingKeyRestoreFFI,
    out_count: *mut usize,
) -> i32 {
    *out_entries = std::ptr::null();
    *out_count = 0;
    0
}
#[cfg(feature = "shielded")]
pub(crate) unsafe extern "C" fn noop_free_viewing_keys(
    _ctx: *mut std::ffi::c_void,
    _entries: *const crate::shielded_persistence::ShieldedViewingKeyRestoreFFI,
    _count: usize,
) {
}

/// Runs the test `test_fn` of the module `module` (its `module_path!()`) alone
/// in a child process of this test binary, with `child_env` set so the test
/// runs its scenario instead of re-spawning, and fails unless the child passes
/// within `wait`.
///
/// For scenarios whose regression is a real deadlock: no timeout can unwind
/// its threads, and in the test process they would hold the process-wide
/// handle registries and wedge every other registry-backed test. The child
/// is killed instead.
pub(crate) fn assert_scenario_passes_in_child(
    module: &str,
    test_fn: &str,
    child_env: &str,
    wait: std::time::Duration,
) {
    let module_below_crate = module
        .split_once("::")
        .map(|(_, path)| path)
        .expect("test module path below the crate root");
    let test_path = format!("{module_below_crate}::{test_fn}");
    let mut child = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", &test_path, "--nocapture"])
        .env(child_env, "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the scenario child");

    let deadline = std::time::Instant::now() + wait;
    let status = loop {
        if let Some(status) = child.try_wait().expect("child status") {
            break Some(status);
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("reap the scenario child");
    let transcript = format!(
        "--- child stdout ---\n{}\n--- child stderr ---\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    match status {
        Some(status) => assert!(
            status.success(),
            "the scenario failed in the child process ({status})\n{transcript}"
        ),
        None => {
            panic!("the scenario deadlocked: the child was killed after {wait:?}\n{transcript}")
        }
    }
}
