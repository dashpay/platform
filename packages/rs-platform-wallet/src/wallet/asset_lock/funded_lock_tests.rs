//! Flow-level tests of the funded-lock seam
//! ([`AssetLockManager::with_funded_lock`]): once an asset-lock-funded
//! operation holds its Core lock, every failure names the lock as held and is
//! never `NotSent`.
//!
//! Each flow resumes a tracked, chain-locked lock (so no broadcast is needed)
//! against a mock SDK with no expectations, so the Platform submit fails. A
//! flow that bypassed the seam would report the submit's own class
//! (`NotSent` for a signer refusal, `MaybeSent` for a transport error), and
//! its test fails.

use std::collections::BTreeMap;

use dashcore::OutPoint;
use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::{AssetLockProof, Identifier};
use dpp::ProtocolError;
use key_wallet::account::account_collection::PlatformPaymentAccountKey;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::error::{PlatformWalletError, SIGNER_KEY_UNAVAILABLE_PREFIX};
use crate::test_support::{
    fund_test_platform_wallet, test_platform_wallet_manager, test_pubkey, WalletSigner,
};
use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
use crate::wallet::asset_lock::AssetLockFunding;
use crate::wallet::PlatformWallet;
use crate::PaymentDelivery;

/// A signer that has no key for anything it is asked to sign: the SDK's
/// "key unavailable" refusal, which on its own is `NotSent`.
#[derive(Debug)]
struct KeylessSigner;

fn key_unavailable() -> ProtocolError {
    ProtocolError::Generic(format!(
        "{SIGNER_KEY_UNAVAILABLE_PREFIX}no key in this test"
    ))
}

#[async_trait::async_trait]
impl dpp::identity::signer::Signer<IdentityPublicKey> for KeylessSigner {
    async fn sign(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        Err(key_unavailable())
    }

    async fn sign_create_witness(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Err(key_unavailable())
    }

    fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
        true
    }
}

#[async_trait::async_trait]
impl dpp::identity::signer::Signer<PlatformAddress> for KeylessSigner {
    async fn sign(
        &self,
        _key: &PlatformAddress,
        _data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        Err(key_unavailable())
    }

    async fn sign_create_witness(
        &self,
        _key: &PlatformAddress,
        _data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Err(key_unavailable())
    }

    fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
        true
    }
}

/// A funded test wallet and a signer over its seed.
async fn wallet() -> (
    std::sync::Arc<crate::PlatformWalletManager<crate::test_support::NoopTestPersister>>,
    std::sync::Arc<PlatformWallet>,
    WalletSigner,
) {
    let (manager, wallet_id) = test_platform_wallet_manager().await;
    let wallet = manager.get_wallet(&wallet_id).await.expect("wallet handle");
    let signer = fund_test_platform_wallet(wallet.wallet_manager(), &wallet_id, 50_000_000).await;
    (manager, wallet, signer)
}

/// Track a chain-locked lock of `funding_type` at `identity_index`, built
/// from the wallet's own funding account so its credit-output path
/// re-derives on resume. Returns its outpoint.
async fn track_chain_locked_lock(
    wallet: &PlatformWallet,
    signer: &WalletSigner,
    funding_type: AssetLockFundingType,
    identity_index: u32,
) -> OutPoint {
    let (transaction, _path) = wallet
        .asset_locks
        .build_asset_lock_transaction(1_000_000, 0, funding_type, identity_index, signer)
        .await
        .expect("build the asset lock");
    let out_point = OutPoint::new(transaction.txid(), 0);
    let mut wm = wallet.wallet_manager().write().await;
    wm.get_wallet_info_mut(&wallet.wallet_id())
        .expect("wallet registered")
        .tracked_asset_locks
        .insert(
            out_point,
            TrackedAssetLock {
                operation_id: None,
                shielded_recipient: None,
                out_point,
                transaction,
                account_index: 0,
                funding_type,
                identity_index,
                amount: 1_000_000,
                status: AssetLockStatus::ChainLocked,
                proof: Some(AssetLockProof::Chain(ChainAssetLockProof {
                    core_chain_locked_height: 42,
                    out_point,
                })),
            },
        );
    out_point
}

