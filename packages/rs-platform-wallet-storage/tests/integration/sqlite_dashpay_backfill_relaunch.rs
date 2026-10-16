//! End-to-end relaunch of the DashPay coreHeight backfill over SQLite.
//!
//! `sqlite_dashpay_backfill.rs` pins the persister boundary: what `store()`
//! writes and `load()` hands back. These tests run the real
//! [`PlatformWalletManager`] over a real [`SqlitePersister`] in
//! [`FlushMode::Immediate`] — its own wallet-event adapter commits every
//! cursor advance, the reconcile stores its record, `load_from_persistor()`
//! rebuilds the wallet — and close and reopen the database file between
//! sessions, so what is asserted is what a relaunching process sees
//! (dashpay/platform#4302).
//!
//! The established contacts are applied in memory and stored as a
//! `ContactChangeSet` round — what the contact-request sync does once both
//! requests are fetched, which is the only step here that would otherwise
//! need the network. Every account, identity and cursor row comes from the
//! manager's own writers.

use crate::common;

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::secure_tempdir;
use dashcore::Network;
use dpp::identity::{Identity, IdentityV0};
use dpp::prelude::Identifier;
use key_wallet::account::account_collection::DashpayAccountKey;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::mnemonic::Mnemonic;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use platform_wallet::changeset::{
    ContactChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence, SentContactRequestKey,
};
use platform_wallet::events::EventHandler;
use platform_wallet::key_wallet_manager::WalletInterface;
use platform_wallet::test_support::{
    encrypt_contact_xpub_for_test, test_platform_wallet_manager_with,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::{
    derive_contact_xpub, ContactRequest, EstablishedContact, PlatformEventHandler, PlatformWallet,
    PlatformWalletManager,
};
use platform_wallet_storage::{FlushMode, SqlitePersister, SqlitePersisterConfig};
use rusqlite::{params, Connection};

/// The mnemonic `test_platform_wallet_manager_with` builds its wallet from.
const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon abandon abandon about";

const OWNER: [u8; 32] = [0xAA; 32];
const CONTACT: [u8; 32] = [0xBB; 32];

/// The contact's request height, below the cursor: a backfill is owed.
const REQUEST_HEIGHT: u32 = 100;
/// The cursor when the contact is established.
const TIP: u32 = 1_000;
/// Where the rescan has climbed to when the process dies.
const MID_CLIMB: u32 = 400;

struct NoopHandler;
impl EventHandler for NoopHandler {}
impl PlatformEventHandler for NoopHandler {}

fn open(path: &Path) -> Arc<SqlitePersister> {
    Arc::new(
        SqlitePersister::open(
            SqlitePersisterConfig::new(path).with_flush_mode(FlushMode::Immediate),
        )
        .expect("open persister"),
    )
}

/// A fresh process: a new manager over `persister`, restored from it.
async fn relaunch(persister: &Arc<SqlitePersister>) -> Arc<PlatformWalletManager<SqlitePersister>> {
    let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
    let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopHandler);
    let manager = Arc::new(PlatformWalletManager::new(
        sdk,
        Arc::clone(persister),
        handler,
    ));
    manager
        .load_from_persistor()
        .await
        .expect("the SQLite file must load into the manager");
    manager
}

