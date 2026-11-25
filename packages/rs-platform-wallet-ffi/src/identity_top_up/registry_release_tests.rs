//! Every address top-up export waits for the host signer without holding the
//! wallet handle registry: while a signature is pending, destroying another
//! wallet handle and reading a balance must both complete.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::c_void;
use std::os::raw::c_char;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use dash_sdk::query_types::{AddressInfo, AddressInfos};
use dash_sdk::SdkBuilder;
use dpp::address_funds::PlatformAddress;
use dpp::identity::{Identity, IdentityV0};
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::PlatformWalletManager;
use rs_sdk_ffi::{
    dash_sdk_signer_create_with_ctx, dash_sdk_signer_destroy, SignCompletionCallback, SignerHandle,
};

use super::{
    platform_wallet_top_up_external_identity_from_addresses_with_signer as top_up_external,
    platform_wallet_top_up_from_addresses_with_signer as top_up_managed,
    platform_wallet_top_up_identity_from_addresses_with_signer as top_up_any,
};
use crate::error::PlatformWalletFFIResult;
use crate::handle::{Handle, PLATFORM_WALLET_STORAGE};
use crate::identity_registration::IdentityFundingInputFFI;
use crate::runtime::runtime;
use crate::wallet::{platform_wallet_destroy, platform_wallet_get_balance};

const IDENTITY: [u8; 32] = [9; 32];
const ADDRESS_HASH: [u8; 20] = [7; 20];
const AMOUNT: u64 = 100_000_000;
/// Long enough for a loaded CI machine to reach the signer, short enough to
/// fail instead of hanging when the registry is held.
const SIGNER_REACHED: Duration = Duration::from_secs(60);
const REGISTRY_FREE: Duration = Duration::from_secs(5);

struct NoopEvents;
impl EventHandler for NoopEvents {}
impl PlatformEventHandler for NoopEvents {}

/// A pending signature request the host has not answered yet.
struct PendingSignature {
    completion_ctx: usize,
    completion: SignCompletionCallback,
}

struct SignerContext {
    requests: Mutex<Sender<PendingSignature>>,
}

unsafe extern "C" fn sign_async(
    signer: *const c_void,
    _pubkey_bytes: *const u8,
    _pubkey_len: usize,
    _key_type: u8,
    _data: *const u8,
    _data_len: usize,
    completion_ctx: *mut c_void,
    completion: SignCompletionCallback,
) {
    let context = &*(signer as *const SignerContext);
    let _ = context.requests.lock().unwrap().send(PendingSignature {
        completion_ctx: completion_ctx as usize,
        completion,
    });
}

unsafe extern "C" fn can_sign_with(_: *const c_void, _: *const u8, _: usize, _: u8) -> bool {
    true
}

unsafe extern "C" fn destroy(signer: *mut c_void) {
    drop(Box::from_raw(signer as *mut SignerContext));
}

fn pending_signer() -> (
    *mut SignerHandle,
    std::sync::mpsc::Receiver<PendingSignature>,
) {
    let (requests, pending) = channel();
    let context = Box::into_raw(Box::new(SignerContext {
        requests: Mutex::new(requests),
    }));
    let signer = unsafe {
        dash_sdk_signer_create_with_ctx(
            context as *mut c_void,
            sign_async,
            can_sign_with,
            Some(destroy),
        )
    };
    (signer, pending)
}

