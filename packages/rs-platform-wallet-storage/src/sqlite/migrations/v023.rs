//! Add `core_instant_locks.verified` only where it is missing.

use rusqlite::{OptionalExtension, Transaction};

use crate::sqlite::error::WalletStorageError;

/// Run V023's `sql` unless `core_instant_locks.verified` already exists. A
/// column with V023's definition is kept with the values it holds; one with
/// another definition stops the migration.
pub(super) fn add_verified_column(
    tx: &Transaction<'_>,
    sql: &str,
) -> Result<(), WalletStorageError> {
    let existing: Option<(String, bool, Option<String>, i64)> = tx
        .query_row(
            "SELECT type, \"notnull\", dflt_value, pk \
             FROM pragma_table_info('core_instant_locks') WHERE name = 'verified'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    match existing {
        None => Ok(tx.execute_batch(sql)?),
        Some((ty, true, Some(default), 0)) if ty == "INTEGER" && default == "0" => Ok(()),
        Some(_) => Err(WalletStorageError::ColumnDefinitionMismatch {
            table: "core_instant_locks",
            column: "verified",
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use rusqlite::{params, Connection, OptionalExtension};

    use crate::sqlite::error::WalletStorageError;
    use crate::sqlite::migrations::{self, insert_test_wallet};

    const WALLET: [u8; 32] = [0x23; 32];

    /// `core_instant_locks.verified` as `pragma table_info` reports it:
    /// type, not-null, default.
    fn verified_column(conn: &Connection) -> Option<(String, bool, Option<String>)> {
        conn.query_row(
            "SELECT type, \"notnull\", dflt_value \
             FROM pragma_table_info('core_instant_locks') WHERE name = 'verified'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .unwrap()
    }

    fn table_sql(conn: &Connection) -> String {
        conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'core_instant_locks'",
            [],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn v023_applied(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT count(*) FROM refinery_schema_history WHERE version = 23",
            [],
            |row| row.get(0),
        )
        .unwrap()
    }

    /// A database migrated through V022, holding one wallet.
    fn at_v022() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::runner()
            .set_target(refinery::Target::Version(22))
            .run(&mut conn)
            .unwrap();
        insert_test_wallet(&conn, &WALLET, 0);
        assert_eq!(verified_column(&conn), None);
        conn
    }

    fn insert_lock_row(conn: &Connection, byte: u8, verified: Option<i64>) {
        match verified {
            Some(verified) => conn.execute(
                "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob, verified) \
                 VALUES (?1, ?2, X'00', ?3)",
                params![&WALLET[..], &[byte; 32][..], verified],
            ),
            None => conn.execute(
                "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob) \
                 VALUES (?1, ?2, X'00')",
                params![&WALLET[..], &[byte; 32][..]],
            ),
        }
        .unwrap();
    }

    fn verified_by_row(conn: &Connection) -> Vec<(u8, i64)> {
        let mut stmt = conn
            .prepare("SELECT txid, verified FROM core_instant_locks ORDER BY txid")
            .unwrap();
        stmt.query_map([], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?[0], row.get::<_, i64>(1)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    }

    #[test]
    fn should_add_the_column_to_a_fresh_database() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::run(&mut conn).unwrap();

        assert_eq!(
            verified_column(&conn),
            Some(("INTEGER".to_owned(), true, Some("0".to_owned())))
        );
        assert!(table_sql(&conn)
            .contains("verified INTEGER NOT NULL DEFAULT 0 CHECK (verified IN (0, 1))"));
        assert_eq!(v023_applied(&conn), 1);
        insert_test_wallet(&conn, &WALLET, 0);
        assert!(
            conn.execute(
                "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob, verified) \
                 VALUES (?1, ?2, X'00', 2)",
                params![&WALLET[..], &[0x01u8; 32][..]],
            )
            .is_err(),
            "verified is 0 or 1"
        );
    }

    /// Rows already on disk may hold a lock nobody verified: they read 0.
    #[test]
    fn should_mark_existing_lock_rows_unverified() {
        let mut conn = at_v022();
        insert_lock_row(&conn, 0x01, None);
        insert_lock_row(&conn, 0x02, None);

        migrations::run(&mut conn).unwrap();

        assert_eq!(verified_by_row(&conn), vec![(0x01, 0), (0x02, 0)]);
    }

    /// A database some build already gave this exact column keeps it and
    /// every value in it, and ends with the schema a fresh database has.
    #[test]
    fn should_adopt_an_existing_column_with_its_values() {
        let mut conn = at_v022();
        conn.execute_batch(
            "ALTER TABLE core_instant_locks
                 ADD COLUMN verified INTEGER NOT NULL DEFAULT 0 CHECK (verified IN (0, 1));",
        )
        .unwrap();
        insert_lock_row(&conn, 0x01, Some(1));
        insert_lock_row(&conn, 0x02, Some(0));
        insert_lock_row(&conn, 0x03, Some(1));

        migrations::run(&mut conn).unwrap();

        assert_eq!(
            verified_by_row(&conn),
            vec![(0x01, 1), (0x02, 0), (0x03, 1)]
        );
        assert_eq!(v023_applied(&conn), 1);
        let mut fresh = Connection::open_in_memory().unwrap();
        migrations::run(&mut fresh).unwrap();
        assert_eq!(table_sql(&conn), table_sql(&fresh));
    }

    /// Running V023 again over its own column (its history row lost, as an
    /// interrupted upgrade could leave a copied file) changes nothing.
    #[test]
    fn should_rerun_over_its_own_column_without_change() {
        let mut conn = at_v022();
        insert_lock_row(&conn, 0x01, None);
        migrations::run(&mut conn).unwrap();
        insert_lock_row(&conn, 0x02, Some(1));
        let schema = table_sql(&conn);
        conn.execute("DELETE FROM refinery_schema_history WHERE version = 23", [])
            .unwrap();

        migrations::run(&mut conn).unwrap();

        assert_eq!(verified_by_row(&conn), vec![(0x01, 0), (0x02, 1)]);
        assert_eq!(table_sql(&conn), schema);
        assert_eq!(v023_applied(&conn), 1);
        migrations::run(&mut conn).unwrap();
        assert_eq!(v023_applied(&conn), 1, "an applied V023 does not run again");
    }

    /// A `verified` column with another definition could read values this
    /// schema does not mean (here every row would default to trusted): the
    /// migration stops, and leaves the database as it was.
    #[test]
    fn should_refuse_a_column_with_another_definition() {
        let mut conn = at_v022();
        conn.execute_batch("ALTER TABLE core_instant_locks ADD COLUMN verified INTEGER DEFAULT 1;")
            .unwrap();
        insert_lock_row(&conn, 0x01, None);

        let err = migrations::run(&mut conn).unwrap_err();

        let cause = err
            .source()
            .and_then(|source| source.downcast_ref::<WalletStorageError>());
        assert!(
            matches!(
                cause,
                Some(WalletStorageError::ColumnDefinitionMismatch {
                    table: "core_instant_locks",
                    column: "verified"
                })
            ),
            "{err:?}"
        );
        assert_eq!(v023_applied(&conn), 0);
        assert_eq!(
            verified_column(&conn),
            Some(("INTEGER".to_owned(), false, Some("1".to_owned())))
        );
        assert_eq!(verified_by_row(&conn), vec![(0x01, 1)]);
    }
}
