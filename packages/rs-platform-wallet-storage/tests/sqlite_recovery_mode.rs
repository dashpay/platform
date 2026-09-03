#![allow(clippy::field_reassign_with_default)]

//! Strict-by-default load vs. opt-in recovery, site by site.
//!
//! Each tolerable-today inconsistency gets a pair: it aborts the load under
//! [`LoadPolicy::Strict`] with a named error, and under
//! [`LoadPolicy::Recovery`] it is logged, counted on
//! [`SqlitePersister::last_load_degradation`], and the documented degraded
//! projection is served instead.

mod common;

use common::{ensure_wallet_meta, fresh_persister, fresh_recovery_persister, wid};
use dashcore::hashes::Hash;
use dashcore::Txid;
use dpp::identity::accessors::IdentityGettersV0;
use platform_wallet::changeset::{
    AccountRegistrationEntry, CoreChangeSet, IdentityEntry, PersistenceError,
    PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet::wallet::identity::IdentityStatus;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{LoadSite, SqlitePersister, WalletStorageError};
use rusqlite::params;

/// Downcast a trait-boundary error to the storage error it wraps.
#[track_caller]
fn typed(err: PersistenceError) -> WalletStorageError {
    let PersistenceError::Backend { source, .. } = err else {
        panic!("expected a typed backend error, got {err:?}");
    };
    *source
        .downcast::<WalletStorageError>()
        .expect("backend source must be a WalletStorageError")
}

/// Assert exactly `expected` tolerated events were counted at `site`, and
/// that no other site fired.
#[track_caller]
fn assert_only_site(persister: &SqlitePersister, site: LoadSite, expected: u32) {
    let degradation = persister.last_load_degradation();
    assert!(degradation.degraded, "load must report itself degraded");
    assert_eq!(
        degradation.by_site.get(&site).copied(),
        Some(expected),
        "per-site count for {site}: {:?}",
        degradation.by_site
    );
    assert_eq!(
        degradation.by_site.len(),
        1,
        "no other site may fire: {:?}",
        degradation.by_site
    );
    assert_eq!(degradation.total, expected);
}

/// Seed a `core_sync_state` row, then overwrite its chain lock with bytes
/// that are not a `ChainLock`.
fn seed_corrupt_chain_lock(persister: &SqlitePersister, wallet: &WalletId) {
    ensure_wallet_meta(persister, wallet);
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(11),
        last_processed_height: Some(11),
        ..Default::default()
    });
    persister.store(*wallet, cs).expect("seed core sync state");
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "UPDATE core_sync_state SET last_applied_chain_lock = ?1 WHERE wallet_id = ?2",
        params![&[0xFFu8; 5][..], wallet.as_slice()],
    )
    .expect("plant corrupt chain lock");
}

/// Seed one blob-bearing `core_transactions` row, then drift its typed
/// `height` column away from the height inside the blob.
fn seed_drifted_transaction(persister: &SqlitePersister, wallet: &WalletId) {
    ensure_wallet_meta(persister, wallet);
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        records: vec![confirmed_record()],
        ..Default::default()
    });
    persister.store(*wallet, cs).expect("seed transaction");
    let conn = persister.lock_conn_for_test();
    let updated = conn
        .execute(
            "UPDATE core_transactions SET height = 999 WHERE wallet_id = ?1",
            params![wallet.as_slice()],
        )
        .expect("drift typed height");
    assert_eq!(updated, 1, "seed must have written exactly one row");
}

fn drifted_txid() -> Txid {
    Txid::from_byte_array([0x7Au8; 32])
}

/// A record whose blob says height 300, so a drifted typed column is
/// unambiguous.
fn confirmed_record() -> key_wallet::managed_account::transaction_record::TransactionRecord {
    use dashcore::{BlockHash, Transaction};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
    let mut record = TransactionRecord::new(
        Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        },
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        TransactionContext::InChainLockedBlock(BlockInfo::new(
            300,
            BlockHash::from_byte_array([0x21u8; 32]),
            1_735_689_600,
        )),
        TransactionType::Standard,
        TransactionDirection::Incoming,
        Vec::new(),
        Vec::new(),
        100,
    );
    record.txid = drifted_txid();
    record
}