fn resume(out_point: OutPoint) -> AssetLockFunding {
    AssetLockFunding::FromExistingAssetLock {
        out_point,
        consume_invitation_voucher: false,
    }
}

fn master_key() -> IdentityPublicKey {
    IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::MASTER,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(test_pubkey().to_bytes()),
        disabled_at: None,
    })
}

/// The class every funded flow must end with here: the resumed lock, held.
fn assert_holds(error: &PlatformWalletError, out_point: OutPoint) {
    assert_eq!(
        error.payment_delivery(),
        PaymentDelivery::LockHeld(out_point),
        "{error:?}"
    );
    assert!(!error.is_retryable(), "{error:?}");
}

#[tokio::test]
async fn should_hold_the_lock_when_a_resumed_registration_fails_at_platform() {
    let (_manager, wallet, signer) = wallet().await;
    let out_point = track_chain_locked_lock(
        &wallet,
        &signer,
        AssetLockFundingType::IdentityRegistration,
        0,
    )
    .await;

    let error = wallet
        .identity()
        .register_identity_with_funding(
            resume(out_point),
            0,
            BTreeMap::from([(0, master_key())]),
            &KeylessSigner,
            &signer,
            None,
        )
        .await
        .expect_err("the mock SDK answers nothing");

    assert_holds(&error, out_point);
}

#[tokio::test]
async fn should_hold_the_lock_when_a_resumed_top_up_fails_at_platform() {
    const INDEX: u32 = 3;
    let (_manager, wallet, signer) = wallet().await;
    let identity = Identity::V0(IdentityV0 {
        id: Identifier::from([4; 32]),
        public_keys: BTreeMap::from([(0, master_key())]),
        balance: 0,
        revision: 0,
    });
    {
        let mut wm = wallet.wallet_manager().write().await;
        wm.get_wallet_info_mut(&wallet.wallet_id())
            .expect("wallet registered")
            .identity_manager
            .add_identity_unstored(identity, INDEX, wallet.wallet_id())
            .expect("add the identity");
    }
    let out_point =
        track_chain_locked_lock(&wallet, &signer, AssetLockFundingType::IdentityTopUp, INDEX).await;

    let error = wallet
        .identity()
        .top_up_identity_with_funding(&Identifier::from([4; 32]), resume(out_point), &signer, None)
        .await
        .expect_err("the mock SDK answers nothing");

    assert_holds(&error, out_point);
}

#[tokio::test]
async fn should_hold_the_lock_when_a_resumed_address_funding_fails_at_platform() {
    let (_manager, wallet, signer) = wallet().await;
    let address = wallet
        .platform()
        .next_unused_receive_address(PlatformPaymentAccountKey {
            account: 0,
            key_class: 0,
        })
        .await
        .expect("owned platform payment address");
    let out_point = track_chain_locked_lock(
        &wallet,
        &signer,
        AssetLockFundingType::AssetLockAddressTopUp,
        0,
    )
    .await;

    let error = wallet
        .platform()
        .fund_from_asset_lock(
            resume(out_point),
            0,
            BTreeMap::from([(address, None)]),
            vec![],
            &KeylessSigner,
            &signer,
            None,
            None,
        )
        .await
        .expect_err("the mock SDK answers nothing");

    assert_holds(&error, out_point);
}