/// End a session: stop the manager's workers, unregister the wallet and
/// release every handle on the database, so the next session can open it
/// (the process-wide open-path registry refuses a second live persister on
/// one file).
///
/// The wallet is removed from the manager (memory only; nothing is deleted
/// from the file) before the drop: a registered `PlatformWallet` is reachable
/// from itself through its SPV broadcaster -> `SpvRuntime` -> event manager ->
/// balance/spend handlers -> the shared `wallets` map, so dropping the
/// manager with the wallet still registered never frees the wallet, nor the
/// persister handles it carries.
async fn close(
    manager: Arc<PlatformWalletManager<SqlitePersister>>,
    wallet: Arc<PlatformWallet>,
    persister: Arc<SqlitePersister>,
) {
    let report = manager.shutdown().await;
    assert!(
        report.all_clean(),
        "shutdown must join every worker: {report:?}"
    );
    let removed = manager
        .remove_wallet(&wallet.wallet_id())
        .await
        .expect("unregister the wallet");
    drop(removed);
    drop(wallet);
    drop(manager);
    // Give any adapter task still unwinding its claim a chance to drop it.
    let deadline = Instant::now() + Duration::from_secs(10);
    while Arc::strong_count(&persister) > 1 && Instant::now() < deadline {
        tokio::task::yield_now().await;
    }
    assert_eq!(
        Arc::strong_count(&persister),
        1,
        "every manager-side handle on the persister must be gone at close"
    );
    drop(persister);
}

fn seed_wallet() -> key_wallet::wallet::Wallet {
    let seed = Mnemonic::from_phrase(TEST_MNEMONIC)
        .expect("mnemonic")
        .to_seed("");
    key_wallet::wallet::Wallet::from_seed_bytes(
        seed,
        Network::Testnet,
        WalletAccountCreationOptions::None,
    )
    .expect("seed wallet")
}

fn bare_identity(id: Identifier) -> Identity {
    Identity::V0(IdentityV0 {
        id,
        public_keys: Default::default(),
        balance: 0,
        revision: 0,
    })
}

fn stored_cursor(persister: &SqlitePersister, wallet_id: &WalletId) -> Option<u32> {
    persister
        .lock_conn_for_test()
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| row.get::<_, Option<u32>>(0),
        )
        .ok()
        .flatten()
}

fn stored_records(persister: &SqlitePersister, wallet_id: &WalletId) -> i64 {
    persister
        .lock_conn_for_test()
        .query_row(
            "SELECT COUNT(*) FROM dashpay_backfill WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| row.get(0),
        )
        .expect("count dashpay_backfill")
}

/// Advance the scan the way the SPV engine does — the wallet manager's
/// `update_wallet_synced_height`, which emits `SyncHeightAdvanced` to the
/// manager's own adapter — and wait until the adapter has committed it.
async fn advance_and_commit(wallet: &PlatformWallet, persister: &SqlitePersister, height: u32) {
    let wallet_id = wallet.wallet_id();
    wallet
        .wallet_manager()
        .write()
        .await
        .update_wallet_synced_height(&wallet_id, height);
    let deadline = Instant::now() + Duration::from_secs(10);
    while stored_cursor(persister, &wallet_id) != Some(height) {
        assert!(
            Instant::now() < deadline,
            "the adapter never committed the advance to {height} (stored: {:?})",
            stored_cursor(persister, &wallet_id)
        );
        tokio::task::yield_now().await;
    }
}

/// Add the owner identity through the manager's writer, then establish the
/// `(owner, contact)` relationship with both requests at `height`: in memory,
/// and on disk as the contact sync's round would store it.
async fn establish_contact(
    wallet: &PlatformWallet,
    persister: &SqlitePersister,
    owner: Identifier,
    contact: Identifier,
    height: u32,
    incoming_account_reference: u32,
    incoming_encrypted_xpub: Vec<u8>,
) -> EstablishedContact {
    let wallet_id = wallet.wallet_id();
    let established = EstablishedContact::new(
        contact,
        ContactRequest::new(owner, contact, 0, 0, 0, vec![0u8; 96], height, 0),
        ContactRequest::new(
            contact,
            owner,
            0,
            0,
            incoming_account_reference,
            incoming_encrypted_xpub,
            height,
            0,
        ),
    );
    {
        let mut state = wallet.state_mut().await;
        if state.identity_manager.managed_identity(&owner).is_none() {
            state
                .identity_manager
                .add_identity(bare_identity(owner), 0, wallet_id, wallet.persister())
                .expect("add the owner identity");
        }
        state
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("owner managed")
            .apply_established_contact(established.clone());
    }
    let mut contacts = ContactChangeSet::default();
    contacts.established.insert(
        SentContactRequestKey {
            owner_id: owner,
            recipient_id: contact,
        },
        established.clone(),
    );
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                contacts: Some(contacts),
                ..Default::default()
            },
        )
        .expect("store the established contact");
    established
}