/// Register a deterministic keyless wallet and return its BIP44 external
/// index-zero address.
fn seed_registered_wallet(
    persister: &SqlitePersister,
    wallet_id: WalletId,
    seed: u8,
) -> dashcore::Address {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
    use key_wallet::wallet::Wallet;

    let wallet = Wallet::from_seed_bytes(
        [seed; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("test wallet");
    let info = ManagedWalletInfo::from_wallet(&wallet, 1);
    let address = info.accounts.standard_bip44_accounts[&0]
        .managed_account_type()
        .address_pools()
        .into_iter()
        .find(|pool| pool.is_external())
        .and_then(|pool| pool.address_at_index(0))
        .expect("BIP44 external address");
    let account_registrations = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|account| AccountRegistrationEntry {
            account_type: account.account_type,
            account_xpub: account.account_xpub,
        })
        .collect();
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0; 32],
                    birth_height: 1,
                }),
                account_registrations,
                ..Default::default()
            },
        )
        .expect("register wallet");
    address
}

fn identity_entry(wallet_id: WalletId, id: u8, index: u32) -> IdentityEntry {
    IdentityEntry {
        id: dpp::prelude::Identifier::from([id; 32]),
        balance: u64::from(id),
        revision: 1,
        identity_index: Some(index),
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Active,
        wallet_id: Some(wallet_id),
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

fn seed_identity_index_collision(persister: &SqlitePersister, wallet_id: WalletId) {
    use platform_wallet_storage::sqlite::schema::blob;

    ensure_wallet_meta(persister, &wallet_id);
    let conn = persister.lock_conn_for_test();
    for id in [0xEE, 0x11] {
        let entry = identity_entry(wallet_id, id, 7);
        conn.execute(
            "INSERT INTO identities \
                (identity_id, wallet_id, identity_index, entry_blob, tombstoned) \
             VALUES (?1, ?2, 7, ?3, 0)",
            params![
                entry.id.as_slice(),
                wallet_id.as_slice(),
                blob::encode(&entry).unwrap()
            ],
        )
        .expect("plant duplicate identity index");
    }
    conn.execute_batch(
        "ANALYZE; \
         UPDATE sqlite_stat1 SET stat = '1000000 1000000' \
             WHERE idx = 'idx_identities_wallet_identity'; \
         UPDATE sqlite_stat1 SET stat = '2 2' WHERE idx = 'idx_identities_wallet'; \
         ANALYZE sqlite_schema;",
    )
    .expect("make the unordered query prefer insertion order");
}

fn seed_asset_lock_status_drift(
    persister: &SqlitePersister,
    wallet_id: WalletId,
) -> dashcore::OutPoint {
    use dashcore::{OutPoint, Transaction};
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use platform_wallet::changeset::AssetLockEntry;
    use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
    use platform_wallet_storage::sqlite::schema::{asset_locks, blob};

    ensure_wallet_meta(persister, &wallet_id);
    let outpoint = OutPoint::new(Txid::from_byte_array([0xA5; 32]), 0);
    let entry = AssetLockEntry {
        out_point: outpoint,
        transaction: Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        },
        account_index: 0,
        funding_type: AssetLockFundingType::IdentityTopUp,
        identity_index: 0,
        amount_duffs: 1_000,
        status: AssetLockStatus::Consumed,
        proof: None,
    };
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO asset_locks \
            (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
         VALUES (?1, ?2, 'built', 0, 0, 1000, ?3)",
        params![
            wallet_id.as_slice(),
            blob::encode_outpoint(&outpoint).unwrap(),
            asset_locks::encode_entry_for_test(&entry).unwrap()
        ],
    )
    .expect("plant asset-lock status drift");

    let live_outpoint = OutPoint::new(Txid::from_byte_array([0xB6; 32]), 0);
    let live_entry = AssetLockEntry {
        out_point: live_outpoint,
        status: AssetLockStatus::Built,
        ..entry
    };
    conn.execute(
        "INSERT INTO asset_locks \
            (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
         VALUES (?1, ?2, 'built', 0, 0, 1000, ?3)",
        params![
            wallet_id.as_slice(),
            blob::encode_outpoint(&live_outpoint).unwrap(),
            asset_locks::encode_entry_for_test(&live_entry).unwrap()
        ],
    )
    .expect("plant live asset-lock control");
    live_outpoint
}

#[test]
fn account_registration_drift_is_strictly_fatal_and_recovery_drops_only_that_row() {
    let wallet = wid(0x3E);
    let outpoint = dashcore::OutPoint::new(Txid::from_byte_array([0x3E; 32]), 0);
    let seed = |persister: &SqlitePersister| {
        let address = seed_registered_wallet(persister, wallet, 0x3E);
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_address_pool \
                (wallet_id, account_type, account_index, key_class, pool_type, \
                 address_index, script, used) \
             VALUES (?1, 'standard_bip44', 0, 0, 0, 0, ?2, 0)",
            params![wallet.as_slice(), address.script_pubkey().as_bytes()],
        )
        .expect("plant BIP44 ownership row");
        conn.execute(
            "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) \
             VALUES (?1, ?2, 7000, ?3, 0)",
            params![
                wallet.as_slice(),
                platform_wallet_storage::sqlite::schema::blob::encode_outpoint(&outpoint).unwrap(),
                address.script_pubkey().as_bytes()
            ],
        )
        .expect("plant BIP44 UTXO");
        assert_eq!(
            conn.execute(
                "UPDATE account_registrations SET account_index = 99 \
                 WHERE wallet_id = ?1 AND account_type = 'standard_bip44'",
                params![wallet.as_slice()],
            )
            .unwrap(),
            1
        );
    };

    let (strict, _tmp, _path) = fresh_persister();
    seed(&strict);
    let err = typed(
        strict
            .load()
            .expect_err("strict must reject registration drift"),
    );
    assert!(matches!(
        err,
        WalletStorageError::AccountRegistrationEntryMismatch
    ));

    let (recovery, _tmp, _path) = fresh_recovery_persister(seed);
    let state = recovery
        .load()
        .expect("recovery drops only the drifted row");
    let loaded = &state.wallets[&wallet];
    assert!(loaded.wallet.accounts.standard_bip44_accounts.is_empty());
    assert!(loaded
        .wallet
        .accounts
        .standard_bip32_accounts
        .contains_key(&0));
    assert!(loaded.wallet.accounts.coinjoin_accounts.contains_key(&0));
    let fallback = &loaded.wallet_info.accounts.standard_bip32_accounts[&0];
    assert!(fallback.utxos.contains_key(&outpoint));
    assert_eq!(fallback.balance.total(), 7_000);
    assert_eq!(loaded.wallet_info.balance.total(), 7_000);
    let degradation = recovery.last_load_degradation();
    assert_eq!(
        degradation.by_site.get(&LoadSite::AccountRegistrationDrift),
        Some(&1)
    );
    assert_eq!(
        degradation.by_site.get(&LoadSite::OrphanedUtxoOwner),
        Some(&2)
    );
    assert_eq!(
        degradation.by_site.get(&LoadSite::UnresolvedUtxoAddress),
        Some(&1)
    );
    assert_eq!(degradation.by_site.len(), 3);
    assert_eq!(degradation.total, 4);
}

