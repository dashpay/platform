//! Add `core_dismissed_txids` only where it is missing.

use rusqlite::Transaction;

use crate::sqlite::error::WalletStorageError;

const TABLE: &str = "core_dismissed_txids";

/// Run V024's `sql`, which creates the table only if it is missing, then
/// check the table it leaves. A table already there with this definition is
/// kept with its rows. One from before the kind, with exactly the earlier
/// definition, is rebuilt in this one with its rows, their `id`s kept, all
/// marked abandoned: which kind each was is not recorded, and an abandon read
/// as swept could be pushed out by later sweeps, while a sweep read as
/// abandoned only takes a slot of the other bound. A row whose wallet is gone
/// is dead and left out, and past the newest 4,096 of a wallet, key-wallet's
/// cap on its abandons, the oldest go. A table of any other
/// shape is refused, since the store's writes and reads rely on its `id`
/// order, its `(wallet_id, txid, abandoned)` key and its cascade from
/// `wallets`.
pub(super) fn add_dismissed_txids(
    tx: &Transaction<'_>,
    sql: &str,
) -> Result<(), WalletStorageError> {
    tx.execute_batch(sql)?;
    if shape(tx)?.is(&BEFORE_THE_KIND) {
        tx.execute_batch(&format!(
            "ALTER TABLE {TABLE} RENAME TO {TABLE}_before_kind; \
             {sql} \
             INSERT INTO {TABLE} (id, wallet_id, txid, abandoned) \
             SELECT id, wallet_id, txid, 1 FROM (SELECT id, wallet_id, txid, \
                 row_number() OVER (PARTITION BY wallet_id ORDER BY id DESC) AS newer \
                 FROM {TABLE}_before_kind) \
             WHERE newer <= 4096 \
             AND wallet_id IN (SELECT wallet_id FROM wallets); \
             DROP TABLE {TABLE}_before_kind;"
        ))?;
    }
    if shape(tx)?.is(&WITH_THE_KIND) {
        Ok(())
    } else {
        Err(WalletStorageError::TableDefinitionMismatch { table: TABLE })
    }
}

/// A shape [`add_dismissed_txids`] accepts: columns as (name, type, NOT NULL
/// on a non-key column, primary-key position, default), and the one full
/// unique key.
struct Accepted {
    columns: &'static [(&'static str, &'static str, bool, i64, Option<&'static str>)],
    unique_key: &'static str,
}

const BEFORE_THE_KIND: Accepted = Accepted {
    columns: &[
        ("id", "INTEGER", false, 1, None),
        ("wallet_id", "BLOB", true, 0, None),
        ("txid", "BLOB", true, 0, None),
    ],
    unique_key: "wallet_id,txid",
};

const WITH_THE_KIND: Accepted = Accepted {
    columns: &[
        ("id", "INTEGER", false, 1, None),
        ("wallet_id", "BLOB", true, 0, None),
        ("txid", "BLOB", true, 0, None),
        ("abandoned", "INTEGER", true, 0, Some("0")),
    ],
    unique_key: "wallet_id,txid,abandoned",
};

/// The table's shape as SQLite reports it, independent of DDL text.
struct Shape {
    columns: Vec<(String, String, bool, i64, Option<String>)>,
    unique_keys: Vec<String>,
    foreign_keys: Vec<(String, String, String, String)>,
}

impl Shape {
    fn is(&self, accepted: &Accepted) -> bool {
        self.columns
            .iter()
            .map(|(n, t, nn, pk, d)| (n.as_str(), t.as_str(), *nn, *pk, d.as_deref()))
            .eq(accepted.columns.iter().copied())
            && self.unique_keys == [accepted.unique_key]
            && self
                .foreign_keys
                .iter()
                .map(|(t, f, to, d)| (t.as_str(), f.as_str(), to.as_str(), d.as_str()))
                .eq([("wallets", "wallet_id", "wallet_id", "CASCADE")])
    }
}