async fn synced_height(wallet: &PlatformWallet) -> u32 {
    wallet.state().await.core_wallet.synced_height()
}

/// dashpay/platform#4302 on the SQLite backend, end to end. The rewind's
/// record and the lowered cursor reach the file; the scan climbs part-way;
/// the process dies. The next process restores the cursor inside the climb
/// and must resume from it rather than rewind again. Then the control: the
/// same file with its `dashpay_backfill` row deleted rewinds again, so it is
/// the persisted record — not anything else the file restores — that stops
/// the second rewind.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_resume_the_backfill_from_the_persisted_cursor_after_a_sqlite_relaunch() {
    let tmp = secure_tempdir().expect("tempdir");
    let path = tmp.path().join("wallet.db");
    let owner = Identifier::from(OWNER);
    let contact = Identifier::from(CONTACT);

    // ── Session one: establish, rewind, climb part-way, die. ──
    let persister = open(&path);
    let (manager, wallet_id) = test_platform_wallet_manager_with(Arc::clone(&persister)).await;
    let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
    let receiving = derive_contact_xpub(&seed_wallet(), Network::Testnet, 0, &owner, &contact)
        .expect("derive receiving xpub")
        .xpub;
    wallet
        .identity()
        .dashpay()
        .register_contact_account(&owner, &contact, 0, receiving)
        .await
        .expect("register the receival account");
    establish_contact(
        &wallet,
        &persister,
        owner,
        contact,
        REQUEST_HEIGHT,
        0,
        vec![0u8; 96],
    )
    .await;
    // The rewind path serves accounts on file before the record existed; the
    // record must not already know this one.
    wallet
        .state_mut()
        .await
        .dashpay_backfill
        .retain(|o, c, _| (o, c) != (&owner, &contact));
    advance_and_commit(&wallet, &persister, TIP).await;

    assert_eq!(
        wallet
            .identity()
            .dashpay()
            .reconcile_dashpay_rescan()
            .await
            .expect("first pass"),
        Some(REQUEST_HEIGHT),
        "a contact funded below the cursor rewinds it to the request height"
    );
    assert_eq!(synced_height(&wallet).await, REQUEST_HEIGHT);
    assert_eq!(
        stored_cursor(&persister, &wallet_id),
        Some(REQUEST_HEIGHT),
        "the rewind must overwrite the stored cursor (synced_height_is_rewind)"
    );
    assert_eq!(stored_records(&persister, &wallet_id), 1);
    {
        let mut state = persister.load().expect("load");
        let record = state
            .wallets
            .remove(&wallet_id)
            .expect("wallet on file")
            .dashpay_backfill;
        assert_eq!((record.floor, record.rewound_from), (REQUEST_HEIGHT, TIP));
        assert_eq!(
            record.covered_from(&owner, &contact, 0),
            Some(REQUEST_HEIGHT)
        );
    }

    advance_and_commit(&wallet, &persister, MID_CLIMB).await;
    close(manager, wallet, persister).await;

    // ── Session two: the next launch resumes instead of rewinding. ──
    let persister = open(&path);
    let manager = relaunch(&persister).await;
    let wallet = manager
        .get_wallet(&wallet_id)
        .await
        .expect("restored wallet");
    assert_eq!(
        synced_height(&wallet).await,
        MID_CLIMB,
        "the cursor comes back inside the climb"
    );
    assert!(
        wallet
            .state()
            .await
            .core_wallet
            .accounts
            .dashpay_receival_accounts
            .contains_key(&DashpayAccountKey {
                index: 0,
                user_identity_id: OWNER,
                friend_identity_id: CONTACT,
            }),
        "the receival account the record covers must be restored"
    );
    assert_eq!(
        wallet
            .identity()
            .dashpay()
            .reconcile_dashpay_rescan()
            .await
            .expect("pass after relaunch"),
        None,
        "a contact the persisted record covers must not be rewound for again"
    );
    assert_eq!(
        synced_height(&wallet).await,
        MID_CLIMB,
        "the scan resumes from the restored cursor"
    );
    assert_eq!(stored_cursor(&persister, &wallet_id), Some(MID_CLIMB));
    assert!(wallet.state().await.dashpay_backfill.is_pending(MID_CLIMB));
    close(manager, wallet, persister).await;

    // ── Control: the same file without its record rewinds again. ──
    {
        let conn = Connection::open(&path).expect("open the closed file");
        let deleted = conn
            .execute(
                "DELETE FROM dashpay_backfill WHERE wallet_id = ?1",
                params![wallet_id.as_slice()],
            )
            .expect("delete the record row");
        assert_eq!(deleted, 1, "the record row must have been on file");
    }
    let persister = open(&path);
    let manager = relaunch(&persister).await;
    let wallet = manager
        .get_wallet(&wallet_id)
        .await
        .expect("restored wallet");
    assert_eq!(synced_height(&wallet).await, MID_CLIMB);
    assert_eq!(
        wallet
            .identity()
            .dashpay()
            .reconcile_dashpay_rescan()
            .await
            .expect("pass without a record"),
        Some(REQUEST_HEIGHT),
        "without the persisted record the relaunch rewinds again — the #4302 defect"
    );
    close(manager, wallet, persister).await;
}

