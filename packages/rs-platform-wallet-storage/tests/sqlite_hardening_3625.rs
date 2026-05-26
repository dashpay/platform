#![allow(clippy::field_reassign_with_default)]

//! #3625 structural hardening pass.
//!
//! Native FK rejection (orphan child + mixed-wallet platform addr),
//! multi-account UTXO bucketing (CMT-003/011), identity-key typed-vs-blob
//! consistency (CMT-004), the truncation guards (CMT-012/014), and the
//! compaction-marker-only load gate (CMT-002).

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};

use dashcore::{Address, Network, OutPoint, TxOut, Txid};
use key_wallet::Utxo;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformAddressBalanceEntry, PlatformAddressChangeSet, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::WalletStorageError;
use rusqlite::params;

/// CMT-001: a child insert without a `wallet_metadata` parent is
/// rejected by the native FK (not a trigger).
#[test]
fn native_fk_rejects_orphan_child() {
    let (persister, _tmp, _path) = fresh_persister();
    let conn = persister.lock_conn_for_test();
    let res = conn.execute(
        "INSERT INTO identities (wallet_id, wallet_index, identity_id, entry_blob, tombstoned) \
         VALUES (?1, NULL, ?2, X'00', 0)",
        params![[7u8; 32].as_slice(), [9u8; 32].as_slice()],
    );
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("FOREIGN KEY"),
        "expected FOREIGN KEY failure, got `{err}`"
    );
}

/// CMT-001: an `identity_keys` row whose `identities` parent does not
/// exist is rejected by the composite FK to `identities`.
#[test]
fn native_fk_rejects_identity_keys_without_identity() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA1);
    ensure_wallet_meta(&persister, &w);
    let conn = persister.lock_conn_for_test();
    let res = conn.execute(
        "INSERT INTO identity_keys \
            (wallet_id, identity_id, key_id, public_key_blob, public_key_hash, derivation_blob) \
         VALUES (?1, ?2, 0, X'00', X'00', NULL)",
        params![w.as_slice(), [3u8; 32].as_slice()],
    );
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("FOREIGN KEY"),
        "expected FOREIGN KEY failure on missing identity parent, got `{err}`"
    );
}

/// CMT-005: a `platform_addresses` entry naming a different wallet than
/// the flush scope fails fast with the typed `WalletIdMismatch`.
#[test]
fn platform_addr_mixed_wallet_rejected() {
    use key_wallet::PlatformP2PKHAddress;
    let (persister, _tmp, _path) = fresh_persister();
    let scope = wid(0xB1);
    let other = wid(0xB2);
    ensure_wallet_meta(&persister, &scope);
    use dash_sdk::platform::address_sync::AddressFunds;
    let mut cs = PlatformWalletChangeSet::default();
    cs.platform_addresses = Some(PlatformAddressChangeSet {
        addresses: vec![PlatformAddressBalanceEntry {
            wallet_id: other,
            account_index: 0,
            address_index: 0,
            address: PlatformP2PKHAddress::new([1u8; 20]),
            funds: AddressFunds {
                nonce: 0,
                balance: 0,
            },
        }],
        ..Default::default()
    });
    // Immediate flush mode surfaces the typed error from `store`.
    let err = persister
        .store(scope, cs)
        .expect_err("mixed-wallet must fail");
    assert!(
        format!("{err}").contains("wallet id mismatch"),
        "expected wallet id mismatch, got `{err}`"
    );
}

fn p2pkh(byte: u8) -> Address {
    use dashcore::address::Payload;
    use dashcore::hashes::Hash;
    use dashcore::PubkeyHash;
    let hash = PubkeyHash::from_byte_array([byte; 20]);
    Address::new(Network::Testnet, Payload::PubkeyHash(hash))
}

fn make_utxo(addr: &Address, vout: u32, value: u64) -> Utxo {
    let outpoint = OutPoint::new(
        Txid::from_raw_hash(dashcore::hashes::Hash::all_zeros()),
        vout,
    );
    let txout = TxOut {
        value,
        script_pubkey: addr.script_pubkey(),
    };
    Utxo::new(outpoint, txout, addr.clone(), 10, false)
}