#[test]
fn consumed_blob_status_withdraws_asset_lock_from_recovery_live_set() {
    let wallet = wid(0x3F);
    let (strict, _tmp, _path) = fresh_persister();
    seed_asset_lock_status_drift(&strict, wallet);
    let err = typed(strict.load().expect_err("strict must reject status drift"));
    assert!(matches!(
        err,
        WalletStorageError::AssetLockStatusMismatch { .. }
    ));

    let live_outpoint = dashcore::OutPoint::new(Txid::from_byte_array([0xB6; 32]), 0);
    let (recovery, _tmp, _path) = fresh_recovery_persister(|strict| {
        seed_asset_lock_status_drift(strict, wallet);
    });
    let state = recovery
        .load()
        .expect("recovery lets the blob withdraw the lock");
    let locks = &state.wallets[&wallet].unused_asset_locks;
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[&0].len(), 1);
    assert!(locks[&0].contains_key(&live_outpoint));
    let drifted_outpoint = dashcore::OutPoint::new(Txid::from_byte_array([0xA5; 32]), 0);
    assert!(!locks[&0].contains_key(&drifted_outpoint));
    assert_only_site(&recovery, LoadSite::AssetLockStatusDrift, 1);
}

// -- previously uncovered load sites -----------------------------------

#[test]
fn identity_index_collision_is_strictly_fatal_and_recovery_loses_no_identity() {
    let wallet = wid(0x40);
    let (strict, _tmp, _path) = fresh_persister();
    seed_identity_index_collision(&strict, wallet);
    let err = typed(strict.load().expect_err("strict must reject the collision"));
    assert!(matches!(
        err,
        WalletStorageError::IdentityIndexConflict { .. }
    ));

    let (recovery, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_identity_index_collision(strict, wallet));
    let state = recovery.load().expect("recovery must select one identity");
    let manager = &state.wallets[&wallet].identity_manager;
    let identities = &manager.wallet_identities[&wallet];
    // Only one identity can hold slot 7 — that part is unavoidable.
    assert_eq!(identities.len(), 1);
    assert_eq!(
        identities[&7].identity.id(),
        dpp::prelude::Identifier::from([0xEE; 32]),
        "ascending identity_id order makes the lexicographically higher row the winner"
    );
    // The load must not LOSE the other one. Nothing on disk says which
    // identity truly owns index 7, so the displaced row is parked in the
    // no-slot bucket rather than dropped. Recovery makes the persister
    // read-only, so an identity dropped here could never be re-persisted.
    let displaced = dpp::prelude::Identifier::from([0x11; 32]);
    assert!(
        manager.out_of_wallet_identities.contains_key(&displaced),
        "the displaced identity must survive the load, not vanish; got keys {:?}",
        manager.out_of_wallet_identities.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        manager.out_of_wallet_identities[&displaced].identity.id(),
        displaced
    );
    assert_only_site(&recovery, LoadSite::IdentityIndexCollision, 1);
}

