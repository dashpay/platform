# platform-wallet-storage

Storage backends for the
[`platform-wallet`](../rs-platform-wallet) crate. Today this crate
ships a SQLite-backed implementation of `PlatformWalletPersistence`
under [`sqlite`](src/sqlite/) plus a maintenance CLI; the crate is
structured so a future `SecretStore` (currently sketched in
[`SECRETS.md`](./SECRETS.md)) can land as a sibling submodule under
[`secrets`](src/) without a crate split.

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
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

let config = SqlitePersisterConfig::new("/tmp/wallets.db");
let persister: Arc<dyn PlatformWalletPersistence> =
    Arc::new(SqlitePersister::open(config)?);
# Ok::<_, platform_wallet_storage::SqlitePersisterError>(())
```

The same types are also reachable via their canonical submodule path —
`platform_wallet_storage::sqlite::SqlitePersister` — for callers that
want to be explicit about the backend.

`SqlitePersisterConfig::new(path)` produces sensible defaults:
`Immediate` flush, 5 s busy timeout, WAL journal, `NORMAL`
synchronous, and an auto-backup dir at `<db_dir>/backups/auto/`.

## CLI

```text
platform-wallet-storage --db <path> migrate [--no-auto-backup]
platform-wallet-storage --db <path> backup --out <dir-or-file>
platform-wallet-storage --db <path> restore --from <backup.db> --yes
platform-wallet-storage --db <path> prune --in <dir> [--keep-last N] [--max-age 30d] [--dry-run]
platform-wallet-storage --db <path> inspect [--wallet-id <hex>] [--format text|tsv|json]
platform-wallet-storage --db <path> delete-wallet --wallet-id <hex> --yes [--no-auto-backup]
```

Destructive subcommands (`restore`, `delete-wallet`) REQUIRE `--yes`
— invoking them without it exits 2 with a usage error. `--no-auto-backup`
opts out of the pre-migration / pre-delete auto-backup respectively;
the library API has no equivalent opt-out (it routes to
[`SqlitePersister::delete_wallet_skip_backup`] internally).

Logging: `-v` / `-vv` / `-vvv` enable `info` / `debug` / `trace`
respectively on stderr; `-q` suppresses non-error output.

Exit codes: `0` success, `1` runtime error, `2` usage error, `3`
validation failure (e.g. corrupt backup source).

## Cargo features

| Feature | Default | What it brings |
|---|---|---|
| `sqlite` | yes | SQLite persister (`platform_wallet_storage::sqlite`) and all of its native deps (`rusqlite`, `refinery`, `dpp`, `dash-sdk`, `key-wallet`, etc.) |
| `cli` | yes | Maintenance binary `platform-wallet-storage`. Implies `sqlite`. |
| `secrets` | no | Reserved for the future `SecretStore` submodule. No code lands today. |
| `test-helpers` | no | Crate-private `lock_conn_for_test` / `config_for_test` accessors. Downstream MUST NOT enable. |

`cargo build -p platform-wallet-storage --no-default-features` builds
the bare crate (no backend, no CLI) and is the entry point for the
future `secrets`-only build.

## Schema

See [`migrations/V001__initial.rs`](./migrations/V001__initial.rs) for
the canonical schema. Foreign-key integrity is emulated with triggers
because barrel's column builder does not emit composite-key `FK`
clauses portably; the result is identical to native FKs from the
caller's perspective.
