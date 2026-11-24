//! What a spend from the shielded credit pool holds while it runs
//! (`PlatformWallet::admit_shielded_spend`): the wallet generation's
//! lifecycle gate and the coordinator's operation admission, taken at every
//! entry point, with the binding checked, before anything is read, signed
//! or sent.

use std::sync::Arc;
use std::time::Duration;

use dashcore::Network;
use dpp::address_funds::PlatformAddress;
use dpp::prelude::Identifier;
use dpp::shielded::builder::OrchardProver;
use grovedb_commitment_tree::ProvingKey;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use simple_signer::signer::SimpleSigner;

use super::viewing_key_bind_tests::{coordinator_at, CapturingPersistence};
use super::{NetworkShieldedCoordinator, OrchardKeySet};
use crate::error::PlatformWalletError;
use crate::events::{EventHandler, PlatformEventHandler};
use crate::wallet::platform_wallet::PlatformWallet;
use crate::PlatformWalletManager;

const SEED: [u8; 64] = [0x42; 64];

/// How long a spawned spend gets to reach the point a test holds it at.
const REACHED: Duration = Duration::from_secs(5);

struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

/// A prover none of these spends reaches: they are refused, held, or fail
/// on an empty wallet before anything is proved.
struct UnreachableProver;
impl OrchardProver for UnreachableProver {
    fn proving_key(&self) -> &ProvingKey {
        panic!("nothing is proved")
    }
}

/// The spends from the credit pool, one per wallet entry point.
#[derive(Clone, Copy, Debug)]
enum CreditPoolSpend {
    Transfer,
    Unshield,
    IdentityTopUp,
    Withdraw,
    IdentityCreate,
    ShieldFromAccount,
    ShieldFromIdentity,
}

const CREDIT_POOL_SPENDS: [CreditPoolSpend; 7] = [
    CreditPoolSpend::Transfer,
    CreditPoolSpend::Unshield,
    CreditPoolSpend::IdentityTopUp,
    CreditPoolSpend::Withdraw,
    CreditPoolSpend::IdentityCreate,
    CreditPoolSpend::ShieldFromAccount,
    CreditPoolSpend::ShieldFromIdentity,
];

type Manager = PlatformWalletManager<CapturingPersistence>;

/// A manager on the coordinators' network, with a persister a bind accepts.
fn manager() -> Arc<Manager> {
    let sdk = dash_sdk::SdkBuilder::new_mock()
        .with_network(Network::Testnet)
        .build()
        .expect("mock sdk");
    Arc::new(PlatformWalletManager::new(
        Arc::new(sdk),
        Arc::new(CapturingPersistence::default()),
        Arc::new(NoopEventHandler),
    ))
}

async fn add_wallet(manager: &Manager) -> Arc<PlatformWallet> {
    manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            &SEED,
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("wallet")
}

/// A wallet of `manager` with shielded account 0 bound on `coordinator`.
async fn add_bound_wallet(
    manager: &Manager,
    coordinator: &Arc<NetworkShieldedCoordinator>,
) -> Arc<PlatformWallet> {
    let wallet = add_wallet(manager).await;
    wallet
        .bind_shielded(&SEED, &[0], coordinator)
        .await
        .expect("bound");
    wallet
}

/// Run `spend` on `wallet`'s account 0, with arguments that pass what each
/// entry point checks before it admits the spend.
async fn spend_from_credit_pool(
    spend: CreditPoolSpend,
    wallet: &PlatformWallet,
    coordinator: &Arc<NetworkShieldedCoordinator>,
) -> Result<(), PlatformWalletError> {
    let network = wallet.network();
    let signer = SimpleSigner::default();
    let identity_id = Identifier::from([0xA1; 32]);
    let platform_address = PlatformAddress::P2pkh([0x11; 20]);
    match spend {
        CreditPoolSpend::Transfer => {
            let recipient = OrchardKeySet::from_seed(&[0x43; 64], network, 0)
                .expect("recipient keys")
                .default_address
                .to_raw_address_bytes();
            wallet
                .shielded_transfer_to(
                    coordinator,
                    &SEED,
                    0,
                    &recipient,
                    1_000,
                    [0; 36],
                    UnreachableProver,
                )
                .await
        }
        CreditPoolSpend::Unshield => {
            wallet
                .shielded_unshield_to(
                    coordinator,
                    &SEED,
                    0,
                    &platform_address.to_bech32m_string(network),
                    1_000,
                    UnreachableProver,
                )
                .await
        }
        CreditPoolSpend::IdentityTopUp => wallet
            .shielded_identity_top_up_from_pool(
                coordinator,
                &SEED,
                0,
                &identity_id,
                1_000,
                UnreachableProver,
            )
            .await
            .map(|_| ()),
        CreditPoolSpend::Withdraw => {
            wallet
                .shielded_withdraw_to(
                    coordinator,
                    &SEED,
                    0,
                    &dashcore::Address::dummy(network, 1).to_string(),
                    1_000,
                    1,
                    UnreachableProver,
                )
                .await
        }
        CreditPoolSpend::IdentityCreate => wallet
            .shielded_identity_create_from_pool(
                coordinator,
                &SEED,
                0,
                0,
                Vec::new(),
                1_000,
                platform_address,
                &signer,
                UnreachableProver,
            )
            .await
            .map(|_| ()),
        CreditPoolSpend::ShieldFromAccount => {
            wallet
                .shielded_shield_from_account(coordinator, 0, 0, 1_000, &signer, UnreachableProver)
                .await
        }
        CreditPoolSpend::ShieldFromIdentity => wallet
            .shielded_shield_from_identity(
                coordinator,
                0,
                &identity_id,
                1_000,
                &signer,
                UnreachableProver,
            )
            .await
            .map(|_| ()),
    }
}

