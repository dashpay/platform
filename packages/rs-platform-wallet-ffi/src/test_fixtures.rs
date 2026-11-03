//! Test-only fixtures shared by the crate's unit tests.

use std::ffi::CStr;
use std::ptr;
use std::sync::Arc;

use dpp::consensus::basic::identity::TooManyMasterPublicKeyError;
use dpp::identity::{KeyType, Purpose, SecurityLevel};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use platform_wallet::{IdentityCreationKeyLayout, PlatformWalletManager};

use crate::error::{
    platform_wallet_ffi_result_free, PlatformWalletFFIResult, PlatformWalletFFIResultCode,
};
use crate::event_handler::{EventHandlerCallbacks, FFIEventHandler};
use crate::handle::{Handle, PLATFORM_WALLET_STORAGE};
use crate::identity_registration_with_signer::IdentityPubkeyFFI;
use crate::persistence::{FFIPersister, PersistenceCallbacks};
use crate::runtime::runtime;

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

/// A wallet registered in the handle storage, with the manager keeping it
/// alive. No funds, and the mock SDK never touches the network.
pub(crate) fn live_wallet_handle() -> (PlatformWalletManager<FFIPersister>, Handle) {
    let manager = mock_manager();
    let wallet = runtime().block_on(async {
        manager
            .create_wallet_from_seed_bytes(
                dashcore::Network::Testnet,
                &[42; 64],
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("create mock wallet")
    });
    let handle = PLATFORM_WALLET_STORAGE.insert(wallet);
    (manager, handle)
}

const KEY_DATA: [[u8; 33]; 5] = [[2; 33], [3; 33], [4; 33], [5; 33], [6; 33]];

/// FFI rows of the `NewIdentity` layout plus a second MASTER key: every
/// wallet check passes, and only Platform's structure validation refuses them.
pub(crate) fn rows_with_two_master_keys() -> Vec<IdentityPubkeyFFI> {
    let mut roles: Vec<(u32, u8, u8, u8)> = IdentityCreationKeyLayout::NewIdentity
        .slots()
        .iter()
        .map(|slot| {
            (
                slot.key_id,
                slot.key_type as u8,
                slot.purpose as u8,
                slot.security_level as u8,
            )
        })
        .collect();
    roles.push((
        4,
        KeyType::ECDSA_SECP256K1 as u8,
        Purpose::AUTHENTICATION as u8,
        SecurityLevel::MASTER as u8,
    ));
    roles
        .into_iter()
        .map(
            |(key_id, key_type, purpose, security_level)| IdentityPubkeyFFI {
                key_id,
                key_type,
                purpose,
                security_level,
                pubkey_bytes: KEY_DATA[key_id as usize].as_ptr(),
                pubkey_len: KEY_DATA[key_id as usize].len(),
                read_only: false,
                contract_bounds_kind: 0,
                contract_bounds_id: ptr::null(),
                contract_bounds_document_type: ptr::null(),
                has_total_budget: false,
                total_budget: 0,
                has_expires_at: false,
                expires_at: 0,
            },
        )
        .collect()
}

/// Asserts `result` is the consensus refusal of [`rows_with_two_master_keys`],
/// then frees it. Any other code means the call got past the pre-flight.
pub(crate) fn assert_refused_for_two_master_keys(mut result: PlatformWalletFFIResult) {
    let message = if result.message.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    };
    assert_eq!(
        result.code,
        PlatformWalletFFIResultCode::ErrorInvalidParameter,
        "{message}"
    );
    let reason = TooManyMasterPublicKeyError::new().to_string();
    assert!(message.contains(&reason), "unexpected message: {message}");
    unsafe { platform_wallet_ffi_result_free(&mut result) };
}
