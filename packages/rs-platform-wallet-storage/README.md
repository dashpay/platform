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
- Writers use `prepare_cached` so each INSERT/UPDATE is parsed once
  per `Connection` lifetime; subsequent flushes hit the cache.

## Flush semantics

`flush()` and `Immediate`-mode `store()` succeed-or-restore: on a
transient SQLite failure (`SQLITE_BUSY` / `SQLITE_LOCKED`) the
buffered changeset is merged back into the per-wallet buffer (LWW
with anything `store()`-d during the failed transaction) and the
call returns a `PersistenceError::Backend(_)` whose payload contains
the marker `flush failed transiently`. **Retry the call** — do not
discard state. Fatal failures (integrity check, encode error, mutex
poison, …) drop the buffer and surface verbatim.

The full classification lives on
[`WalletStorageError::is_transient`](src/sqlite/error.rs); the
boundary mapping into `PersistenceError::Backend(String)` flattens
the `Display` chain so operators can grep for variant names + hex
wallet ids in production logs.

## load() reconstruction

`SqlitePersister::load()` populates `ClientStartState` with every
sub-area that has a wired-up reader today:

| Slot | Reader | Status |
|---|---|---|
| `platform_addresses` | `schema::platform_addrs::load_state` | covered |
| `identities`         | `schema::identities::load_state`     | covered |
| `contacts`           | `schema::contacts::load_state`       | covered |
| `asset_locks`        | `schema::asset_locks::load_state`    | covered |
| `wallets`            | — | empty pending upstream `Wallet::from_persisted` |

`ClientStartState` is `#[non_exhaustive]` — initialise via
`Default::default()` and overwrite individual slots; do not
exhaustively destructure. A future slot addition is non-breaking for
callers that respect the marker.

Each reader skips per-row decode failures (corruption tolerance):
the call still returns `Ok(state)` with the partial result, every
skipped row emits a structured `tracing::warn!` with `wallet_id` +
`table` + `error`, and the load summary log carries a
`skipped_rows` counter alongside `wallets_seen`,
`addresses_loaded`, `identities_loaded`, `contacts_loaded`,
`asset_locks_loaded`, `wallets_rehydrated`, and
`wallets_pending_rehydration`.

## Library usage

```rust,no_run
use std::sync::Arc;
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

let config = SqlitePersisterConfig::new("/tmp/wallets.db");
let persister: Arc<dyn PlatformWalletPersistence> =
    Arc::new(SqlitePersister::open(config)?);
# Ok::<_, platform_wallet_storage::WalletStorageError>(())
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
platform-wallet-storage --db <path> prune --in <dir> [--keep-last N] [--max-age 30d]
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
| `__test-helpers` | no | Crate-private `lock_conn_for_test` / `config_for_test` accessors. The double-underscore prefix follows Cargo's "do not enable from downstream" convention; the methods are also `#[doc(hidden)]`. |

`cargo build -p platform-wallet-storage --no-default-features` builds
the crate with neither the SQLite backend nor the CLI compiled in.
The resulting library has no public surface today; the build mode
exists to support a future split where one cargo target wants only
the secrets feature.

## Schema

See [`migrations/V001__initial.rs`](./migrations/V001__initial.rs) for
the canonical schema and
[`migrations/V002__defensive_update_triggers.rs`](./migrations/V002__defensive_update_triggers.rs)
for the `BEFORE UPDATE` FK-column guards. Foreign-key integrity is
emulated with triggers because barrel's column builder does not emit
composite-key `FK` clauses portably; INSERT, DELETE-cascade, and
UPDATE of `wallet_id` / `identity_id` are all covered. The result
matches native FKs for the persister's own write path, which never
mutates those columns directly.
