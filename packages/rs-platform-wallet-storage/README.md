# platform-wallet-storage

## Why this crate exists

A wallet on Dash Platform carries a lot of **public** state — UTXOs,
transactions, account registrations, address pools, identities and their
public keys, contacts, asset locks, token balances, DashPay overlays, and
platform-address sync snapshots. A client needs all of it on disk so it
can restart and pick up where it left off instead of re-scanning the chain
from genesis. Until now every integrator built that storage themselves.

`platform-wallet-storage` is the ready-to-use, embeddable answer: a SQLite
persistence backend for [`platform-wallet`](../rs-platform-wallet), plus a
small set of operational tools around it. One `.db` file holds many
wallets, durable across restarts, with online backup, restore, and
migration handled for you — and a contract you can lean on: **no
private-key material is ever written to that file.**

## What integrators get

- **Durable multi-wallet storage** in a single SQLite file. Every
  per-wallet row is keyed by `wallet_id`, so one file is the home for as
  many wallets as the host app manages. Writers use `prepare_cached` and
  the database runs WAL journaling by default.
- **The private-key boundary, in writing.** Mnemonics, seeds, and raw
  private keys never touch this file. Public-only material goes to SQLite;
  signing material stays in the OS keyring or the encrypted vault. See
  [SECRETS.md](./SECRETS.md).
- **Backup, restore, and migration handled for you.** Backups use
  SQLite's online backup API (safe under a concurrent writer); restores
  run under `BEGIN EXCLUSIVE` so peers back off instead of racing the
  swap; schema migrations apply automatically on every open.
- **A flush contract you can build retries on.** Transient SQLite
  failures return a *retryable* error with the buffered changeset intact;
  fatal and constraint failures are reported distinctly and drop the
  buffer. A corrupt database surfaces as a typed error on load rather than
  silently losing rows.
- **Crypto on by default.** The `secrets` backends ship in the default
  feature set, so `Cargo.lock` unconditionally pins the reviewed crypto
  stack.

## Features

### SQLite persister

The flagship: an implementation of `platform-wallet`'s
`PlatformWalletPersistence` over a single SQLite file. One database, many
wallets, every per-wallet row carrying a `wallet_id BLOB` primary-key
component. The persister is `Send + Sync` and usable behind
`Arc<dyn PlatformWalletPersistence>`. Wallet removal
([`SqlitePersister::delete_wallet`]) and explicit Manual-mode commit
([`SqlitePersister::commit_writes`]) are inherent methods on the persister,
not part of the trait.

### KV / ObjectId metadata

The `kv` feature adds a per-object key/value store ([`KvStore`](src/kv.rs))
for stashing app-managed metadata — aliases, flags, notes, sync hints,
ordering — alongside wallet objects. It is independent of
`PlatformWalletPersistence`: reads and writes go straight to the store
without flowing through the wallet changeset buffer. A no-foreign-key soft
cascade means metadata can be attached *ahead* of sync and still gets
cleaned up when its wallet is deleted.

### Secrets

The `secrets` module ([SECRETS.md](./SECRETS.md)) is where signing material
that the persister refuses to touch actually lives. One `SecretStore` front
door fronts two backends: an in-house encrypted-file vault (Argon2id +
XChaCha20-Poly1305) and the OS keyring. Wrappers zeroize on drop and the
error surface is typed and secret-free. It is fully implemented and **on by
default.**

### Maintenance CLI

The `cli` feature ships the `platform-wallet-storage` binary with four
subcommands — `migrate`, `backup`, `restore`, `prune` — for operating the
database without writing custom code.

---

## Technical details

### Library usage

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
`platform_wallet_storage::sqlite::SqlitePersister` — for callers that want
to be explicit about the backend.

`SqlitePersisterConfig::new(path)` produces sensible defaults: `Immediate`
flush, 5 s busy timeout, WAL journal, `NORMAL` synchronous, and an
auto-backup dir at `<db_dir>/backups/auto/`.