/// Wait until an operation holds `coordinator`'s admission: the spawned
/// spend has passed its liveness check and been admitted.
async fn admitted(coordinator: &NetworkShieldedCoordinator, what: impl std::fmt::Debug) {
    let deadline = tokio::time::Instant::now() + REACHED;
    while !coordinator.has_admitted_operation() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "{what:?}: never admitted"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// `true` while a removal of `wallet` would wait: its generation's teardown
/// side cannot be taken.
async fn removal_waits(wallet: &PlatformWallet) -> bool {
    tokio::time::timeout(
        Duration::from_millis(100),
        wallet.generation().teardown_guard(),
    )
    .await
    .is_err()
}

/// A handle to a wallet removed, or re-created under the same id, since it
/// was taken spends nothing from the credit pool, although its shielded keys
/// are still on it: every entry point refuses it at the liveness check,
/// before it reads a key, reserves a note or records activity.
#[tokio::test]
async fn should_refuse_a_credit_pool_spend_from_a_wallet_no_longer_current() {
    let dir = tempfile::tempdir().expect("temporary shielded store");
    let coordinator = coordinator_at(dir.path());
    let manager = manager();
    let removed = add_bound_wallet(&manager, &coordinator).await;
    let wallet_id = removed.wallet_id();
    manager.remove_wallet(&wallet_id).await.expect("removed");
    for spend in CREDIT_POOL_SPENDS {
        let refused = spend_from_credit_pool(spend, &removed, &coordinator).await;
        assert!(
            matches!(&refused, Err(PlatformWalletError::WalletRemovedBeforeSend(id))
                if *id == hex::encode(wallet_id)),
            "{spend:?}: {refused:?}"
        );
    }

    let recreated = add_wallet(&manager).await;
    assert_eq!(recreated.wallet_id(), wallet_id);
    for spend in CREDIT_POOL_SPENDS {
        let refused = spend_from_credit_pool(spend, &removed, &coordinator).await;
        assert!(
            matches!(
                refused,
                Err(PlatformWalletError::WalletRemovedBeforeSend(_))
            ),
            "{spend:?}: {refused:?}"
        );
        // The re-created wallet is current, and not bound yet.
        let current = spend_from_credit_pool(spend, &recreated, &coordinator).await;
        assert!(
            matches!(current, Err(PlatformWalletError::ShieldedNotBound)),
            "{spend:?}: {current:?}"
        );
    }
    assert!(
        !coordinator.has_admitted_operation(),
        "a refused spend holds no admission"
    );
}

/// A Clear drops the wallet's registration but not the keys its handle
/// holds, and a shield needs no note from the cleared store. A credit pool
/// spend admitted after a Clear is therefore refused before it reads a key
/// or signs, until the wallet is bound again.
#[tokio::test]
async fn should_refuse_a_credit_pool_spend_after_a_clear_until_the_wallet_is_bound_again() {
    let dir = tempfile::tempdir().expect("temporary shielded store");
    let coordinator = coordinator_at(dir.path());
    let manager = manager();
    let wallet = add_bound_wallet(&manager, &coordinator).await;

    coordinator.clear().await.expect("cleared");
    assert!(
        wallet.is_shielded_bound().await,
        "the handle keeps its keys across a Clear"
    );
    for spend in CREDIT_POOL_SPENDS {
        let refused = spend_from_credit_pool(spend, &wallet, &coordinator).await;
        assert!(
            matches!(refused, Err(PlatformWalletError::ShieldedNotBound)),
            "{spend:?}: {refused:?}"
        );
    }

    wallet
        .bind_shielded(&SEED, &[0], &coordinator)
        .await
        .expect("bound again");
    for spend in CREDIT_POOL_SPENDS {
        let admitted = spend_from_credit_pool(spend, &wallet, &coordinator).await;
        assert!(
            !matches!(
                admitted,
                Err(PlatformWalletError::ShieldedNotBound
                    | PlatformWalletError::WalletRemovedBeforeSend(_))
            ),
            "{spend:?}: a wallet bound again is admitted: {admitted:?}"
        );
    }
}

/// Every credit pool spend that was admitted holds the wallet generation's
/// lifecycle gate and the coordinator's operation admission until it
/// returns: a removal of the wallet waits for it, and a Clear finds an
/// operation admitted. Each spend is held here at the first wallet lock it
/// takes after its admission.
#[tokio::test]
async fn should_hold_removal_and_clear_admission_until_a_credit_pool_spend_returns() {
    for spend in CREDIT_POOL_SPENDS {
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let coordinator = coordinator_at(dir.path());
        let manager = manager();
        let wallet = add_bound_wallet(&manager, &coordinator).await;

        let single_flight = wallet.shield_guard.lock().await;
        let keys = wallet.shielded_keys.write().await;
        let running = {
            let wallet = Arc::clone(&wallet);
            let coordinator = Arc::clone(&coordinator);
            tokio::spawn(async move { spend_from_credit_pool(spend, &wallet, &coordinator).await })
        };
        admitted(&coordinator, spend).await;
        assert!(
            removal_waits(&wallet).await,
            "{spend:?}: a removal waits for the spend in flight"
        );
        assert!(!running.is_finished(), "{spend:?}: held at a wallet lock");
        // The configuration lock is taken for the binding check only: a
        // bind of this wallet, and another of its spends, do not wait for
        // the proof and the result of this one.
        assert!(
            wallet.shielded_config_lock.try_lock().is_ok(),
            "{spend:?}: the configuration lock is free while the spend runs"
        );

        drop(keys);
        drop(single_flight);
        let returned = running.await.expect("spend task");
        assert!(
            !matches!(
                returned,
                Err(PlatformWalletError::WalletRemovedBeforeSend(_)
                    | PlatformWalletError::ShieldedNotBound)
            ),
            "{spend:?}: the wallet was current and bound when it was checked: {returned:?}"
        );
        assert!(
            !coordinator.has_admitted_operation(),
            "{spend:?}: the admission is released with the spend"
        );
        assert!(
            !removal_waits(&wallet).await,
            "{spend:?}: the removal goes on once the spend returned"
        );
    }
}

/// The admission outlives the entry point's own reads: a transfer held
/// inside the operation, at the shielded store it selects its notes from,
/// still excludes a removal and a Clear, and a removal issued meanwhile
/// completes only once the transfer has returned.
#[tokio::test]
async fn should_hold_a_wallets_removal_until_a_transfer_held_at_the_store_returns() {
    let dir = tempfile::tempdir().expect("temporary shielded store");
    let coordinator = coordinator_at(dir.path());
    let manager = manager();
    let wallet = add_bound_wallet(&manager, &coordinator).await;
    let wallet_id = wallet.wallet_id();

    let store = coordinator.store().write().await;
    let transfer = {
        let wallet = Arc::clone(&wallet);
        let coordinator = Arc::clone(&coordinator);
        tokio::spawn(async move {
            spend_from_credit_pool(CreditPoolSpend::Transfer, &wallet, &coordinator).await
        })
    };
    admitted(&coordinator, "transfer").await;
    // Past the key derivation, which takes and releases the key slot.
    let keys = tokio::time::timeout(REACHED, wallet.shielded_keys.write())
        .await
        .expect("the transfer released the key slot");
    drop(keys);
    let removal = {
        let manager = Arc::clone(&manager);
        tokio::spawn(async move { manager.remove_wallet(&wallet_id).await.map(|_| ()) })
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!transfer.is_finished(), "the transfer waits at the store");
    assert!(
        !removal.is_finished(),
        "the removal waits for the transfer in flight"
    );
    assert!(coordinator.has_admitted_operation());
    assert!(wallet.core().is_current_generation().await);

    drop(store);
    let returned = transfer.await.expect("transfer task");
    assert!(
        !matches!(
            returned,
            Err(PlatformWalletError::WalletRemovedBeforeSend(_)
                | PlatformWalletError::ShieldedNotBound)
        ),
        "the wallet was current and bound when it was checked: {returned:?}"
    );
    removal
        .await
        .expect("removal task")
        .expect("the removal completes");
    assert!(!wallet.core().is_current_generation().await);
}
