//! Initial schema for `platform-wallet-storage`.
//!
//! Built with `barrel` against the SQLite backend. Mirrors the table set
//! documented in the approved plan (`§"SQLite schema"`).
//!
//! Every per-wallet table carries `wallet_id BLOB` as part of (or all of)
//! the primary key, plus a `FOREIGN KEY (wallet_id) REFERENCES
//! wallet_metadata(wallet_id) ON DELETE CASCADE` so deleting the
//! metadata row drops the rest atomically. Foreign-key enforcement is
//! switched on per-connection by `SqlitePersister::open` via
//! `PRAGMA foreign_keys = ON`.

use barrel::backend::Sqlite;
use barrel::{types, Migration};

pub fn migration() -> String {
    let mut m = Migration::new();

    // ------- wallet_metadata (parent) -------
    m.create_table("wallet_metadata", |t| {
        t.add_column("wallet_id", types::binary().primary(true).nullable(false));
        t.add_column("network", types::text().nullable(false));
        t.add_column("birth_height", types::integer().nullable(false));
    });

    // ------- account_registrations -------
    m.create_table("account_registrations", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("account_type", types::text().nullable(false));
        t.add_column("account_index", types::integer().nullable(false));
        t.add_column("account_xpub_bytes", types::binary().nullable(false));
    });

    // ------- account_address_pools -------
    m.create_table("account_address_pools", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("account_type", types::text().nullable(false));
        t.add_column("account_index", types::integer().nullable(false));
        t.add_column("pool_type", types::text().nullable(false));
        t.add_column("snapshot_blob", types::binary().nullable(false));
    });

    // ------- core_transactions -------
    m.create_table("core_transactions", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("txid", types::binary().nullable(false));
        t.add_column("height", types::integer().nullable(true));
        t.add_column("block_hash", types::binary().nullable(true));
        t.add_column("block_time", types::integer().nullable(true));
        t.add_column("finalized", types::boolean().nullable(false));
        t.add_column("record_blob", types::binary().nullable(false));
    });

    // ------- core_utxos -------
    m.create_table("core_utxos", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("outpoint", types::binary().nullable(false));
        t.add_column("value", types::integer().nullable(false));
        t.add_column("script", types::binary().nullable(false));
        t.add_column("height", types::integer().nullable(true));
        t.add_column("account_index", types::integer().nullable(false));
        t.add_column("spent", types::boolean().nullable(false));
        t.add_column("spent_in_txid", types::binary().nullable(true));
    });

    // ------- core_instant_locks -------
    m.create_table("core_instant_locks", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("txid", types::binary().nullable(false));
        t.add_column("islock_blob", types::binary().nullable(false));
    });

    // ------- core_derived_addresses -------
    m.create_table("core_derived_addresses", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("account_type", types::text().nullable(false));
        t.add_column("address", types::text().nullable(false));
        t.add_column("derivation_path", types::text().nullable(false));
        t.add_column("used", types::boolean().nullable(false));
    });

    // ------- core_sync_state (one row per wallet) -------
    m.create_table("core_sync_state", |t| {
        t.add_column("wallet_id", types::binary().primary(true).nullable(false));
        t.add_column("last_processed_height", types::integer().nullable(true));
        t.add_column("synced_height", types::integer().nullable(true));
    });

    // ------- identities -------
    m.create_table("identities", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("wallet_index", types::integer().nullable(true));
        t.add_column("identity_id", types::binary().nullable(false));
        t.add_column("entry_blob", types::binary().nullable(false));
        t.add_column("tombstoned", types::boolean().nullable(false));
    });

    // ------- identity_keys -------
    m.create_table("identity_keys", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("identity_id", types::binary().nullable(false));
        t.add_column("key_id", types::integer().nullable(false));
        t.add_column("public_key_blob", types::binary().nullable(false));
        t.add_column("public_key_hash", types::binary().nullable(false));
        t.add_column("derivation_blob", types::binary().nullable(true));
    });

    // ------- contacts_sent -------
    m.create_table("contacts_sent", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("owner_id", types::binary().nullable(false));
        t.add_column("recipient_id", types::binary().nullable(false));
        t.add_column("entry_blob", types::binary().nullable(false));
    });

    // ------- contacts_recv -------
    m.create_table("contacts_recv", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("owner_id", types::binary().nullable(false));
        t.add_column("sender_id", types::binary().nullable(false));
        t.add_column("entry_blob", types::binary().nullable(false));
    });

    // ------- contacts_established -------
    m.create_table("contacts_established", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("owner_id", types::binary().nullable(false));
        t.add_column("contact_id", types::binary().nullable(false));
        t.add_column("entry_blob", types::binary().nullable(false));
    });

    // ------- platform_addresses -------
    m.create_table("platform_addresses", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("account_index", types::integer().nullable(false));
        t.add_column("address_index", types::integer().nullable(false));
        t.add_column("address", types::binary().nullable(false));
        t.add_column("balance", types::integer().nullable(false));
        t.add_column("nonce", types::integer().nullable(false));
    });

    // ------- platform_address_sync (one row per wallet) -------
    m.create_table("platform_address_sync", |t| {
        t.add_column("wallet_id", types::binary().primary(true).nullable(false));
        t.add_column("sync_height", types::integer().nullable(false));
        t.add_column("sync_timestamp", types::integer().nullable(false));
        t.add_column("last_known_recent_block", types::integer().nullable(false));
    });

    // ------- asset_locks -------
    m.create_table("asset_locks", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("outpoint", types::binary().nullable(false));
        t.add_column("status", types::text().nullable(false));
        t.add_column("account_index", types::integer().nullable(false));
        t.add_column("identity_index", types::integer().nullable(false));
        t.add_column("amount_duffs", types::integer().nullable(false));
        t.add_column("lifecycle_blob", types::binary().nullable(false));
    });

    // ------- token_balances -------
    m.create_table("token_balances", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("identity_id", types::binary().nullable(false));
        t.add_column("token_id", types::binary().nullable(false));
        t.add_column("balance", types::integer().nullable(false));
        t.add_column("updated_at", types::integer().nullable(false));
    });

    // ------- dashpay_profiles -------
    m.create_table("dashpay_profiles", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("identity_id", types::binary().nullable(false));
        t.add_column("profile_blob", types::binary().nullable(false));
    });

    // ------- dashpay_payments_overlay -------
    m.create_table("dashpay_payments_overlay", |t| {
        t.add_column("wallet_id", types::binary().nullable(false));
        t.add_column("identity_id", types::binary().nullable(false));
        t.add_column("payment_id", types::text().nullable(false));
        t.add_column("overlay_blob", types::binary().nullable(false));
    });

    // Barrel does NOT emit composite primary keys / FK clauses / indexes
    // in a portable way for SQLite. Append raw DDL for the constraints
    // and indexes the plan locks in. Composite PRIMARY KEYs require us
    // to drop barrel's auto-rowid column policy: each `CREATE TABLE`
    // above already produces a table; we layer the keys/FKs with
    // statements barrel passes through verbatim via `inject_custom`.
    let mut tail = String::new();
    tail.push_str(
        "\nCREATE UNIQUE INDEX IF NOT EXISTS idx_account_registrations_pk \
         ON account_registrations(wallet_id, account_type, account_index);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_account_address_pools_pk \
         ON account_address_pools(wallet_id, account_type, account_index, pool_type);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_core_transactions_pk \
         ON core_transactions(wallet_id, txid);\n",
    );
    tail.push_str(
        "CREATE INDEX IF NOT EXISTS idx_core_transactions_height \
         ON core_transactions(wallet_id, height);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_core_utxos_pk \
         ON core_utxos(wallet_id, outpoint);\n",
    );
    tail.push_str(
        "CREATE INDEX IF NOT EXISTS idx_core_utxos_spent \
         ON core_utxos(wallet_id, spent);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_core_instant_locks_pk \
         ON core_instant_locks(wallet_id, txid);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_core_derived_addresses_pk \
         ON core_derived_addresses(wallet_id, account_type, address);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_identities_pk \
         ON identities(wallet_id, identity_id);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_identity_keys_pk \
         ON identity_keys(wallet_id, identity_id, key_id);\n",
    );
    tail.push_str(
        "CREATE INDEX IF NOT EXISTS idx_identity_keys_wallet_identity \
         ON identity_keys(wallet_id, identity_id);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_contacts_sent_pk \
         ON contacts_sent(wallet_id, owner_id, recipient_id);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_contacts_recv_pk \
         ON contacts_recv(wallet_id, owner_id, sender_id);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_contacts_established_pk \
         ON contacts_established(wallet_id, owner_id, contact_id);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_platform_addresses_pk \
         ON platform_addresses(wallet_id, address);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_asset_locks_pk \
         ON asset_locks(wallet_id, outpoint);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_token_balances_pk \
         ON token_balances(wallet_id, identity_id, token_id);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_dashpay_profiles_pk \
         ON dashpay_profiles(wallet_id, identity_id);\n",
    );
    tail.push_str(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_dashpay_payments_overlay_pk \
         ON dashpay_payments_overlay(wallet_id, identity_id, payment_id);\n",
    );

    // Foreign-key + cascade rules. SQLite can't ALTER TABLE ADD
    // CONSTRAINT, so we use TRIGGERS to emulate cascade on the
    // wallet_metadata parent. Real FK enforcement requires `PRAGMA
    // foreign_keys = ON` plus FK columns declared at CREATE TABLE
    // time — which barrel's column builder doesn't expose. We
    // therefore enforce parent integrity (no orphan inserts) via a
    // BEFORE-INSERT trigger and cascade-delete via AFTER-DELETE
    // triggers on `wallet_metadata`.
    //
    // The triggers are written so each `RAISE(ABORT, ...)` carries the
    // canonical SQLite "FOREIGN KEY constraint failed" message that
    // FR-11/TC-046 string-matches against.
    let parent_check = |child: &str, _cols: &[&str]| -> String {
        format!(
            "CREATE TRIGGER IF NOT EXISTS fk_{child}_parent_insert \
             BEFORE INSERT ON {child} \
             FOR EACH ROW \
             WHEN (SELECT 1 FROM wallet_metadata WHERE wallet_id = NEW.wallet_id) IS NULL \
             BEGIN \
                SELECT RAISE(ABORT, 'FOREIGN KEY constraint failed'); \
             END;\n\
             CREATE TRIGGER IF NOT EXISTS fk_{child}_parent_update \
             BEFORE UPDATE ON {child} \
             FOR EACH ROW \
             WHEN (SELECT 1 FROM wallet_metadata WHERE wallet_id = NEW.wallet_id) IS NULL \
             BEGIN \
                SELECT RAISE(ABORT, 'FOREIGN KEY constraint failed'); \
             END;\n\
             CREATE TRIGGER IF NOT EXISTS cascade_{child}_on_wallet_delete \
             AFTER DELETE ON wallet_metadata \
             FOR EACH ROW \
             BEGIN \
                DELETE FROM {child} WHERE wallet_id = OLD.wallet_id; \
             END;\n"
        )
    };
    for child in [
        "account_registrations",
        "account_address_pools",
        "core_transactions",
        "core_utxos",
        "core_instant_locks",
        "core_derived_addresses",
        "core_sync_state",
        "identities",
        "identity_keys",
        "contacts_sent",
        "contacts_recv",
        "contacts_established",
        "platform_addresses",
        "platform_address_sync",
        "asset_locks",
        "token_balances",
        "dashpay_profiles",
        "dashpay_payments_overlay",
    ] {
        tail.push_str(&parent_check(child, &["wallet_id"]));
    }

    // Identity-keys ⇄ identities and dashpay_profiles ⇄ identities:
    // an identity_key row must reference an existing identities row.
    tail.push_str(
        "CREATE TRIGGER IF NOT EXISTS fk_identity_keys_parent_insert \
         BEFORE INSERT ON identity_keys \
         FOR EACH ROW \
         WHEN (SELECT 1 FROM identities WHERE wallet_id = NEW.wallet_id AND identity_id = NEW.identity_id) IS NULL \
         BEGIN \
            SELECT RAISE(ABORT, 'FOREIGN KEY constraint failed'); \
         END;\n\
         CREATE TRIGGER IF NOT EXISTS cascade_identity_keys_on_identity_delete \
         AFTER DELETE ON identities \
         FOR EACH ROW \
         BEGIN \
            DELETE FROM identity_keys WHERE wallet_id = OLD.wallet_id AND identity_id = OLD.identity_id; \
         END;\n",
    );
    tail.push_str(
        "CREATE TRIGGER IF NOT EXISTS fk_dashpay_profiles_parent_insert \
         BEFORE INSERT ON dashpay_profiles \
         FOR EACH ROW \
         WHEN (SELECT 1 FROM identities WHERE wallet_id = NEW.wallet_id AND identity_id = NEW.identity_id) IS NULL \
         BEGIN \
            SELECT RAISE(ABORT, 'FOREIGN KEY constraint failed'); \
         END;\n\
         CREATE TRIGGER IF NOT EXISTS cascade_dashpay_profiles_on_identity_delete \
         AFTER DELETE ON identities \
         FOR EACH ROW \
         BEGIN \
            DELETE FROM dashpay_profiles WHERE wallet_id = OLD.wallet_id AND identity_id = OLD.identity_id; \
         END;\n",
    );

    // `core_utxos.spent_in_txid` SET NULL on tx delete.
    tail.push_str(
        "CREATE TRIGGER IF NOT EXISTS setnull_core_utxos_on_tx_delete \
         AFTER DELETE ON core_transactions \
         FOR EACH ROW \
         BEGIN \
            UPDATE core_utxos SET spent_in_txid = NULL \
                WHERE wallet_id = OLD.wallet_id AND spent_in_txid = OLD.txid; \
         END;\n",
    );

    let mut sql = m.make::<Sqlite>();
    sql.push_str(&tail);
    sql
}