#[test]
fn orphaned_utxo_owner_is_counted_by_recovery_load() {
    let wallet = wid(0x41);
    let (persister, _tmp, _path) = fresh_recovery_persister(|strict| {
        let address = seed_registered_wallet(strict, wallet, 0x41);
        let conn = strict.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_address_pool \
                (wallet_id, account_type, account_index, key_class, pool_type, \
                 address_index, script, used) \
             VALUES (?1, 'standard_bip44', 999, 0, 0, 0, ?2, 1)",
            params![wallet.as_slice(), address.script_pubkey().as_bytes()],
        )
        .expect("plant orphaned pool owner");
    });

    persister.load().expect("recovery load");
    assert_only_site(&persister, LoadSite::OrphanedUtxoOwner, 1);
}

#[test]
fn unresolved_and_undecodable_addresses_are_counted_separately() {
    let wallet = wid(0x42);
    let (persister, _tmp, _path) = fresh_recovery_persister(|strict| {
        seed_registered_wallet(strict, wallet, 0x42);
        let conn = strict.lock_conn_for_test();
        let bad_script = [0x6A_u8];
        conn.execute(
            "INSERT INTO core_address_pool \
                (wallet_id, account_type, account_index, key_class, pool_type, \
                 address_index, script, used) \
             VALUES (?1, 'standard_bip44', 0, 0, 0, 0, ?2, 1)",
            params![wallet.as_slice(), bad_script.as_slice()],
        )
        .expect("plant undecodable pool script");
        conn.execute(
            "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) \
             VALUES (?1, ?2, 0, ?3, 1)",
            params![wallet.as_slice(), &[0xFF_u8; 36], bad_script.as_slice()],
        )
        .expect("plant undecodable UTXO script");

        for index in 0_u32..900 {
            let mut hash = [0_u8; 20];
            hash[..4].copy_from_slice(&index.to_le_bytes());
            let address = dashcore::Address::new(
                dashcore::Network::Testnet,
                dashcore::address::Payload::PubkeyHash(dashcore::PubkeyHash::from_byte_array(hash)),
            );
            let mut outpoint = [0_u8; 36];
            outpoint[..4].copy_from_slice(&index.to_le_bytes());
            outpoint[4] = 1;
            conn.execute(
                "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) \
                 VALUES (?1, ?2, 0, ?3, 1)",
                params![
                    wallet.as_slice(),
                    outpoint,
                    address.script_pubkey().as_bytes()
                ],
            )
            .expect("plant unresolved used address");
        }
    });

    persister
        .load()
        .expect("recovery must skip undecodable scripts");
    let degradation = persister.last_load_degradation();
    assert_eq!(
        degradation.by_site.get(&LoadSite::UnresolvedUtxoAddress),
        Some(&900)
    );
    assert_eq!(
        degradation.by_site.get(&LoadSite::UndecodableAddressScript),
        Some(&2)
    );
    assert_eq!(degradation.by_site.len(), 2);
    assert_eq!(degradation.total, 900 + 2);
}

#[test]
fn identity_scan_state_contradiction_is_counted_by_recovery_load() {
    let wallet = wid(0x43);
    let (persister, _tmp, _path) = fresh_recovery_persister(|strict| {
        ensure_wallet_meta(strict, &wallet);
        let conn = strict.lock_conn_for_test();
        conn.execute(
            "INSERT INTO identity_scan_states \
                (wallet_id, complete, probed_from, probed_through, unlocated_gap) \
             VALUES (?1, 1, 0, 9, 0)",
            params![wallet.as_slice()],
        )
        .expect("plant complete verdict");
        conn.execute(
            "INSERT INTO identity_scan_failed_indices (wallet_id, failed_index) \
             VALUES (?1, 4)",
            params![wallet.as_slice()],
        )
        .expect("plant unanswered index");
    });

    let state = persister.load().expect("recovery clamps the verdict");
    assert!(!state.wallets[&wallet].identity_manager.scan_states[&wallet].complete);
    assert_only_site(&persister, LoadSite::IdentityScanStateContradiction, 1);
}

// ── (a) chain-lock blob ─────────────────────────────────────────────────

