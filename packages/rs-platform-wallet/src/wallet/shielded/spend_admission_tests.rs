//! What a spend from the shielded credit pool holds while it runs
//! (`PlatformWallet::lock_for_shielded_operation`, then
//! `ShieldedOperationLocks::admit_spend`): the wallet generation's lifecycle
//! gate and the coordinator's admission, taken at every entry point, with the
//! wallet's registration checked, before anything is read, signed or sent.

use std::sync::Arc;
use std::time::Duration;

use dashcore::Network;
use dpp::address_funds::PlatformAddress;
use dpp::prelude::Identifier;
use dpp::shielded::builder::OrchardProver;
use grovedb_commitment_tree::ProvingKey;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use simple_signer::signer::SimpleSigner;

use super::viewing_key_bind_tests::CapturingPersistence;
use super::{NetworkShieldedCoordinator, OrchardKeySet};
use crate::error::PlatformWalletError;
use crate::events::{EventHandler, PlatformEventHandler};
use crate::test_support::WipeCountingPersistence;
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

/// A manager with a persister a bind accepts, its shielded store configured
/// under `dir`, and its coordinator: the one its wallets' spends run on.
async fn configured_manager(
    dir: &std::path::Path,
) -> (Arc<Manager>, Arc<NetworkShieldedCoordinator>) {
    let sdk = dash_sdk::SdkBuilder::new_mock()
        .with_network(Network::Testnet)
        .build()
        .expect("mock sdk");
    let manager = Arc::new(PlatformWalletManager::new(
        Arc::new(sdk),
        Arc::new(CapturingPersistence::default()),
        Arc::new(NoopEventHandler),
    ));
    manager
        .configure_shielded(dir.join("tree.sqlite"))
        .await
        .expect("configured");
    let coordinator = manager.shielded_coordinator().await.expect("coordinator");
    (manager, coordinator)
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

/// A wallet of `manager` with shielded account 0 bound.
async fn add_bound_wallet(manager: &Manager) -> Arc<PlatformWallet> {
    let wallet = add_wallet(manager).await;
    wallet.bind_shielded(&SEED, &[0]).await.expect("bound");
    wallet
}

/// Run `spend` on `wallet`'s account 0, with arguments that pass what each
/// entry point checks before it admits the spend.
async fn spend_from_credit_pool(
    spend: CreditPoolSpend,
    wallet: &PlatformWallet,
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
                .shielded_transfer_to(&SEED, 0, &recipient, 1_000, [0; 36], UnreachableProver)
                .await
        }
        CreditPoolSpend::Unshield => {
            wallet
                .shielded_unshield_to(
                    &SEED,
                    0,
                    &platform_address.to_bech32m_string(network),
                    1_000,
                    UnreachableProver,
                )
                .await
        }
        CreditPoolSpend::IdentityTopUp => wallet
            .shielded_identity_top_up_from_pool(&SEED, 0, &identity_id, 1_000, UnreachableProver)
            .await
            .map(|_| ()),
        CreditPoolSpend::Withdraw => {
            wallet
                .shielded_withdraw_to(
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
                .shielded_shield_from_account(0, 0, 1_000, &signer, UnreachableProver)
                .await
        }
        CreditPoolSpend::ShieldFromIdentity => wallet
            .shielded_shield_from_identity(0, &identity_id, 1_000, &signer, UnreachableProver)
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
    let (manager, coordinator) = configured_manager(dir.path()).await;
    let removed = add_bound_wallet(&manager).await;
    let wallet_id = removed.wallet_id();
    manager.remove_wallet(&wallet_id).await.expect("removed");
    for spend in CREDIT_POOL_SPENDS {
        let refused = spend_from_credit_pool(spend, &removed).await;
        assert!(
            matches!(&refused, Err(PlatformWalletError::WalletRemovedBeforeSend(id))
                if *id == hex::encode(wallet_id)),
            "{spend:?}: {refused:?}"
        );
    }

    let recreated = add_wallet(&manager).await;
    assert_eq!(recreated.wallet_id(), wallet_id);
    for spend in CREDIT_POOL_SPENDS {
        let refused = spend_from_credit_pool(spend, &removed).await;
        assert!(
            matches!(
                refused,
                Err(PlatformWalletError::WalletRemovedBeforeSend(_))
            ),
            "{spend:?}: {refused:?}"
        );
        // The re-created wallet is current, and not bound yet.
        let current = spend_from_credit_pool(spend, &recreated).await;
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
    let (manager, coordinator) = configured_manager(dir.path()).await;
    let wallet = add_bound_wallet(&manager).await;

    coordinator
        .clear(&WipeCountingPersistence::default())
        .await
        .expect("cleared");
    assert!(
        wallet.is_shielded_bound().await,
        "the handle keeps its keys across a Clear"
    );
    for spend in CREDIT_POOL_SPENDS {
        let refused = spend_from_credit_pool(spend, &wallet).await;
        assert!(
            matches!(refused, Err(PlatformWalletError::ShieldedNotBound)),
            "{spend:?}: {refused:?}"
        );
    }

    wallet
        .bind_shielded(&SEED, &[0])
        .await
        .expect("bound again");
    for spend in CREDIT_POOL_SPENDS {
        let admitted = spend_from_credit_pool(spend, &wallet).await;
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

/// A credit pool spend that was admitted holds the wallet generation's
/// lifecycle gate and the coordinator's operation admission until it
/// returns: a removal of the wallet waits for it, and a Clear finds an
/// operation admitted. Each spend is held here at the key slot, the first
/// wallet lock it takes after its admission. The two shields read the
/// wallet manager first, and are held there by the test after this one.
#[tokio::test]
async fn should_hold_removal_and_clear_admission_until_a_credit_pool_spend_returns() {
    for spend in [
        CreditPoolSpend::Transfer,
        CreditPoolSpend::Unshield,
        CreditPoolSpend::IdentityTopUp,
        CreditPoolSpend::Withdraw,
        CreditPoolSpend::IdentityCreate,
    ] {
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let (manager, coordinator) = configured_manager(dir.path()).await;
        let wallet = add_bound_wallet(&manager).await;

        let keys = wallet.shielded_keys.write().await;
        let running = {
            let wallet = Arc::clone(&wallet);
            tokio::spawn(async move { spend_from_credit_pool(spend, &wallet).await })
        };
        admitted(&coordinator, spend).await;
        assert!(
            removal_waits(&wallet).await,
            "{spend:?}: a removal waits for the spend in flight"
        );
        assert!(!running.is_finished(), "{spend:?}: held at the key slot");
        // The configuration lock is not taken: a bind of this wallet, and
        // another of its spends, do not wait for the proof and the result
        // of this one.
        assert!(
            wallet.shielded_config_lock.try_lock().is_ok(),
            "{spend:?}: the configuration lock is free while the spend runs"
        );

        drop(keys);
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
    let (manager, coordinator) = configured_manager(dir.path()).await;
    let wallet = add_bound_wallet(&manager).await;
    let wallet_id = wallet.wallet_id();

    let store = coordinator.store().write().await;
    let transfer = {
        let wallet = Arc::clone(&wallet);
        tokio::spawn(
            async move { spend_from_credit_pool(CreditPoolSpend::Transfer, &wallet).await },
        )
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

/// The spends that run one at a time on a wallet.
const SHIELD_CLASS_SPENDS: [CreditPoolSpend; 3] = [
    CreditPoolSpend::IdentityTopUp,
    CreditPoolSpend::ShieldFromAccount,
    CreditPoolSpend::ShieldFromIdentity,
];

/// A shield-class spend that finds another one running, here an asset-lock
/// funding's hold on `shield_guard`, which can last as long as a ChainLock
/// takes, waits for the guard before it is admitted: it has read, reserved
/// and signed nothing, so a Clear requested meanwhile finds no operation
/// admitted and is not refused for it. It holds the lifecycle gate while it
/// waits, as the funding does, so a removal waits for both. It is admitted
/// when its turn comes.
#[tokio::test]
async fn should_refuse_no_clear_while_a_shield_class_spend_waits_for_another() {
    for spend in SHIELD_CLASS_SPENDS {
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let (manager, coordinator) = configured_manager(dir.path()).await;
        let wallet = add_bound_wallet(&manager).await;

        let in_flight = wallet.shield_guard.lock().await;
        let queued = {
            let wallet = Arc::clone(&wallet);
            tokio::spawn(async move { spend_from_credit_pool(spend, &wallet).await })
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!queued.is_finished(), "{spend:?}: waits its turn");
        assert!(
            !coordinator.has_admitted_operation(),
            "{spend:?}: a Clear finds no operation admitted while the spend is queued"
        );
        assert!(
            removal_waits(&wallet).await,
            "{spend:?}: a removal waits for the queued spend, which holds the gate"
        );

        drop(in_flight);
        let returned = tokio::time::timeout(REACHED, queued)
            .await
            .expect("the queued spend takes its turn")
            .expect("spend task");
        assert!(
            !matches!(
                returned,
                Err(PlatformWalletError::WalletRemovedBeforeSend(_)
                    | PlatformWalletError::ShieldedNotBound)
            ),
            "{spend:?}: admitted once the other operation finished: {returned:?}"
        );
        assert!(!coordinator.has_admitted_operation());
    }
}

/// The two shields hold their admission once they run, as the other spends
/// do. Each is let past its admission, then held at the wallet manager,
/// which it reads next (its inputs, or the identity it debits).
#[tokio::test]
async fn should_hold_removal_and_clear_admission_while_a_shield_runs() {
    for spend in [
        CreditPoolSpend::ShieldFromAccount,
        CreditPoolSpend::ShieldFromIdentity,
    ] {
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let (manager, coordinator) = configured_manager(dir.path()).await;
        let wallet = add_bound_wallet(&manager).await;

        // Held at the wallet manager, which the shield reads once it is
        // admitted: nothing before its admission takes it.
        let wallet_manager = wallet.wallet_manager.write().await;
        let running = {
            let wallet = Arc::clone(&wallet);
            tokio::spawn(async move { spend_from_credit_pool(spend, &wallet).await })
        };
        admitted(&coordinator, spend).await;
        // Running: it has the single-flight guard.
        let deadline = tokio::time::Instant::now() + REACHED;
        while wallet.shield_guard.try_lock().is_ok() {
            assert!(
                tokio::time::Instant::now() < deadline,
                "{spend:?}: never took its turn"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(
            !running.is_finished(),
            "{spend:?}: held at the wallet manager"
        );
        assert!(
            coordinator.has_admitted_operation(),
            "{spend:?}: a Clear finds the running shield admitted"
        );
        assert!(
            removal_waits(&wallet).await,
            "{spend:?}: a removal waits for the running shield"
        );

        drop(wallet_manager);
        let returned = tokio::time::timeout(REACHED, running)
            .await
            .expect("the shield returns")
            .expect("spend task");
        assert!(
            !matches!(
                returned,
                Err(PlatformWalletError::WalletRemovedBeforeSend(_)
                    | PlatformWalletError::ShieldedNotBound)
            ),
            "{spend:?}: the wallet was current and bound when it was checked: {returned:?}"
        );
        assert!(!coordinator.has_admitted_operation());
        assert!(!removal_waits(&wallet).await);
    }
}

/// Several shield-class spends that waited for the same operation all take
/// their turn once it finishes, one at a time under the single-flight
/// guard, and none is admitted while it waits. Each is polled until it
/// waits before the operation in flight finishes.
#[tokio::test]
async fn should_run_every_shield_class_spend_that_waited_for_another() {
    let dir = tempfile::tempdir().expect("temporary shielded store");
    let (manager, coordinator) = configured_manager(dir.path()).await;
    let wallet = add_bound_wallet(&manager).await;

    let in_flight = wallet.shield_guard.lock().await;
    let mut queued: Vec<_> = [
        CreditPoolSpend::IdentityTopUp,
        CreditPoolSpend::IdentityTopUp,
        CreditPoolSpend::ShieldFromAccount,
        CreditPoolSpend::ShieldFromIdentity,
    ]
    .into_iter()
    .map(|spend| Box::pin(spend_from_credit_pool(spend, &wallet)))
    .collect();
    for (index, spend) in queued.iter_mut().enumerate() {
        assert!(
            futures::poll!(spend.as_mut()).is_pending(),
            "queued spend {index} waits for the operation in flight"
        );
    }
    assert!(!coordinator.has_admitted_operation());

    drop(in_flight);
    let returned = tokio::time::timeout(REACHED, futures::future::join_all(queued))
        .await
        .expect("every queued spend takes its turn");
    for (index, returned) in returned.into_iter().enumerate() {
        assert!(
            !matches!(
                returned,
                Err(PlatformWalletError::WalletRemovedBeforeSend(_)
                    | PlatformWalletError::ShieldedNotBound)
            ),
            "queued spend {index} was admitted: {returned:?}"
        );
    }
    assert!(!coordinator.has_admitted_operation());
}

/// A Clear issued while a credit pool spend runs is refused at once and
/// changes nothing; it neither waits for the spend nor holds it up. Once the
/// spend has returned, the Clear goes through.
#[tokio::test]
async fn should_refuse_a_clear_while_a_credit_pool_spend_runs() {
    let dir = tempfile::tempdir().expect("temporary shielded store");
    let (manager, coordinator) = configured_manager(dir.path()).await;
    let wallet = add_bound_wallet(&manager).await;

    let store = coordinator.store().write().await;
    let transfer = {
        let wallet = Arc::clone(&wallet);
        tokio::spawn(
            async move { spend_from_credit_pool(CreditPoolSpend::Transfer, &wallet).await },
        )
    };
    admitted(&coordinator, "transfer").await;
    let refused = tokio::time::timeout(
        REACHED,
        coordinator.clear(&WipeCountingPersistence::default()),
    )
    .await
    .expect("a Clear does not wait for the spend");
    assert!(
        matches!(
            refused,
            Err(PlatformWalletError::ShieldedOperationInProgress)
        ),
        "{refused:?}"
    );
    assert!(!transfer.is_finished(), "the transfer waits at the store");

    drop(store);
    let returned = tokio::time::timeout(REACHED, transfer)
        .await
        .expect("the refused Clear does not hold the spend up")
        .expect("transfer task");
    assert!(
        !matches!(returned, Err(PlatformWalletError::ShieldedNotBound)),
        "the refused Clear changed nothing: {returned:?}"
    );
    coordinator
        .clear(&WipeCountingPersistence::default())
        .await
        .expect("a Clear goes through once the spend has returned");
}

/// A Clear is not refused for a shield-class spend that is only waiting its
/// turn. The spend is told that the wallet is not bound when its turn comes:
/// the Clear dropped the wallet's registration while it waited, and it is
/// admitted, and its registration checked, only once it has the guard.
#[tokio::test]
async fn should_tell_a_spend_that_waited_through_a_clear_that_the_wallet_is_not_bound() {
    let dir = tempfile::tempdir().expect("temporary shielded store");
    let (manager, coordinator) = configured_manager(dir.path()).await;
    let wallet = add_bound_wallet(&manager).await;

    let in_flight = wallet.shield_guard.lock().await;
    let mut waiting = Box::pin(spend_from_credit_pool(
        CreditPoolSpend::IdentityTopUp,
        &wallet,
    ));
    assert!(futures::poll!(waiting.as_mut()).is_pending());
    coordinator
        .clear(&WipeCountingPersistence::default())
        .await
        .expect("a Clear is not refused for a spend that has not started");

    drop(in_flight);
    let waited = tokio::time::timeout(REACHED, waiting)
        .await
        .expect("the waiting spend takes its turn");
    assert!(
        matches!(waited, Err(PlatformWalletError::ShieldedNotBound)),
        "{waited:?}"
    );
}
