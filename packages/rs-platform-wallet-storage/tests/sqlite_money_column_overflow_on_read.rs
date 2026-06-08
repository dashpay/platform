#![allow(clippy::field_reassign_with_default)]

//! Money/balance columns: a negative `i64` stored on disk (a wrapped
//! value from a pre-`safe_cast` build, a restored-corruption row, or a
//! torn write that passes `PRAGMA integrity_check`) MUST abort the read
//! with [`WalletStorageError::IntegerOverflow`] rather than
//! sign-extending into a multi-quintillion `u64` balance.
//!
//! The crate-level invariant ("Every cross-boundary cast in the writer
//! / reader paths runs through one of these helpers and produces a typed
//! `IntegerOverflow` on out-of-range input" — `src/sqlite/util/safe_cast.rs`)
//! is asserted for `birth_height` and `sync_height` in
//! `sqlite_structural_hardening.rs`, but NOT for the genuine
//! value-bearing columns. These are the casts where a silent wrap is a
//! money-correctness corruption-on-load, so they get their own guard
//! tests here. The `platform_addresses.balance` case rides the
//! production `load()` path end-to-end.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet_storage::WalletStorageError;
use rusqlite::params;

/// `platform_addresses.balance`: a negative on-disk value must abort
/// the production `load()` with `IntegerOverflow{field:
/// "platform_addresses.balance"}`, NOT load a sign-extended u64
/// balance. This rides `load() -> platform_addrs::load_all ->
/// decode_address_row -> i64_to_u64`.
#[test]
fn platform_address_balance_negative_on_disk_errors_on_load() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE1);
    ensure_wallet_meta(&persister, &w);
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO platform_addresses \
                (wallet_id, account_index, address_index, address, balance, nonce) \
             VALUES (?1, 0, 0, X'0000000000000000000000000000000000000000', ?2, 0)",
            params![w.as_slice(), -1i64],
        )
        .unwrap();
    }

    let err = PlatformWalletPersistence::load(&persister)
        .expect_err("a negative on-disk balance must abort load(), not sign-extend");
    let backend = format!("{err:?}");
    assert!(
        backend.contains("IntegerOverflow") && backend.contains("platform_addresses.balance"),
        "expected IntegerOverflow for platform_addresses.balance, got {backend}"
    );
}

/// `core_utxos.value`: a negative on-disk value must abort the unspent
/// read with `IntegerOverflow{field: "core_utxos.value"}` rather than
/// reporting a sign-extended u64 amount for live funds.
#[test]
fn core_utxo_value_negative_on_disk_errors_on_read() {
    use dashcore::hashes::Hash;
    use platform_wallet_storage::sqlite::schema::{blob, core_state};
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE2);
    ensure_wallet_meta(&persister, &w);
    let outpoint = blob::encode_outpoint(&dashcore::OutPoint {
        txid: dashcore::Txid::from_byte_array([0x22; 32]),
        vout: 0,
    })
    .unwrap();
    {
        let conn = persister.lock_conn_for_test();
        // Declare the address so the row is treated as a real, unspent
        // UTXO and the value cast is reached (account_index 0).
        conn.execute(
            "INSERT INTO core_utxos \
                (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid) \
             VALUES (?1, ?2, ?3, X'00', NULL, 0, 0, NULL)",
            params![w.as_slice(), &outpoint, -1i64],
        )
        .unwrap();
    }
    let conn = persister.lock_conn_for_test();
    let err = core_state::list_unspent_utxos(&conn, &w)
        .expect_err("a negative on-disk utxo value must error, not sign-extend");
    let s = format!("{err:?}");
    assert!(
        matches!(err, WalletStorageError::IntegerOverflow { field, .. } if field == "core_utxos.value"),
        "expected IntegerOverflow for core_utxos.value, got {s}"
    );
}