#[test]
fn corrupt_chain_lock_blob_is_fatal_under_strict() {
    let wallet = wid(0x20);
    let (persister, _tmp, _path) = fresh_persister();
    seed_corrupt_chain_lock(&persister, &wallet);

    let err = typed(
        persister
            .load()
            .expect_err("a corrupt chain lock must abort a strict load"),
    );
    assert!(
        matches!(err, WalletStorageError::BincodeDecode { .. }),
        "expected the upstream decode error to survive, got {err:?}"
    );
    assert!(
        !persister.is_degraded(),
        "a failed load must not report a partial tally"
    );
}

#[test]
fn corrupt_chain_lock_blob_is_tolerated_in_recovery() {
    let wallet = wid(0x21);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_corrupt_chain_lock(strict, &wallet));

    let state = persister.load().expect("recovery must complete the load");
    let loaded = state.wallets.get(&wallet).expect("wallet must rehydrate");
    assert!(
        loaded
            .wallet_info
            .metadata
            .last_applied_chain_lock
            .is_none(),
        "the undecodable chain lock must be dropped, not guessed at"
    );
    assert_eq!(
        loaded.wallet_info.metadata.synced_height, 11,
        "the rest of the sync state must survive"
    );
    assert_only_site(&persister, LoadSite::ChainLockBlob, 1);
}

// ── (c) core-transaction typed-column drift ─────────────────────────────

#[test]
fn core_transaction_column_drift_is_fatal_under_strict() {
    let wallet = wid(0x22);
    let (persister, _tmp, _path) = fresh_persister();
    seed_drifted_transaction(&persister, &wallet);

    let err = typed(
        persister
            .load()
            .expect_err("typed columns disagreeing with the blob must abort a strict load"),
    );
    assert!(
        matches!(
            err,
            WalletStorageError::CoreTransactionEntryMismatch {
                typed_height: Some(999),
                blob_height: Some(300),
                ..
            }
        ),
        "expected CoreTransactionEntryMismatch, got {err:?}"
    );
}

#[test]
fn core_transaction_column_drift_is_tolerated_in_recovery() {
    let wallet = wid(0x23);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_drifted_transaction(strict, &wallet));

    persister.load().expect("recovery must complete the load");
    assert_only_site(&persister, LoadSite::CoreTransactionColumnDrift, 1);
}

#[test]
fn get_core_tx_record_never_writes() {
    // The read path used to repair drifted typed columns in place. A `&self`
    // read on the persistence trait must not mutate the database at all —
    // this pins the row bytes across a drift read.
    let wallet = wid(0x24);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_drifted_transaction(strict, &wallet));

    let before = transaction_row(&persister, &wallet);
    let record = persister
        .get_core_tx_record(wallet, &drifted_txid())
        .expect("recovery must still serve the point read")
        .expect("blob-bearing row must return its record");
    assert_eq!(
        record.height(),
        Some(300),
        "the blob stays authoritative for the returned record"
    );
    assert_eq!(
        transaction_row(&persister, &wallet),
        before,
        "a read must leave the row byte-identical"
    );
}

#[test]
fn get_core_tx_record_drift_is_fatal_under_strict() {
    let wallet = wid(0x25);
    let (persister, _tmp, _path) = fresh_persister();
    seed_drifted_transaction(&persister, &wallet);

    let err = typed(
        persister
            .get_core_tx_record(wallet, &drifted_txid())
            .expect_err("a drifted row must not be served silently under strict"),
    );
    assert!(
        matches!(err, WalletStorageError::CoreTransactionEntryMismatch { .. }),
        "expected CoreTransactionEntryMismatch, got {err:?}"
    );
}