/// The seam's own boundaries, on a real tracked row: a lock that is consumed,
/// consumption-unknown or gone is not named (the error is `MaybeSent`), and a
/// resume refused at its entry keeps its class.
#[tokio::test]
async fn should_name_only_a_tracked_unconsumed_lock() {
    let (_manager, wallet, signer) = wallet().await;
    let out_point = track_chain_locked_lock(
        &wallet,
        &signer,
        AssetLockFundingType::IdentityRegistration,
        0,
    )
    .await;
    let refused = || PlatformWalletError::Sdk(dash_sdk::Error::Protocol(key_unavailable()));
    assert_eq!(refused().payment_delivery(), PaymentDelivery::NotSent);
    let locks = &wallet.asset_locks;

    let held = locks.funded_lock_error(refused(), Some(out_point)).await;
    assert_holds(&held, out_point);
    assert_eq!(held.to_string(), refused().to_string());
    assert!(matches!(
        held.dispatched_source(),
        PlatformWalletError::Sdk(_)
    ));

    let transport = PlatformWalletError::Sdk(dash_sdk::Error::Generic("transport".into()));
    assert_holds(
        &locks.funded_lock_error(transport, Some(out_point)).await,
        out_point,
    );

    let consumed = locks
        .funded_lock_error(
            PlatformWalletError::AssetLockAlreadyConsumed(out_point),
            Some(out_point),
        )
        .await;
    assert!(matches!(
        consumed,
        PlatformWalletError::AssetLockAlreadyConsumed(_)
    ));

    let untracked = OutPoint::new(dashcore::Txid::from_byte_array([1; 32]), 9);
    let not_named = locks.funded_lock_error(refused(), Some(untracked)).await;
    assert_eq!(not_named.payment_delivery(), PaymentDelivery::MaybeSent);
    assert_eq!(
        locks
            .funded_lock_error(refused(), None)
            .await
            .payment_delivery(),
        PaymentDelivery::MaybeSent
    );
    assert_eq!(
        locks
            .resume_error(
                PlatformWalletError::AssetLockNotTracked(untracked),
                &untracked
            )
            .await
            .payment_delivery(),
        PaymentDelivery::NotSent,
        "a resume refused at its entry dispatched nothing"
    );
    assert_holds(&locks.resume_error(refused(), &out_point).await, out_point);

    for status in [
        AssetLockStatus::Consumed,
        AssetLockStatus::DrainCompleted,
        AssetLockStatus::RecoveredFromChain,
    ] {
        {
            let mut wm = wallet.wallet_manager().write().await;
            wm.get_wallet_info_mut(&wallet.wallet_id())
                .expect("wallet registered")
                .tracked_asset_locks
                .get_mut(&out_point)
                .expect("tracked")
                .status = status.clone();
        }
        let error = locks.funded_lock_error(refused(), Some(out_point)).await;
        assert_eq!(
            error.payment_delivery(),
            PaymentDelivery::MaybeSent,
            "{status:?}"
        );
    }
}

/// Once a shielded funding holds its Core lock, a refusal before any Platform
/// dispatch (here the lock is too small to pay the pool fee) names the lock as
/// held: the funded-lock seam covers the sizing checks that follow the lock's
/// resolution, not only the submit.
#[cfg(feature = "shielded")]
#[tokio::test]
async fn should_hold_the_lock_when_a_resumed_shielded_funding_cannot_cover_its_fee() {
    use crate::wallet::shielded::{
        prover::CachedOrchardProver, FileBackedShieldedStore, NetworkShieldedCoordinator,
    };
    use dpp::balances::credits::CREDITS_PER_DUFF;

    let (_manager, wallet, signer) = wallet().await;
    let dir = std::env::temp_dir().join(format!("funded_lock_{}", rand::random::<u64>()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("shielded.sqlite");
    let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
    let coordinator = std::sync::Arc::new(NetworkShieldedCoordinator::new(
        wallet.sdk_arc(),
        dashcore::Network::Testnet,
        path,
        store,
    ));
    let pool_fee = wallet.shield_from_asset_lock_pool_fee(2).unwrap();
    let out_point = track_chain_locked_lock(
        &wallet,
        &signer,
        AssetLockFundingType::AssetLockShieldedAddressTopUp,
        0,
    )
    .await;
    // The value a chain-proven lock funds is its row's recorded amount:
    // record the pool fee, so the funding has nothing left to shield.
    {
        let mut wm = wallet.wallet_manager().write().await;
        let lock = wm
            .get_wallet_info_mut(&wallet.wallet_id())
            .expect("wallet registered")
            .tracked_asset_locks
            .get_mut(&out_point)
            .expect("tracked");
        lock.amount = pool_fee / CREDITS_PER_DUFF;
    }

    let error = wallet
        .shielded_fund_from_asset_lock(
            &coordinator,
            resume(out_point),
            vec![(crate::test_support::orchard_address_fixture(0x58), None)],
            &signer,
            &CachedOrchardProver::new(),
            None,
            0,
            None,
            None,
            None,
        )
        .await
        .expect_err("the lock cannot pay the pool fee");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        matches!(
            error.dispatched_source(),
            PlatformWalletError::ShieldedBuildError(_)
        ),
        "{error:?}"
    );
    assert_holds(&error, out_point);
}
