//! The external-identity top-up export: its argument guards, and an identity
//! Platform does not know reported as code 7 through the real wallet handle.

use std::ptr::NonNull;
use std::sync::Arc;

use dpp::address_funds::AddressWitness;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::ProtocolError;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use super::platform_wallet_top_up_external_identity_from_addresses_with_signer as top_up;
use crate::error::PlatformWalletFFIResultCode;
use crate::handle::PLATFORM_WALLET_STORAGE;
use crate::identity_registration::IdentityFundingInputFFI;
use crate::runtime::runtime;
use crate::test_fixtures::mock_manager;

const IDENTITY: [u8; 32] = [9; 32];
const UNTOUCHED: u64 = 0xDEAD;

fn input(address_type: u8) -> IdentityFundingInputFFI {
    IdentityFundingInputFFI {
        address_type,
        hash: [7; 20],
        credits: 100_000_000,
    }
}

/// Never asked to sign: every scenario here ends before signing.
#[derive(Debug)]
struct UnusedSigner;

#[async_trait::async_trait]
impl Signer<IdentityPublicKey> for UnusedSigner {
    async fn sign(&self, _: &IdentityPublicKey, _: &[u8]) -> Result<BinaryData, ProtocolError> {
        Err(ProtocolError::Generic("not expected to sign".into()))
    }

    async fn sign_create_witness(
        &self,
        _: &IdentityPublicKey,
        _: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Err(ProtocolError::Generic("not expected to sign".into()))
    }

    fn can_sign_with(&self, _: &IdentityPublicKey) -> bool {
        false
    }
}

fn dangling_signer() -> *mut SignerHandle {
    NonNull::<SignerHandle>::dangling().as_ptr()
}

#[test]
fn should_refuse_null_pointers() {
    let id = IDENTITY;
    let inputs = [input(0)];
    let mut balance = UNTOUCHED;
    let calls = [
        unsafe {
            top_up(
                0,
                std::ptr::null(),
                inputs.as_ptr(),
                1,
                dangling_signer(),
                &mut balance,
            )
        },
        unsafe { top_up(0, &id, std::ptr::null(), 1, dangling_signer(), &mut balance) },
        unsafe {
            top_up(
                0,
                &id,
                inputs.as_ptr(),
                1,
                std::ptr::null_mut(),
                &mut balance,
            )
        },
        unsafe {
            top_up(
                0,
                &id,
                inputs.as_ptr(),
                1,
                dangling_signer(),
                std::ptr::null_mut(),
            )
        },
    ];
    for result in calls {
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }
    assert_eq!(balance, UNTOUCHED);
}

#[test]
fn should_refuse_empty_inputs_and_an_unknown_address_type() {
    let id = IDENTITY;
    let mut balance = UNTOUCHED;
    let empty = unsafe {
        top_up(
            0,
            &id,
            [input(0)].as_ptr(),
            0,
            dangling_signer(),
            &mut balance,
        )
    };
    let unknown_type = unsafe {
        top_up(
            0,
            &id,
            [input(2)].as_ptr(),
            1,
            dangling_signer(),
            &mut balance,
        )
    };
    assert_eq!(
        empty.code,
        PlatformWalletFFIResultCode::ErrorInvalidParameter
    );
    assert_eq!(
        unknown_type.code,
        PlatformWalletFFIResultCode::ErrorInvalidParameter
    );
    assert_eq!(balance, UNTOUCHED);
}

/// The identity is looked up on Platform, not in the wallet: an identity
/// Platform does not know is `ErrorIdentityNotFound` (7), the balance out
/// is not written, and the wallet still manages no identity.
#[test]
fn should_report_an_identity_platform_does_not_know_as_not_found() {
    let manager = mock_manager();
    let mut sdk = manager.sdk().clone();
    runtime()
        .block_on(
            sdk.mock()
                .expect_fetch::<Identity, _>(Identifier::from(IDENTITY), None),
        )
        .expect("the fetch expectation");
    let wallet = runtime()
        .block_on(manager.create_wallet_from_seed_bytes(
            key_wallet::Network::Testnet,
            &[42; 64],
            WalletAccountCreationOptions::None,
            Some(0),
        ))
        .expect("a wallet");
    let handle = PLATFORM_WALLET_STORAGE.insert(wallet.clone());
    let signer = Box::into_raw(Box::new(VTableSigner::from_native(Arc::new(UnusedSigner))))
        as *mut SignerHandle;

    let id = IDENTITY;
    let mut balance = UNTOUCHED;
    let result = unsafe { top_up(handle, &id, [input(0)].as_ptr(), 1, signer, &mut balance) };

    let _ = PLATFORM_WALLET_STORAGE.remove(handle);
    drop(unsafe { Box::from_raw(signer as *mut VTableSigner) });
    assert_eq!(
        result.code,
        PlatformWalletFFIResultCode::ErrorIdentityNotFound
    );
    assert_eq!(balance, UNTOUCHED);
    let identities = runtime().block_on(async {
        wallet
            .wallet_manager()
            .read()
            .await
            .get_wallet_info(&wallet.wallet_id())
            .expect("the wallet's info")
            .identity_manager
            .identity_count()
    });
    assert_eq!(identities, 0);
}
