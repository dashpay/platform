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

## Cargo features

| Feature | Default | What it brings |
|---|---|---|
| `sqlite` | yes | SQLite persister (`platform_wallet_storage::sqlite`) and all of its native deps (`rusqlite`, `refinery`, `dpp`, `dash-sdk`, `key-wallet`, etc.) |
| `cli` | yes | Maintenance binary `platform-wallet-storage`. Implies `sqlite`. |
| `secrets` | yes | `platform_wallet_storage::secrets` submodule — zeroizing secret wrappers (`SecretBytes`, `SecretString`), the `EncryptedFileStore` Argon2id + XChaCha20-Poly1305 vault backend, and the `default_credential_store()` OS-keyring constructor. Implements the upstream `keyring_core::api::{CredentialApi, CredentialStoreApi}` SPI. |
| `__test-helpers` | no | Crate-private `lock_conn_for_test` / `config_for_test` accessors. The double-underscore prefix follows Cargo's "do not enable from downstream" convention; the methods are also `#[doc(hidden)]`. |
| `__secrets-test-helpers` | no | Exposes `secrets::MemoryCredentialStore`, the in-RAM test double. Double-underscore = unreachable from production builds. |

`cargo build -p platform-wallet-storage --no-default-features` builds a
minimal core with neither the SQLite backend, the CLI, nor the secrets
submodule. `--no-default-features --features sqlite,cli` is the
"persister-only" build mode (no crypto dependencies).

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