/// `(txid, height, record_blob)` of the wallet's single transaction row.
fn transaction_row(
    persister: &SqlitePersister,
    wallet: &WalletId,
) -> (Vec<u8>, Option<i64>, Vec<u8>) {
    let conn = persister.lock_conn_for_test();
    conn.query_row(
        "SELECT txid, height, record_blob FROM core_transactions WHERE wallet_id = ?1",
        params![wallet.as_slice()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .expect("transaction row must exist")
}

// ── (b) shielded viewing-key row ────────────────────────────────────────

/// One valid viewing key plus one whose blob is a byte short of the fixed
/// 96-byte width.
#[cfg(feature = "shielded")]
fn seed_corrupt_viewing_key(
    persister: &SqlitePersister,
    valid_wallet: &WalletId,
    corrupt_wallet: &WalletId,
) {
    ensure_wallet_meta(persister, valid_wallet);
    ensure_wallet_meta(persister, corrupt_wallet);
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO shielded_viewing_keys (wallet_id, account_index, viewing_key) \
         VALUES (?1, ?2, ?3)",
        params![valid_wallet.as_slice(), 1_i64, &[0xE5_u8; 96]],
    )
    .expect("insert valid viewing key");
    conn.execute(
        "INSERT INTO shielded_viewing_keys (wallet_id, account_index, viewing_key) \
         VALUES (?1, ?2, ?3)",
        params![corrupt_wallet.as_slice(), 2_i64, &[0xF6_u8; 95]],
    )
    .expect("insert corrupt viewing key");
}

#[cfg(feature = "shielded")]
#[test]
fn corrupt_shielded_viewing_key_row_is_fatal_under_strict() {
    let valid_wallet = wid(0x28);
    let corrupt_wallet = wid(0x29);
    let (persister, _tmp, _path) = fresh_persister();
    seed_corrupt_viewing_key(&persister, &valid_wallet, &corrupt_wallet);

    let err = typed(
        persister
            .load()
            .expect_err("a corrupt viewing-key row must abort a strict load"),
    );
    assert!(
        matches!(err, WalletStorageError::BlobDecode { .. }),
        "expected BlobDecode for the short viewing key, got {err:?}"
    );
}

/// Relocated from `sqlite_shielded_viewing_keys.rs`: skipping one corrupt
/// row is now recovery-only behaviour, not the default.
#[cfg(feature = "shielded")]
#[test]
fn corrupt_shielded_viewing_key_row_is_skipped_in_recovery() {
    use platform_wallet::wallet::shielded::SubwalletId;
    let valid_wallet = wid(0x2A);
    let corrupt_wallet = wid(0x2B);
    let (persister, _tmp, _path) = fresh_recovery_persister(|strict| {
        seed_corrupt_viewing_key(strict, &valid_wallet, &corrupt_wallet)
    });

    let state = persister
        .load()
        .expect("one corrupt viewing key must not fail a recovery load");
    assert!(state.wallets.contains_key(&valid_wallet));
    assert!(state.wallets.contains_key(&corrupt_wallet));
    assert_eq!(
        state
            .shielded
            .viewing_keys
            .get(&SubwalletId::new(valid_wallet, 1)),
        Some(&vec![0xE5; 96])
    );
    assert!(!state
        .shielded
        .viewing_keys
        .contains_key(&SubwalletId::new(corrupt_wallet, 2)));
    assert_only_site(&persister, LoadSite::ShieldedViewingKeyRow, 1);
}

// ── (e) used address with two owning accounts ───────────────────────────

/// One script under two `core_address_pool` rows: the used one at account
/// index 1, an unused one at index 0. The pool reader sees only `used = 1`
/// (index 1); the per-script resolver the UTXO reader uses tie-breaks on
/// `account_index ASC` (index 0). Two sources, two answers, same address.
fn seed_conflicting_used_address_owner(persister: &SqlitePersister, wallet: &WalletId) {
    ensure_wallet_meta(persister, wallet);
    // A P2PKH script, so both readers can turn it back into an address.
    let script = {
        let address = dashcore::Address::new(
            dashcore::Network::Testnet,
            dashcore::address::Payload::PubkeyHash(dashcore::PubkeyHash::from_byte_array(
                [0x3Cu8; 20],
            )),
        );
        address.script_pubkey().to_bytes()
    };
    let conn = persister.lock_conn_for_test();
    for (account_index, used) in [(0_i64, 0_i64), (1, 1)] {
        conn.execute(
            "INSERT INTO core_address_pool \
                (wallet_id, account_type, account_index, key_class, pool_type, \
                 address_index, script, used) \
             VALUES (?1, 'standard_bip44', ?2, 0, 0, 0, ?3, ?4)",
            params![wallet.as_slice(), account_index, script.as_slice(), used],
        )
        .expect("seed pool row");
    }
    conn.execute(
        "INSERT INTO core_utxos (wallet_id, outpoint, value, script, spent) \
         VALUES (?1, ?2, 1000, ?3, 1)",
        params![wallet.as_slice(), &[0x11u8; 36][..], script.as_slice()],
    )
    .expect("seed spent utxo carrying the same script");
}

#[test]
fn used_address_owner_conflict_is_fatal_under_strict() {
    let wallet = wid(0x30);
    let (persister, _tmp, _path) = fresh_persister();
    seed_conflicting_used_address_owner(&persister, &wallet);

    let err = typed(
        persister
            .load()
            .expect_err("two owners for one used address must abort a strict load"),
    );
    assert!(
        matches!(err, WalletStorageError::UsedAddressOwnerConflict { .. }),
        "expected UsedAddressOwnerConflict, got {err:?}"
    );
}

#[test]
fn used_address_owner_conflict_is_tolerated_in_recovery() {
    let wallet = wid(0x31);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_conflicting_used_address_owner(strict, &wallet));

    persister.load().expect("recovery must complete the load");
    assert_only_site(&persister, LoadSite::UsedAddressOwnerConflict, 1);
}