/// CMT-003/011: UTXOs resolve their real `account_index` from the
/// derived-address map written earlier in the same transaction, instead
/// of a hardcoded 0. A UTXO on an undeclared address defaults to 0.
#[test]
fn multi_account_utxos_bucket_to_real_account() {
    use platform_wallet_storage::sqlite::schema::core_state;

    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xC7);
    ensure_wallet_meta(&persister, &w);

    let addr_acct5 = p2pkh(0x05);
    let addr_acct9 = p2pkh(0x09);
    let addr_unknown = p2pkh(0xEE);

    {
        let mut conn = persister.lock_conn_for_test();
        // Pre-seed the derived-address map with two distinct accounts.
        for (acct, addr) in [(5u32, &addr_acct5), (9u32, &addr_acct9)] {
            conn.execute(
                "INSERT INTO core_derived_addresses \
                    (wallet_id, account_type, account_index, address, derivation_path, used) \
                 VALUES (?1, 'standard', ?2, ?3, '0/0', 0)",
                params![w.as_slice(), acct as i64, addr.to_string()],
            )
            .unwrap();
        }

        let cs = CoreChangeSet {
            new_utxos: vec![
                make_utxo(&addr_acct5, 0, 1000),
                make_utxo(&addr_acct9, 1, 2000),
                make_utxo(&addr_unknown, 2, 3000),
            ],
            ..Default::default()
        };
        let tx = conn.transaction().unwrap();
        core_state::apply(&tx, &w, &cs).unwrap();
        tx.commit().unwrap();
    }

    let conn = persister.lock_conn_for_test();
    let by_account = core_state::list_unspent_utxos(&conn, &w).unwrap();
    assert_eq!(
        by_account.get(&5).map(|v| v.len()),
        Some(1),
        "account 5 should hold exactly one UTXO"
    );
    assert_eq!(
        by_account.get(&9).map(|v| v.len()),
        Some(1),
        "account 9 should hold exactly one UTXO"
    );
    // The undeclared address falls back to account 0.
    assert_eq!(
        by_account.get(&0).map(|v| v.len()),
        Some(1),
        "undeclared address should default to account 0"
    );
}

/// CMT-014: an out-of-range `birth_height` errors rather than truncating.
#[test]
fn birth_height_overflow_errors_not_truncates() {
    use platform_wallet_storage::sqlite::schema::wallet_meta;
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xD1);
    {
        let conn = persister.lock_conn_for_test();
        // 1<<40 overflows u32 but fits the i64 column.
        conn.execute(
            "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', ?2)",
            params![w.as_slice(), 1_099_511_627_776i64],
        )
        .unwrap();
    }
    let conn = persister.lock_conn_for_test();
    let err = wallet_meta::fetch(&conn, &w).expect_err("overflow must error");
    assert!(
        matches!(err, WalletStorageError::IntegerOverflow { .. }),
        "expected IntegerOverflow, got {err:?}"
    );
}

/// CMT-012: an out-of-range stored sync height errors rather than
/// truncating during the monotonic-max read.
#[test]
fn sync_height_overflow_errors_not_truncates() {
    use platform_wallet_storage::sqlite::schema::core_state;
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xD2);
    ensure_wallet_meta(&persister, &w);
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) \
             VALUES (?1, ?2, NULL)",
            params![w.as_slice(), 1_099_511_627_776i64],
        )
        .unwrap();
    }
    let mut conn = persister.lock_conn_for_test();
    let tx = conn.transaction().unwrap();
    let cs = CoreChangeSet {
        synced_height: Some(5),
        ..Default::default()
    };
    let err = core_state::apply(&tx, &w, &cs).expect_err("overflow must error");
    assert!(
        matches!(err, WalletStorageError::IntegerOverflow { .. }),
        "expected IntegerOverflow, got {err:?}"
    );
}