fn shape(tx: &Transaction<'_>) -> Result<Shape, WalletStorageError> {
    let columns = tx
        .prepare(
            "SELECT lower(name), upper(type), \"notnull\" AND pk = 0, pk, dflt_value \
             FROM pragma_table_info(?1) ORDER BY cid",
        )?
        .query_map([TABLE], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let unique_keys = tx
        .prepare(
            "SELECT (SELECT group_concat(lower(name), ',') FROM \
                     (SELECT name FROM pragma_index_info(l.name) ORDER BY seqno)) \
             FROM pragma_index_list(?1) AS l \
             WHERE l.\"unique\" = 1 AND l.partial = 0 ORDER BY l.name",
        )?
        .query_map([TABLE], |r| r.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    let foreign_keys = tx
        .prepare(
            "SELECT lower(\"table\"), lower(\"from\"), lower(coalesce(\"to\", 'wallet_id')), \
                    upper(on_delete) \
             FROM pragma_foreign_key_list(?1)",
        )?
        .query_map([TABLE], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Shape {
        columns,
        unique_keys,
        foreign_keys,
    })
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::sqlite::error::WalletStorageError;
    use crate::sqlite::migrations;

    fn at(version: i32) -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::runner()
            .set_target(refinery::Target::Version(version))
            .run(&mut conn)
            .unwrap();
        conn
    }

    fn at_v023() -> Connection {
        at(23)
    }

    fn schema(conn: &Connection) -> Vec<(String, Option<String>)> {
        conn.prepare("SELECT name, sql FROM sqlite_master ORDER BY type, name")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn table_sql(conn: &Connection) -> Option<String> {
        conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'core_dismissed_txids'",
            [],
            |row| row.get(0),
        )
        .ok()
    }

    /// Opened two versions back, so the refusal must also undo the
    /// migration the same run applied before this one.
    fn refuses(existing: &str) {
        let mut conn = at(22);
        conn.execute_batch(existing).unwrap();
        let before = schema(&conn);

        let err = migrations::run(&mut conn).expect_err("another shape must be refused");

        assert!(
            matches!(
                std::error::Error::source(&err)
                    .and_then(|e| e.downcast_ref::<WalletStorageError>()),
                Some(WalletStorageError::TableDefinitionMismatch {
                    table: "core_dismissed_txids"
                })
            ),
            "unexpected error: {err:?}"
        );
        assert_eq!(schema(&conn), before, "the refused run left changes");
        let latest: i64 = conn
            .query_row(
                "SELECT max(version) FROM refinery_schema_history",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(latest, 22, "the refused run was recorded");
    }

    /// A fresh database gets the table with the definition the check expects.
    #[test]
    fn should_add_the_table_to_a_fresh_database() {
        let mut conn = at_v023();
        migrations::run(&mut conn).unwrap();
        assert!(table_sql(&conn).is_some());
    }

    /// The same table declared in other letter case, with its primary key
    /// spelled NOT NULL or its reference naming no parent column, is the
    /// same table.
    #[test]
    fn should_adopt_the_table_declared_another_way() {
        for ddl in [
            "create table core_dismissed_txids (
                 id integer primary key,
                 wallet_id blob not null,
                 txid blob not null,
                 unique (wallet_id, txid),
                 foreign key (wallet_id) references wallets(wallet_id) on delete cascade
             );",
            "CREATE TABLE core_dismissed_txids (
                 ID INTEGER PRIMARY KEY NOT NULL,
                 WALLET_ID BLOB NOT NULL REFERENCES Wallets ON DELETE CASCADE,
                 TXID BLOB NOT NULL,
                 UNIQUE (WALLET_ID, TXID)
             );",
        ] {
            let mut conn = at_v023();
            conn.execute_batch(ddl).unwrap();
            migrations::run(&mut conn).unwrap();
        }
    }

    /// A key unique only over some rows lets a duplicate in.
    #[test]
    fn should_refuse_a_partial_unique_key() {
        refuses(
            "CREATE TABLE core_dismissed_txids (
                 id INTEGER PRIMARY KEY,
                 wallet_id BLOB NOT NULL REFERENCES wallets(wallet_id) ON DELETE CASCADE,
                 txid BLOB NOT NULL
             );
             CREATE UNIQUE INDEX dismissed_some ON core_dismissed_txids (wallet_id, txid)
                 WHERE id > 0;",
        );
    }

    /// Without its `(wallet_id, txid)` key every dismissal write would fail.
    #[test]
    fn should_refuse_a_table_without_its_unique_key() {
        refuses(
            "CREATE TABLE core_dismissed_txids (
                 id INTEGER PRIMARY KEY,
                 wallet_id BLOB NOT NULL REFERENCES wallets(wallet_id) ON DELETE CASCADE,
                 txid BLOB NOT NULL
             );",
        );
    }

    /// Without `id` the load cannot order the rows.
    #[test]
    fn should_refuse_a_table_without_its_id() {
        refuses(
            "CREATE TABLE core_dismissed_txids (
                 wallet_id BLOB NOT NULL REFERENCES wallets(wallet_id) ON DELETE CASCADE,
                 txid BLOB NOT NULL,
                 UNIQUE (wallet_id, txid)
             );",
        );
    }

    /// A table from before the kind is rebuilt as a fresh file declares it.
    #[test]
    fn should_rebuild_a_table_from_before_the_kind() {
        let mut conn = at_v023();
        conn.execute_batch(
            "CREATE TABLE core_dismissed_txids (
                 id INTEGER PRIMARY KEY,
                 wallet_id BLOB NOT NULL,
                 txid BLOB NOT NULL,
                 UNIQUE (wallet_id, txid),
                 FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
             );",
        )
        .unwrap();
        migrations::run(&mut conn).unwrap();
        let kind: (String, bool, Option<String>) = conn
            .query_row(
                "SELECT upper(type), \"notnull\", dflt_value \
                 FROM pragma_table_info('core_dismissed_txids') WHERE name = 'abandoned'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(kind, ("INTEGER".to_owned(), true, Some("0".to_owned())));
        let key: String = conn
            .query_row(
                "SELECT group_concat(name, ',') FROM pragma_index_info( \
                     (SELECT name FROM pragma_index_list('core_dismissed_txids') \
                      WHERE \"unique\" = 1))",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(key, "wallet_id,txid,abandoned");
    }

    /// A kind that may be NULL would read as neither kind.
    #[test]
    fn should_refuse_a_kind_that_may_be_null() {
        refuses(
            "CREATE TABLE core_dismissed_txids (
                 id INTEGER PRIMARY KEY,
                 wallet_id BLOB NOT NULL REFERENCES wallets(wallet_id) ON DELETE CASCADE,
                 txid BLOB NOT NULL,
                 abandoned INTEGER,
                 UNIQUE (wallet_id, txid, abandoned)
             );",
        );
    }

    /// The rebuild leaves out a row whose wallet is gone (an open refuses such
    /// a file first, in its foreign-key check; a bare migration run does not).
    #[test]
    fn should_leave_a_row_of_a_deleted_wallet_out_of_the_rebuild() {
        let mut conn = at_v023();
        conn.execute_batch(
            "INSERT INTO wallets (wallet_id, network, birth_height) \
                 VALUES (x'01', 'testnet', 0);
             CREATE TABLE core_dismissed_txids (
                 id INTEGER PRIMARY KEY,
                 wallet_id BLOB NOT NULL,
                 txid BLOB NOT NULL,
                 UNIQUE (wallet_id, txid),
                 FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
             );
             PRAGMA foreign_keys = OFF;
             INSERT INTO core_dismissed_txids (id, wallet_id, txid) VALUES (3, x'01', x'aa'), (5, x'02', x'bb');
             PRAGMA foreign_keys = ON;",
        )
        .unwrap();
        migrations::run(&mut conn).unwrap();
        let rows: Vec<(i64, Vec<u8>, bool)> = conn
            .prepare("SELECT id, wallet_id, abandoned FROM core_dismissed_txids ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(rows, vec![(3, vec![0x01], true)]);
    }

    /// The rebuild keeps a wallet's newest 4,096 rows, as key-wallet would
    /// hold them, and every row of a wallet with fewer.
    #[test]
    fn should_keep_a_wallets_newest_4096_in_the_rebuild() {
        let mut conn = at_v023();
        conn.execute_batch(
            "INSERT INTO wallets (wallet_id, network, birth_height) \
                 VALUES (x'01', 'testnet', 0), (x'03', 'testnet', 0);
             CREATE TABLE core_dismissed_txids (
                 id INTEGER PRIMARY KEY,
                 wallet_id BLOB NOT NULL,
                 txid BLOB NOT NULL,
                 UNIQUE (wallet_id, txid),
                 FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
             );
             WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 4098)
             INSERT INTO core_dismissed_txids (id, wallet_id, txid)
                 SELECT i * 2, x'01', randomblob(32) FROM n;
             INSERT INTO core_dismissed_txids (id, wallet_id, txid)
                 VALUES (1, x'03', x'aa'), (3, x'03', x'bb');",
        )
        .unwrap();
        migrations::run(&mut conn).unwrap();
        let kept = |wallet: u8| -> (i64, i64, i64) {
            conn.query_row(
                "SELECT count(*), min(id), max(id) FROM core_dismissed_txids \
                 WHERE wallet_id = ?1 AND abandoned = 1",
                [vec![wallet]],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap()
        };
        assert_eq!(kept(0x01), (4_096, 6, 8_196));
        assert_eq!(kept(0x03), (2, 1, 3));
    }

    /// A kind without its default could not take a row written without one.
    #[test]
    fn should_refuse_a_kind_without_its_default() {
        refuses(
            "CREATE TABLE core_dismissed_txids (
                 id INTEGER PRIMARY KEY,
                 wallet_id BLOB NOT NULL REFERENCES wallets(wallet_id) ON DELETE CASCADE,
                 txid BLOB NOT NULL,
                 abandoned INTEGER NOT NULL CHECK (abandoned IN (0, 1)),
                 UNIQUE (wallet_id, txid, abandoned)
             );",
        );
    }

    /// With the kind but the key from before it, a transaction could not be
    /// held as both kinds.
    #[test]
    fn should_refuse_a_kind_without_its_key() {
        refuses(
            "CREATE TABLE core_dismissed_txids (
                 id INTEGER PRIMARY KEY,
                 wallet_id BLOB NOT NULL REFERENCES wallets(wallet_id) ON DELETE CASCADE,
                 txid BLOB NOT NULL,
                 abandoned INTEGER NOT NULL DEFAULT 0 CHECK (abandoned IN (0, 1)),
                 UNIQUE (wallet_id, txid)
             );",
        );
    }

    /// Without the cascade a deleted wallet would leave its rows behind.
    #[test]
    fn should_refuse_a_table_without_its_cascade() {
        refuses(
            "CREATE TABLE core_dismissed_txids (
                 id INTEGER PRIMARY KEY,
                 wallet_id BLOB NOT NULL,
                 txid BLOB NOT NULL,
                 UNIQUE (wallet_id, txid)
             );",
        );
    }
}
