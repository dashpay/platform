# platform-wallet-sqlite

A SQLite-backed implementation of `PlatformWalletPersistence` for the
[`platform-wallet`](../rs-platform-wallet) crate, plus a small CLI for
maintenance tasks (backup / restore / prune / inspect / migrate /
delete-wallet).

## At a glance

- One `.db` file holds many wallets — every per-wallet row carries a
  `wallet_id BLOB` primary-key component.
- Schema migrations are append-only Rust files under `migrations/`,
  applied via [`refinery`](https://github.com/rust-db/refinery) on every
  `open`.
- Online backup uses `rusqlite::backup::Backup::run_to_completion` —
  safe under a concurrent writer.
- **No private-key material.** See [`SECRETS.md`](./SECRETS.md).
- `Send + Sync`; usable behind `Arc<dyn PlatformWalletPersistence>`.

## Library usage

```rust,no_run
use std::sync::Arc;
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet_sqlite::{SqlitePersister, SqlitePersisterConfig};

let config = SqlitePersisterConfig::new("/tmp/wallets.db");
let persister: Arc<dyn PlatformWalletPersistence> =
    Arc::new(SqlitePersister::open(config)?);
# Ok::<_, platform_wallet_sqlite::SqlitePersisterError>(())
```

`SqlitePersisterConfig::new(path)` produces sensible defaults:
`Immediate` flush, 5 s busy timeout, WAL journal, `NORMAL`
synchronous, and an auto-backup dir at `<db_dir>/backups/auto/`.

## CLI

```text
platform-wallet-sqlite --db <path> migrate
platform-wallet-sqlite --db <path> backup --out <dir-or-file>
platform-wallet-sqlite --db <path> restore --from <backup.db> --yes
platform-wallet-sqlite --db <path> prune --in <dir> [--keep-last N] [--max-age 30d] [--dry-run]
platform-wallet-sqlite --db <path> inspect [--wallet-id <hex>] [--format text|tsv|json]
platform-wallet-sqlite --db <path> delete-wallet --wallet-id <hex> --yes [--no-auto-backup]
```

Exit codes: `0` success, `1` runtime error, `2` usage error, `3`
validation failure (e.g. corrupt backup source).

## Schema

See [`migrations/V001__initial.rs`](./migrations/V001__initial.rs) for
the canonical schema. Foreign-key integrity is emulated with triggers
because barrel's column builder does not emit composite-key `FK`
clauses portably; the result is identical to native FKs from the
caller's perspective.
