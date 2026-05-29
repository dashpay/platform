# platform-wallet-storage

Storage backends for the
[`platform-wallet`](../rs-platform-wallet) crate. This crate ships a
SQLite-backed implementation of `PlatformWalletPersistence` under
[`sqlite`](src/sqlite/), a maintenance CLI, and the
[`secrets`](src/secrets/) submodule — a `keyring_core` SPI
implementation pairing the in-house `EncryptedFileStore`
(Argon2id + XChaCha20-Poly1305 on-disk vault) with the OS keyring
backends. All three are on by default; see [`SECRETS.md`](./SECRETS.md)
for the secret-storage threat model and design.

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
call returns a `PersistenceError::Backend { kind: Transient, source }`
whose source carries the marker `flush failed transiently`.
**Retry the call** — do not discard state. Fatal failures (integrity
check, encode error, mutex poison, …) return `kind: Fatal` (or
`kind: Constraint` for SQL constraint violations) and drop the buffer.

The full classification lives on
[`WalletStorageError::is_transient`](src/sqlite/error.rs) and the
companion [`WalletStorageError::persistence_kind`](src/sqlite/error.rs)
that selects the trait-side kind. The `source` field is a
`Box<dyn Error + Send + Sync>` over the original `WalletStorageError`
— operators can walk `Error::source()` for the full typed chain;
the outer `Display` carries the variant marker + hex wallet id so
production-log greps still work.

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
platform-wallet-storage prune --in <dir> [--keep-last N] [--max-age 30d]
platform-wallet-storage --db <path> inspect [--wallet-id <hex>] [--format text|tsv|json]
```

Destructive subcommands (`restore`) REQUIRE `--yes` — invoking them
without it exits 2 with a usage error. `--no-auto-backup` opts out of
the pre-restore (or pre-migration) auto-backup; it is the only
supported way to disable auto-backup.

Wallet removal is a library-only API
([`SqlitePersister::delete_wallet`] / `delete_wallet_skip_backup`);
no CLI subcommand exposes it.

Logging: `-v` / `-vv` / `-vvv` enable `info` / `debug` / `trace`
respectively on stderr; `-q` suppresses non-error output.

Exit codes: `0` success, `1` runtime error, `2` usage error, `3`
validation failure (e.g. corrupt backup source).

## Operational notes

**Restore exclusion.** `restore` opens a short-lived writer connection
on the destination DB and holds a SQLite-native `BEGIN EXCLUSIVE`
transaction across the entire restore body. This interlocks with every
other SQLite peer — sibling `SqlitePersister` handles, bare
`rusqlite::Connection` instances, the CLI — so concurrent writes back
off via SQLite's `busy_timeout` instead of racing the atomic swap. If a
peer holds the destination busy for longer than the timeout, `restore`
returns `WalletStorageError::RestoreDestinationLocked`. The lock conn is
released BEFORE the rename so SQLite's file handle on the old inode goes
away before the new DB takes its place.

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
| `secrets` | yes | `platform_wallet_storage::secrets` submodule — zeroizing secret wrappers (`SecretBytes`, `SecretString`), the `EncryptedFileStore` Argon2id + XChaCha20-Poly1305 vault backend, and the `default_credential_store()` OS-keyring constructor. Implements the upstream `keyring_core::api::{CredentialApi, CredentialStoreApi}` SPI. |
| `__test-helpers` | no | Crate-private `lock_conn_for_test` / `config_for_test` accessors. The double-underscore prefix follows Cargo's "do not enable from downstream" convention; the methods are also `#[doc(hidden)]`. |

`cargo build -p platform-wallet-storage --no-default-features` builds a
minimal core with neither the SQLite backend, the CLI, nor the secrets
submodule. `--no-default-features --features sqlite,cli` is the
"persister-only" build mode (no crypto dependencies).

## Schema

See [`migrations/V001__initial.rs`](./migrations/V001__initial.rs) for
the canonical schema. It is hand-written `CREATE TABLE … PRIMARY KEY …
FOREIGN KEY …` SQL with native `ON DELETE CASCADE` constraints; INSERT,
DELETE-cascade, and UPDATE re-parenting are all enforced by SQLite
itself. Wallet-scoped tables FK directly to `wallet_metadata`;
identity-owned tables (`identity_keys`, `token_balances`,
`dashpay_profiles`, `dashpay_payments_overlay`) are keyed by
`identity_id` only and cascade through `identities` (whose `wallet_id`
is nullable to support identity-only flows). Foreign-key enforcement is
enabled and read-back-asserted on every connection open via the
`open_conn` choke-point — if the linked SQLite cannot honor
`PRAGMA foreign_keys`, open fails hard. The single remaining trigger
clears `core_utxos.spent_in_txid` to NULL on transaction delete (a
native composite `SET NULL` would null the NOT-NULL `wallet_id` column
too).