/// CMT-004: an `identity_keys` upsert whose entry fields disagree with
/// its map key is rejected, so the typed columns and serialized blob
/// can never describe different rows.
#[test]
fn identity_key_entry_mismatch_rejected() {
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use platform_wallet::changeset::{IdentityKeyEntry, IdentityKeysChangeSet};

    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xF4);
    ensure_wallet_meta(&persister, &w);

    let key_identity = Identifier::from([0xAA; 32]);
    let entry_identity = Identifier::from([0xBB; 32]); // deliberately different
    let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::HIGH,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(vec![2u8; 33]),
        disabled_at: None,
    });
    let entry = IdentityKeyEntry {
        identity_id: entry_identity,
        key_id: 0,
        public_key,
        public_key_hash: [3u8; 20],
        wallet_id: Some(w),
        derivation_indices: None,
    };
    let mut keys = IdentityKeysChangeSet::default();
    keys.upserts.insert((key_identity, 0), entry);

    // Immediate flush mode surfaces the typed error from `store`.
    let err = persister
        .store(
            w,
            PlatformWalletChangeSet {
                identity_keys: Some(keys),
                ..Default::default()
            },
        )
        .expect_err("mismatch must fail");
    assert!(
        format!("{err}").contains("disagree"),
        "expected identity key entry mismatch, got `{err}`"
    );
}

/// CMT-007: an asset_locks row whose lifecycle blob disagrees with the
/// typed `account_index` column is rejected at decode time with the
/// typed `AssetLockEntryMismatch` rather than silently mis-bucketing.
#[test]
fn asset_lock_typed_vs_blob_mismatch_rejected() {
    use dashcore::Transaction as DashTx;
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use platform_wallet::changeset::AssetLockEntry;
    use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
    use platform_wallet_storage::sqlite::schema::{asset_locks, blob};

    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xF7);
    ensure_wallet_meta(&persister, &w);

    // Forge a blob whose internal account_index disagrees with the
    // typed column we'll insert it under.
    let outpoint = OutPoint::new(Txid::from_raw_hash(dashcore::hashes::Hash::all_zeros()), 7);
    let entry = AssetLockEntry {
        out_point: outpoint,
        transaction: DashTx {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        },
        account_index: 99, // blob says 99
        funding_type: AssetLockFundingType::IdentityTopUp,
        identity_index: 0,
        amount_duffs: 1,
        status: AssetLockStatus::Built,
        proof: None,
    };
    let lifecycle_blob = blob::encode(&entry).unwrap();
    let op_bytes = blob::encode_outpoint(&outpoint);

    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO asset_locks (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
             VALUES (?1, ?2, 'built', ?3, 0, 1, ?4)",
            params![w.as_slice(), &op_bytes[..], 5i64 /* typed says 5, blob says 99 */, lifecycle_blob],
        )
        .unwrap();
    }

    let conn = persister.lock_conn_for_test();
    let err = asset_locks::load_state(&conn, &w).expect_err("mismatch must fail");
    assert!(
        matches!(err, WalletStorageError::AssetLockEntryMismatch { .. }),
        "expected AssetLockEntryMismatch, got {err:?}"
    );
}

/// CMT-002: a wallet whose only platform-address state is the
/// compaction marker (`last_known_recent_block > 0`) is kept by `load`,
/// not silently dropped.
#[test]
fn load_keeps_compaction_marker_only_wallet() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE3);
    ensure_wallet_meta(&persister, &w);
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO platform_address_sync \
                (wallet_id, sync_height, sync_timestamp, last_known_recent_block) \
             VALUES (?1, 0, 0, 42)",
            params![w.as_slice()],
        )
        .unwrap();
    }
    let state = PlatformWalletPersistence::load(&persister).expect("load");
    assert!(
        state.platform_addresses.contains_key(&w),
        "compaction-marker-only wallet must be retained"
    );
    assert_eq!(
        state.platform_addresses[&w].last_known_recent_block, 42,
        "marker value should round-trip"
    );
}
