#![allow(clippy::field_reassign_with_default)]

//! `delete_wallet` pre-flush apply failure driven by a REAL SQL error,
//! exercising the apply / commit restore branches a test injector cannot.
//!
//! Strategy: buffer a wallet's full changeset in `Manual` mode, then drop
//! a child table the pre-flush apply needs (`core_sync_state`) via a side
//! connection. The delete's pre-flush `apply_changeset_to_tx` then hits a
//! real "no such table" failure on the core-state INSERT, and the
//! buffered changeset MUST be restored (not lost).

mod common;

use common::wid;
use key_wallet::Network;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet_storage::{FlushMode, SqlitePersister, SqlitePersisterConfig};

fn full_changeset(synced: u32) -> PlatformWalletChangeSet {
    let mut cs = PlatformWalletChangeSet::default();
    cs.wallet_metadata = Some(WalletMetadataEntry {
        network: Network::Testnet,
        wallet_group_id: [0u8; 32],
        birth_height: 0,
    });
    cs.core = Some(CoreChangeSet {
        synced_height: Some(synced),
        last_processed_height: Some(synced),
        ..Default::default()
    });
    cs
}

#[test]
fn delete_wallet_pre_flush_apply_real_sql_failure_restores_buffer() {
    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let cfg = SqlitePersisterConfig::new(&path).with_flush_mode(FlushMode::Manual);
    let persister = SqlitePersister::open(cfg).unwrap();
    let w = wid(0xC7);

    // Buffer a brand-new wallet (state lives only in the buffer).
    persister.store(w, full_changeset(21)).unwrap();
    assert!(
        persister.buffer_has_changeset_for_test(&w),
        "precondition: changeset is buffered"
    );

    // Drop the table the pre-flush apply will INSERT into. Use a side
    // connection so the persister's own conn is untouched until delete.
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("DROP TABLE core_sync_state", []).unwrap();
    }

    // delete_wallet drains the buffer, opens the pre-flush EXCLUSIVE tx,
    // and applies the changeset — the core-state INSERT now fails with a
    // real "no such table" SQL error.
    let err = persister.delete_wallet_skip_backup(w);
    assert!(
        err.is_err(),
        "delete must fail when the pre-flush apply hits a real SQL error; got {err:?}"
    );

    // The buffered changeset MUST survive the failed delete — the
    // apply-branch restore put it back.
    assert!(
        persister.buffer_has_changeset_for_test(&w),
        "buffered changeset must be restored after a real pre-flush apply failure"
    );
}