// ── (g) unowned identity carrying a registration index ──────────────────

/// Plant an identity with no owning wallet that nonetheless claims a
/// position within one — a row that contradicts itself.
///
/// Written straight to SQLite: `store` refuses to create this state
/// (`WalletlessIdentityIndex`), so only a legacy database predating that
/// check can hold it — which is the state under test. `load_prekeyed`
/// buckets on the index inside `entry_blob`, so the contradiction has to
/// live in the blob, not just the column.
fn seed_self_contradictory_unowned_identity(persister: &SqlitePersister, identity_id: &[u8; 32]) {
    use platform_wallet::changeset::IdentityEntry;
    use platform_wallet::wallet::identity::IdentityStatus;
    use platform_wallet_storage::sqlite::schema::blob;
    let id = dpp::prelude::Identifier::from(*identity_id);
    let entry = IdentityEntry {
        id,
        balance: 0,
        revision: 0,
        // No owning wallet, yet a position within one.
        identity_index: Some(4),
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Unknown,
        wallet_id: None,
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    };
    let payload = blob::encode(&entry).expect("encode unowned identity entry");
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO identities (identity_id, wallet_id, identity_index, entry_blob, tombstoned) \
         VALUES (?1, NULL, 4, ?2, 0)",
        params![id.as_slice(), payload],
    )
    .expect("seed unowned identity carrying a registration index");
}

#[test]
fn unowned_identity_with_registration_index_is_fatal_under_strict() {
    let identity_id = [0x5Au8; 32];
    let (persister, _tmp, _path) = fresh_persister();
    seed_self_contradictory_unowned_identity(&persister, &identity_id);

    let err = persister
        .load_unowned_identities()
        .expect_err("a self-contradictory identity row must not be served under strict");
    assert!(
        matches!(
            err,
            WalletStorageError::UnownedIdentityHasRegistrationIndex {
                identity_index: 4,
                ..
            }
        ),
        "expected UnownedIdentityHasRegistrationIndex, got {err:?}"
    );
}

#[test]
fn unowned_identity_with_registration_index_is_tolerated_in_recovery() {
    let identity_id = [0x5Bu8; 32];
    let (persister, _tmp, _path) = fresh_recovery_persister(|strict| {
        seed_self_contradictory_unowned_identity(strict, &identity_id)
    });

    let unowned = persister
        .load_unowned_identities()
        .expect("recovery must return the identity anyway");
    assert!(
        unowned.contains_key(&dpp::prelude::Identifier::from(identity_id)),
        "the identity must still be reachable for rescue"
    );
    assert_only_site(&persister, LoadSite::UnownedIdentityHasRegistrationIndex, 1);
}

#[test]
fn load_unowned_identities_adds_to_the_load_snapshot_instead_of_replacing_it() {
    let wallet = wid(0x2C);
    let identity_id = [0x5Cu8; 32];
    let (persister, _tmp, _path) = fresh_recovery_persister(|strict| {
        seed_corrupt_chain_lock(strict, &wallet);
        seed_self_contradictory_unowned_identity(strict, &identity_id);
    });

    persister.load().expect("recovery load");
    persister
        .load_unowned_identities()
        .expect("recovery unowned read");

    let degradation = persister.last_load_degradation();
    assert_eq!(
        degradation.by_site.get(&LoadSite::ChainLockBlob).copied(),
        Some(1),
        "the load()'s own tally must survive: {:?}",
        degradation.by_site
    );
    assert_eq!(
        degradation
            .by_site
            .get(&LoadSite::UnownedIdentityHasRegistrationIndex)
            .copied(),
        Some(1),
        "the unowned read must fold in: {:?}",
        degradation.by_site
    );
    assert_eq!(degradation.total, 2);
}

// ── (h) rows in tables `load()` has no reader for ───────────────────────

#[test]
fn unimplemented_rows_are_counted_without_setting_degraded() {
    let wallet = wid(0x33);
    let identity_id = [0x6Au8; 32];
    let (persister, _tmp, _path) = fresh_persister();
    ensure_wallet_meta(&persister, &wallet);
    common::ensure_identity(&persister, &identity_id, Some(&wallet));
    common::ensure_token_balance(&persister, &identity_id, &[0x6Bu8; 32]);

    persister.load().expect("clean load");
    let degradation = persister.last_load_degradation();
    assert_eq!(
        degradation.unimplemented_rows, 1,
        "the un-rehydrated token balance must be reported"
    );
    assert!(
        !degradation.degraded,
        "intact-but-unread rows are not a degradation"
    );
    assert_eq!(degradation.total, 0);
}

