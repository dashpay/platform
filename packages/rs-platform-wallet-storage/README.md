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

`SqlitePersister::load()` returns the base `ClientStartState`
(plain struct, two slots — no `#[non_exhaustive]`):

| Slot | Reader | Status |
|---|---|---|
| `platform_addresses` | `schema::platform_addrs::load_all` (a `wallet_meta::list_ids` → `load_state` loop) | populated |
| `wallets`            | — | empty pending upstream `Wallet::from_persisted` |

The `identities` / `contacts` / `asset_locks` per-area readers exist
as hardened dormant helpers (`schema::<area>::load_state`) but are not
wired into `load()` — `ClientStartState` carries no slot for them.

Loading is **fail-hard**: any row that fails to decode, or a stored
`wallet_id` that is not exactly 32 bytes, aborts the whole call with a
typed [`WalletStorageError`](src/sqlite/error.rs)
(`BincodeDecode` / `BlobDecode` / `InvalidWalletIdLength`). There is no
corruption tolerance, no per-row skip, and no partial `Ok` — a corrupt
database surfaces as an error rather than silently losing rows.

The summary `tracing::info!` carries `wallets_seen`,
`addresses_loaded`, `wallets_rehydrated`, and
`wallets_pending_rehydration` (the count of wallets that *would* be
rehydrated once upstream provides `Wallet::from_persisted`).

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

## Operational notes

**Restore advisory-lock warning.** `restore` takes an exclusive `flock(2)`
on the destination DB and holds it across the entire restore body, so a
concurrent writer can't race the atomic swap. On filesystems where
advisory locking is unsupported (some NFS / FUSE / network mounts), the
crate emits a `tracing::warn!` on the
`platform_wallet_storage` target —

> `advisory lock unsupported on this filesystem; concurrent-writer race possible`

— and proceeds anyway (there's no alternative on such filesystems).
If you see this warning, ensure no other process opens the destination
DB during the restore window, or move the DB to a filesystem with flock
support before restoring.

**Manual-mode drop diagnostic.** `SqlitePersister` configured with
[`FlushMode::Manual`] emits a `tracing::error!` on drop if the buffer
still holds uncommitted writes (with `dirty_wallets` and `total_fields`
fields). The crate does NOT auto-flush from `Drop` — call
[`SqlitePersister::commit_writes`] (or per-wallet `flush`) before drop
to make Manual-mode writes durable.

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
the canonical schema. It is hand-written `CREATE TABLE … PRIMARY KEY …
FOREIGN KEY …` SQL with native `ON DELETE CASCADE` constraints; INSERT,
DELETE-cascade, and UPDATE re-parenting are all enforced by SQLite
itself. Foreign-key enforcement is enabled and read-back-asserted on
every connection open via the `open_conn` choke-point — if the linked
SQLite cannot honor `PRAGMA foreign_keys`, open fails hard. The single
remaining trigger clears `core_utxos.spent_in_txid` to NULL on
transaction delete (a native composite `SET NULL` would null the
NOT-NULL `wallet_id` column too).