/// The outbound-account marker survives a SQLite relaunch together with the
/// account it names. Registering the outbound (`DashpayExternalAccount`)
/// account binds the marker in the same round as the account and its pool;
/// after a reopen the contact carries the marker (so the startup sweep does
/// not rebuild the account) AND the account is there with the same xpub and
/// pool — a marker restored without its account would suppress the rebuild
/// of an account that no longer exists.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_restore_the_outbound_account_marker_with_its_account_after_a_sqlite_relaunch() {
    let tmp = secure_tempdir().expect("tempdir");
    let path = tmp.path().join("wallet.db");
    let owner = Identifier::from(OWNER);
    let contact = Identifier::from(CONTACT);
    let key = DashpayAccountKey {
        index: 0,
        user_identity_id: OWNER,
        friend_identity_id: CONTACT,
    };
    const ACCOUNT_REFERENCE: u32 = 7;
    let shared_key = [0x55u8; 32];

    // ── Session one: establish, then build the outbound account. ──
    let persister = open(&path);
    let (manager, wallet_id) = test_platform_wallet_manager_with(Arc::clone(&persister)).await;
    let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
    // The contact's receiving xpub for us, as their request carries it.
    let theirs = derive_contact_xpub(&seed_wallet(), Network::Testnet, 0, &contact, &owner)
        .expect("derive the contact's xpub");
    let encrypted = encrypt_contact_xpub_for_test(&shared_key, &theirs.compact_xpub());
    establish_contact(
        &wallet,
        &persister,
        owner,
        contact,
        REQUEST_HEIGHT,
        ACCOUNT_REFERENCE,
        encrypted.clone(),
    )
    .await;
    let registration = wallet
        .identity()
        .dashpay()
        .register_external_contact_account(
            &owner,
            &bare_identity(contact),
            &encrypted,
            zeroize::Zeroizing::new(shared_key),
        )
        .await
        .expect("register the outbound account");
    assert_eq!(format!("{registration:?}"), "Built");

    let (xpub_before, pool_before) = {
        let state = wallet.state().await;
        let established = state
            .identity_manager
            .managed_identity(&owner)
            .expect("owner")
            .dashpay()
            .established_contacts()
            .get(&contact)
            .expect("contact")
            .clone();
        assert_eq!(
            established.external_account_reference,
            Some(ACCOUNT_REFERENCE),
            "an inline, atomic backend gets the marker in the registration round"
        );
        let managed = state
            .core_wallet
            .accounts
            .dashpay_external_accounts
            .get(&key)
            .expect("outbound account registered");
        let pool: Vec<_> = managed
            .managed_account_type()
            .address_pools()
            .first()
            .expect("outbound pool")
            .addresses
            .values()
            .map(|info| info.address.clone())
            .collect();
        drop(state);
        let wm = wallet.wallet_manager().read().await;
        let xpub = wm
            .get_wallet(&wallet_id)
            .expect("wallet")
            .accounts
            .dashpay_external_accounts
            .get(&key)
            .expect("outbound account on the wallet")
            .account_xpub;
        (xpub, pool)
    };
    assert!(!pool_before.is_empty(), "the outbound pool is pre-derived");
    {
        let conn = persister.lock_conn_for_test();
        let marker: Option<i64> = conn
            .query_row(
                "SELECT external_account_reference FROM contacts \
                 WHERE owner_id = ?1 AND contact_id = ?2",
                params![&OWNER[..], &CONTACT[..]],
                |row| row.get(0),
            )
            .expect("contact row");
        assert_eq!(marker, Some(i64::from(ACCOUNT_REFERENCE)));
    }
    close(manager, wallet, persister).await;

    // ── Session two: the marker and its account both come back. ──
    let persister = open(&path);
    let manager = relaunch(&persister).await;
    let wallet = manager
        .get_wallet(&wallet_id)
        .await
        .expect("restored wallet");
    {
        let state = wallet.state().await;
        let established = state
            .identity_manager
            .managed_identity(&owner)
            .expect("owner restored")
            .dashpay()
            .established_contacts()
            .get(&contact)
            .expect("established contact restored")
            .clone();
        assert_eq!(
            established.external_account_reference,
            Some(ACCOUNT_REFERENCE),
            "the persisted marker must survive the relaunch"
        );
        assert_eq!(
            established.external_account_reference,
            Some(established.incoming_request.account_reference),
            "the marker names the live request, so the sweep does not rebuild"
        );
        let managed = state
            .core_wallet
            .accounts
            .dashpay_external_accounts
            .get(&key)
            .expect("the outbound account the marker names must be restored");
        let pool: Vec<_> = managed
            .managed_account_type()
            .address_pools()
            .first()
            .expect("outbound pool restored")
            .addresses
            .values()
            .map(|info| info.address.clone())
            .collect();
        assert_eq!(pool, pool_before, "the restored pool is the one built");
    }
    {
        let wm = wallet.wallet_manager().read().await;
        let account = wm
            .get_wallet(&wallet_id)
            .expect("wallet")
            .accounts
            .dashpay_external_accounts
            .get(&key)
            .expect("the outbound account's xpub must be restored on the wallet");
        assert_eq!(account.account_xpub, xpub_before);
        // The account is rebuilt from the 69-byte compact form, which omits
        // the BIP32 metadata, so compare the key material it derives from.
        assert_eq!(
            (
                account.account_xpub.public_key,
                account.account_xpub.chain_code
            ),
            (theirs.xpub.public_key, theirs.xpub.chain_code),
            "the restored account derives from the contact's xpub"
        );
    }
    // The startup sweep would now take the account as built: re-registering
    // finds it rather than rebuilding it.
    let again = wallet
        .identity()
        .dashpay()
        .register_external_contact_account(
            &owner,
            &bare_identity(contact),
            &encrypted,
            zeroize::Zeroizing::new(shared_key),
        )
        .await
        .expect("re-register after relaunch");
    assert_eq!(format!("{again:?}"), "AlreadyExisted");
    close(manager, wallet, persister).await;
}