/// The point read `get_core_tx_record` tolerates drift without tallying it
/// — one context per transaction folded into a per-load snapshot would grow
/// without bound. Pinned so the limitation stays a decision instead of
/// becoming a regression someone "fixes" in either direction.
#[test]
fn get_core_tx_record_drift_leaves_the_load_snapshot_alone() {
    let wallet = wid(0x2E);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_drifted_transaction(strict, &wallet));

    persister
        .get_core_tx_record(wallet, &drifted_txid())
        .expect("recovery must serve the point read")
        .expect("blob-bearing row must return its record");
    assert!(
        !persister.is_degraded(),
        "a point read must not degrade a persister that never loaded: {:?}",
        persister.last_load_degradation()
    );

    persister.load().expect("recovery load");
    let after_load = persister.last_load_degradation();
    persister
        .get_core_tx_record(wallet, &drifted_txid())
        .expect("recovery must serve the point read")
        .expect("blob-bearing row must return its record");
    assert_eq!(
        persister.last_load_degradation(),
        after_load,
        "a point read must not move the snapshot the last load left"
    );
}

/// The snapshot rustdoc promises "a database restored from a backup and
/// reloaded clean reports clean". Loading the same dirty database twice
/// cannot tell replacement apart from "keep whichever was worse"; only the
/// dirty → repaired transition can.
#[test]
fn a_repaired_database_reloads_clean() {
    let wallet = wid(0x2F);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_corrupt_chain_lock(strict, &wallet));

    persister.load().expect("first recovery load");
    assert_only_site(&persister, LoadSite::ChainLockBlob, 1);

    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "UPDATE core_sync_state SET last_applied_chain_lock = NULL WHERE wallet_id = ?1",
            params![wallet.as_slice()],
        )
        .expect("repair the undecodable chain lock");
    }

    persister.load().expect("reload after repair");
    let degradation = persister.last_load_degradation();
    assert!(
        !persister.is_degraded(),
        "a repaired database must report clean: {degradation:?}"
    );
    assert!(degradation.by_site.is_empty());
    assert_eq!(degradation.total, 0);
}

/// A load that returns `Err` leaves the snapshot empty — the error is the
/// verdict, not a partial tally. `BlobTooLarge` is the one inconsistency
/// recovery mode still refuses (an allocation guard cannot be waived), so
/// it is what proves the sites tolerated on the way there are discarded.
#[test]
fn a_fatal_blob_in_recovery_discards_the_sites_tolerated_before_it() {
    // `load()` walks wallets in `ORDER BY wallet_id`, so the tolerable
    // wallet is reached first and its site is counted before the fatal one.
    let tolerated = wid(0x01);
    let fatal = wid(0xF0);
    let (persister, _tmp, _path) = fresh_recovery_persister(|strict| {
        seed_corrupt_chain_lock(strict, &tolerated);
        seed_corrupt_chain_lock(strict, &fatal);
        let conn = strict.lock_conn_for_test();
        conn.execute(
            "UPDATE core_sync_state SET last_applied_chain_lock = ?1 WHERE wallet_id = ?2",
            params![
                vec![0u8; platform_wallet_storage::SIZE_LIMIT_BYTES + 1].as_slice(),
                fatal.as_slice()
            ],
        )
        .expect("plant an oversize chain lock");
    });

    let err = typed(
        persister
            .load()
            .expect_err("an oversize blob stays fatal in recovery mode"),
    );
    assert!(
        matches!(err, WalletStorageError::BlobTooLarge { .. }),
        "expected BlobTooLarge, got {err:?}"
    );
    assert_eq!(
        persister.last_load_degradation(),
        platform_wallet_storage::LoadDegradation::default(),
        "a failed load must leave no partial tally behind"
    );
}

// ── flag semantics ──────────────────────────────────────────────────────

#[test]
fn degraded_flag_is_false_on_a_clean_load() {
    let wallet = wid(0x26);
    let (persister, _tmp, _path) = fresh_persister();
    ensure_wallet_meta(&persister, &wallet);

    persister.load().expect("clean load");
    assert!(!persister.is_degraded());
    assert_eq!(persister.last_load_degradation().total, 0);
    assert!(persister.last_load_degradation().by_site.is_empty());
}

#[test]
fn degraded_counts_are_per_load_not_cumulative() {
    let wallet = wid(0x27);
    let (persister, _tmp, _path) =
        fresh_recovery_persister(|strict| seed_corrupt_chain_lock(strict, &wallet));

    persister.load().expect("first recovery load");
    assert_only_site(&persister, LoadSite::ChainLockBlob, 1);
    persister.load().expect("second recovery load");
    assert_only_site(
        &persister,
        LoadSite::ChainLockBlob,
        1, // replaced, not summed — otherwise a repaired DB could never read clean
    );
}