The trait surface is `store` / `flush` / `load` / `get_core_tx_record`.
Schema migrations are append-only Rust files under `migrations/`, applied
via [`refinery`](https://github.com/rust-db/refinery) on every `open`.

#### Flush semantics (store / flush)

`flush()` and `Immediate`-mode `store()` succeed-or-restore: on a transient
SQLite failure (`SQLITE_BUSY` / `SQLITE_LOCKED`) the buffered changeset is
merged back into the per-wallet buffer (LWW with anything `store()`-d during
the failed transaction) and the call returns a
`PersistenceError::Backend { kind: Transient, source }` whose source carries
the marker `flush failed transiently`. **Retry the call** — do not discard
state. Fatal failures (integrity check, encode error, mutex poison, …)
return `kind: Fatal` (or `kind: Constraint` for SQL constraint violations)
and drop the buffer.

The full classification lives on
[`WalletStorageError::is_transient`](src/sqlite/error.rs) and the companion
[`WalletStorageError::persistence_kind`](src/sqlite/error.rs) that selects
the trait-side kind. The `source` field is a
`Box<dyn Error + Send + Sync>` over the original `WalletStorageError` —
operators can walk `Error::source()` for the full typed chain; the outer
`Display` carries the variant marker + hex wallet id so production-log greps
still work.

A `SqlitePersister` configured with [`FlushMode::Manual`] does NOT
auto-flush from `Drop`; it emits a `tracing::error!` on drop if the buffer
still holds uncommitted writes (with `dirty_wallets` and `total_fields`
fields). Call [`SqlitePersister::commit_writes`] (or per-wallet `flush`)
before drop to make Manual-mode writes durable.
[`SqlitePersister::commit_writes`] returns a `CommitReport` whose
`succeeded` / `failed` / `still_pending` vectors classify each dirty wallet
so one failed wallet does not hide its siblings.

#### load() reconstruction

`SqlitePersister::load()` returns the base `ClientStartState` (plain struct,
two slots — no `#[non_exhaustive]`):

| Slot | Reader | Status |
|---|---|---|
| `platform_addresses` | `schema::platform_addrs::load_all` (a fixed set of grouped scans over `platform_address_sync`, `platform_addresses`, and `account_registrations`, driven by the `wallet_meta::list_ids` wallet universe) | populated |
| `wallets`            | — | empty pending upstream `Wallet::from_persisted` |

The `identities` / `contacts` / `asset_locks` per-area readers exist as
hardened dormant helpers (`schema::<area>::load_state`) but are not wired
into `load()` — `ClientStartState` carries no slot for them.

Loading is **fail-hard**: any row that fails to decode, or a stored
`wallet_id` that is not exactly 32 bytes, aborts the whole call with a typed
[`WalletStorageError`](src/sqlite/error.rs)
(`BincodeDecode` / `BlobDecode` / `InvalidWalletIdLength`). There is no
corruption tolerance, no per-row skip, and no partial `Ok` — a corrupt
database surfaces as an error rather than silently losing rows.

The summary `tracing::info!` carries `wallets_seen`, `addresses_loaded`,
`wallets_rehydrated`, and `wallets_pending_rehydration` (the count of
wallets that *would* be rehydrated once upstream provides
`Wallet::from_persisted`).

### KV metadata API

Each [`ObjectId`](src/kv.rs) variant addresses a dedicated `meta_*` table
across six scopes:

| `ObjectId` | Table | Scope |
|---|---|---|
| `Global` | `meta_global` | App-wide; no parent, survives wallet deletion |
| `Wallet(wid)` | `meta_wallet` | Per wallet |
| `Identity(id)` | `meta_identity` | Per identity |
| `Token { identity_id, token_id }` | `meta_token` | Per token balance |
| `Contact { wallet_id, owner_id, contact_id }` | `meta_contact` | Per contact (any lifecycle state) |
| `PlatformAddress { wallet_id, address }` | `meta_platform_address` | Per platform address |

**No-FK soft cascade.** Except for `Global`, a `put` does NOT require the
parent object to exist yet — metadata may be attached ahead of sync. When a
wallet is deleted, `AFTER DELETE` triggers broom every `meta_*` row keyed to
that wallet (by `wallet_id`, or by `identity_id` via the identity cascade) —
including rows whose typed parent was never written. `Global` is the only
scope that survives a wallet delete. Values are opaque `Vec<u8>` (the app
picks its own serialization); keys are 1..=128 chars and values are capped
at 16 MiB (`MAX_VALUE_LEN`), enforced by `put` before the write. For the
orphan-metadata limitation and future garbage-collection semantics, see
[SCHEMA.md](./SCHEMA.md#orphan-metadata-and-future-garbage-collection).

The four `KvStore` methods:

```rust,ignore
fn get(&self, scope: &ObjectId, key: &str) -> Result<Option<Vec<u8>>, KvError>;
fn put(&self, scope: &ObjectId, key: &str, value: &[u8]) -> Result<(), KvError>;
fn delete(&self, scope: &ObjectId, key: &str) -> Result<(), KvError>; // idempotent
fn list_keys(&self, scope: &ObjectId, prefix: Option<&str>) -> Result<Vec<String>, KvError>;
```

```rust,no_run
use platform_wallet_storage::{KvStore, ObjectId, SqlitePersister, SqlitePersisterConfig};

let persister = SqlitePersister::open(SqlitePersisterConfig::new("/tmp/wallets.db"))?;
persister.put(&ObjectId::Global, "ui.theme", b"dark")?;
let theme: Option<Vec<u8>> = persister.get(&ObjectId::Global, "ui.theme")?;
let keys = persister.list_keys(&ObjectId::Global, Some("ui."))?;
# Ok::<_, Box<dyn std::error::Error>>(())
```

### CLI usage

```text
platform-wallet-storage --db <path> migrate [--no-auto-backup]
platform-wallet-storage --db <path> backup --out <dir-or-file>
platform-wallet-storage --db <path> restore --from <backup.db> --yes
platform-wallet-storage prune --in <dir> [--keep-last N] [--max-age 30d]
```

Destructive subcommands (`restore`) REQUIRE `--yes` — invoking them without
it exits 2 with a usage error. `--no-auto-backup` opts out of the
pre-restore (or pre-migration) auto-backup; it is the only supported way to
disable auto-backup.

Wallet removal is a library-only API
([`SqlitePersister::delete_wallet`] / `delete_wallet_skip_backup`); no CLI
subcommand exposes it. `delete_wallet` returns a `DeleteWalletReport`
carrying the deleted `wallet_id` and the pre-delete `backup_path` — the
rows themselves are removed by the native FK cascade plus the `meta_*`
soft-cascade triggers, so there is no per-table receipt.

Logging: `-v` / `-vv` / `-vvv` enable `info` / `debug` / `trace`
respectively on stderr; `-q` suppresses non-error output.

Exit codes: `0` success, `1` runtime error, `2` usage error, `3` validation
failure (e.g. corrupt backup source).

**Restore exclusion.** `restore` opens a short-lived writer connection on
the destination DB and holds a SQLite-native `BEGIN EXCLUSIVE` transaction
across the entire restore body. This interlocks with every other SQLite
peer — sibling `SqlitePersister` handles, bare `rusqlite::Connection`
instances, the CLI — so concurrent writes back off via SQLite's
`busy_timeout` instead of racing the atomic swap. If a peer holds the
destination busy for longer than the timeout, `restore` returns
`WalletStorageError::RestoreDestinationLocked`. The lock conn is released
BEFORE the rename so SQLite's file handle on the old inode goes away before
the new DB takes its place.

### Cargo features

`default = ["sqlite", "cli", "secrets", "kv"]`

| Feature | Default | What it brings |
|---|---|---|
| `sqlite` | yes | SQLite persister (`platform_wallet_storage::sqlite`) and all of its native deps (`rusqlite`, `refinery`, `dpp`, `dash-sdk`, `key-wallet`, etc.) |
| `cli` | yes | Maintenance binary `platform-wallet-storage`. Implies `sqlite`. |
| `secrets` | yes | `platform_wallet_storage::secrets` submodule — zeroizing secret wrappers (`SecretBytes`, `SecretString`), the `EncryptedFileStore` Argon2id + XChaCha20-Poly1305 vault backend, and the `default_credential_store()` OS-keyring constructor. Implements the upstream `keyring_core::api::{CredentialApi, CredentialStoreApi}` SPI. |
| `kv` | yes | Per-object-type key/value metadata API (`KvStore`, `KvError`, `ObjectId`) plus its SQLite-backed impl on `SqlitePersister`. Implies `sqlite`. The `meta_*` tables are always created by V001 so DB files stay interoperable across feature combos; this gate only controls the Rust API surface. |
| `__test-helpers` | no | Crate-private `lock_conn_for_test` / `config_for_test` accessors. The double-underscore prefix follows Cargo's "do not enable from downstream" convention; the methods are also `#[doc(hidden)]`. |

`cargo build -p platform-wallet-storage --no-default-features` builds a
minimal core with neither the SQLite backend, the CLI, nor the secrets
submodule. `--no-default-features --features sqlite,cli` is the
"persister-only" build mode (no crypto dependencies).

### Persistence error model

A failed write tells the caller exactly what to do next via
`PersistenceErrorKind`:

```rust,ignore
pub enum PersistenceErrorKind {
    Transient,   // not committed, buffer preserved — caller MAY retry
    Fatal,       // unrecoverable — caller MUST NOT retry, buffer dropped
    Constraint,  // SQL constraint violation — buffer dropped (fatal for retry)
}
```

`PersistenceErrorKind` is intentionally **not** `#[non_exhaustive]`: a
future variant must force every consumer `match` to update explicitly. The
SQLite side classifies its native errors through
`WalletStorageError::persistence_kind` and exposes the retry decision
directly via `WalletStorageError::is_transient`.

### Schema

The canonical schema is [`migrations/V001__initial.rs`](./migrations/V001__initial.rs)
— 23 tables of hand-written `CREATE TABLE … FOREIGN KEY …` SQL with native
`ON DELETE CASCADE`. Foreign-key enforcement is enabled and
read-back-asserted on every connection open. For the full table reference,
the cascade triggers, the no-FK `meta_*` soft cascade, the orphan-metadata
limitation, and the enum-domain CHECK constraints, see
[SCHEMA.md](./SCHEMA.md).

### Secrets

The crate writes no secrets to SQLite. Signing material lives in the
`secrets` backends instead. For the private-key boundary, the Argon2id +
XChaCha20-Poly1305 vault, the OS-keyring arm, the `SecretStore` API, the
error surface, and the threat model, see [SECRETS.md](./SECRETS.md).