/// The wallet the export runs on (funded address, identity known both to the
/// wallet and to the mock Platform) and a second wallet to destroy.
async fn wallets() -> (Handle, Handle) {
    let mut sdk = SdkBuilder::new_mock()
        .with_version(PlatformVersion::latest())
        .with_time_tolerance(None)
        .build()
        .unwrap();
    let address = PlatformAddress::P2pkh(ADDRESS_HASH);
    sdk.mock()
        .expect_fetch_many::<PlatformAddress, AddressInfo, _, AddressInfos>(
            BTreeSet::from([address]),
            Some(AddressInfos::from_iter([(
                address,
                Some(AddressInfo {
                    address,
                    nonce: 0,
                    balance: AMOUNT,
                }),
            )])),
        )
        .await
        .unwrap();
    let identity = Identity::V0(IdentityV0 {
        id: Identifier::from(IDENTITY),
        public_keys: BTreeMap::new(),
        balance: 0,
        revision: 0,
    });
    sdk.mock()
        .expect_fetch::<Identity, _>(Identifier::from(IDENTITY), Some(identity.clone()))
        .await
        .unwrap();
    let manager = PlatformWalletManager::new(
        Arc::new(sdk),
        Arc::new(NoPlatformPersistence),
        Arc::new(NoopEvents),
    );
    let mut handles = Vec::new();
    for seed in [42u8, 43] {
        let wallet = manager
            .create_wallet_from_seed_bytes(
                key_wallet::Network::Testnet,
                &[seed; 64],
                WalletAccountCreationOptions::None,
                Some(0),
            )
            .await
            .unwrap();
        if seed == 42 {
            wallet
                .wallet_manager()
                .write()
                .await
                .get_wallet_info_mut(&wallet.wallet_id())
                .unwrap()
                .identity_manager
                .add_identity(identity.clone(), 0, wallet.wallet_id(), wallet.persister())
                .unwrap();
        }
        handles.push(PLATFORM_WALLET_STORAGE.insert(wallet));
    }
    (handles[0], handles[1])
}

type Export = unsafe extern "C" fn(
    Handle,
    *const [u8; 32],
    *const IdentityFundingInputFFI,
    usize,
    *mut SignerHandle,
    *mut u64,
) -> PlatformWalletFFIResult;

/// Holds the signer while the export waits on it, destroys the second wallet
/// handle and reads a balance, and reports whether both finished before the
/// signer was answered.
fn registry_is_free_while_the_signer_is_pending(export: Export) {
    let (export_handle, other_handle) = runtime().block_on(wallets());
    let (signer, pending) = pending_signer();

    let signer_addr = signer as usize;
    let exporting = thread::spawn(move || {
        let row = IdentityFundingInputFFI {
            address_type: 0,
            hash: ADDRESS_HASH,
            credits: AMOUNT,
        };
        let id = IDENTITY;
        let mut balance = 0u64;
        unsafe {
            export(
                export_handle,
                &id,
                &row,
                1,
                signer_addr as *mut SignerHandle,
                &mut balance,
            )
        };
    });
    let request = pending
        .recv_timeout(SIGNER_REACHED)
        .expect("the export never asked the signer for a signature");

    let (finished, finishes) = channel();
    let destroyed = finished.clone();
    thread::spawn(move || {
        unsafe { platform_wallet_destroy(other_handle) };
        let _ = destroyed.send("destroy");
    });
    thread::spawn(move || {
        let mut confirmed = 0u64;
        unsafe {
            platform_wallet_get_balance(
                export_handle,
                &mut confirmed,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        let _ = finished.send("balance read");
    });
    let mut done_before_release = Vec::new();
    while done_before_release.len() < 2 {
        match finishes.recv_timeout(REGISTRY_FREE) {
            Ok(name) => done_before_release.push(name),
            Err(_) => break,
        }
    }

    // Answer the signer whatever happened, so no thread outlives the test.
    let signature = [0u8; 65];
    unsafe {
        (request.completion)(
            request.completion_ctx as *mut c_void,
            signature.as_ptr(),
            signature.len(),
            0,
            std::ptr::null::<c_char>(),
        )
    };
    exporting.join().unwrap();
    for _ in done_before_release.len()..2 {
        finishes.recv_timeout(SIGNER_REACHED).unwrap();
    }
    let _ = PLATFORM_WALLET_STORAGE.remove(export_handle);
    unsafe { dash_sdk_signer_destroy(signer) };

    assert_eq!(
        done_before_release.len(),
        2,
        "only {done_before_release:?} finished while the export waited for the signer: \
         the wallet registry stayed locked"
    );
}

#[test]
fn should_not_hold_the_wallet_registry_while_the_top_up_of_any_identity_waits_for_the_signer() {
    registry_is_free_while_the_signer_is_pending(top_up_any);
}

#[test]
fn should_not_hold_the_wallet_registry_while_the_external_top_up_waits_for_the_signer() {
    registry_is_free_while_the_signer_is_pending(top_up_external);
}

#[test]
fn should_not_hold_the_wallet_registry_while_the_managed_top_up_waits_for_the_signer() {
    registry_is_free_while_the_signer_is_pending(top_up_managed);
}
