# Changelog

All notable changes to this crate are documented here. Format loosely
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the
workspace-level [CHANGELOG.md](../../CHANGELOG.md) is generated from
Conventional Commits and remains the single source of truth for release
notes.

## [Unreleased]

### Changed

- **Blob encoder swapped to bincode-serde.** Every `_blob` column
  (`core_transactions.record_blob`, `core_instant_locks.islock_blob`,
  `identities.entry_blob`, `identity_keys.public_key_blob`,
  `contacts_*.entry_blob`, `asset_locks.lifecycle_blob`,
  `dashpay_*.{profile,overlay}_blob`,
  `account_registrations.account_xpub_bytes`,
  `account_address_pools.snapshot_blob`) is now a single
  `bincode::serde::encode_to_vec` payload prefixed with a 1-byte
  schema-revision tag. The hand-rolled `BlobWriter` / `BlobReader`
  walker from the initial implementation is gone; the schema-writer
  modules each shed ~30-100 LOC of field-by-field plumbing.
  `IdentityKeyEntry` keeps a tiny wire-shape adapter
  (`IdentityKeyWire`) inside the storage crate because dpp's
  `IdentityPublicKey` uses `serde(tag = "$formatVersion")`, which
  bincode-serde rejects — the adapter re-encodes that one field via
  bincode 2's native `Encode/Decode` derives while everything around
  it still rides bincode-serde.
- **Crate renamed**: `platform-wallet-sqlite` → `platform-wallet-storage`.
  Module layout regrouped under `platform_wallet_storage::sqlite`; root
  re-exports (`SqlitePersister`, `SqlitePersisterConfig`, `FlushMode`,
  `SqlitePersisterError`, `RetentionPolicy`, `PruneReport`,
  `DeleteWalletReport`, `AutoBackupOperation`, `JournalMode`,
  `Synchronous`) preserved so most import sites stay identical.
- Bin renamed to `platform-wallet-storage` (matching the crate name).
  All `--db` / `--out` / subcommand flags unchanged.
- Cargo features reshaped: the SQLite backend is now gated by the
  default-on `sqlite` feature; `cli` (default-on) implies `sqlite`;
  `secrets` is reserved as a no-op slot for the future
  `SecretStore` submodule.
- Downstream consumers should update `Cargo.toml` to
  `platform-wallet-storage = { … }` and (if they were reaching past
  the root re-exports) replace `platform_wallet_sqlite::` with
  `platform_wallet_storage::` or
  `platform_wallet_storage::sqlite::`.

### Added

- Initial implementation: SQLite-backed `PlatformWalletPersistence`
  with per-wallet in-memory buffer, atomic per-wallet flush (one
  transaction per call), `FlushMode` selection, online backup via
  the rusqlite Backup API, restore with source-integrity +
  schema-version validation, retention pruning with AND-semantics,
  automatic pre-migration and pre-delete backups, `delete_wallet`
  cascade with typed `DeleteWalletReport`, and a
  `delete_wallet_skip_backup` library entry for the CLI's
  `--no-auto-backup` flag.
- Maintenance CLI binary `platform-wallet-storage` with `migrate`,
  `backup`, `restore`, `prune`, `inspect`, `delete-wallet`
  subcommands; `-v` / `-q` flags wired to `tracing_subscriber`.
- 18-table SQLite schema, FK enforcement emulated via triggers
  (barrel cannot emit composite-key FK clauses portably on SQLite).
- 55+ tests covering migrations, buffer semantics, FK cascade,
  backup / restore / retention, auto-backup behaviour, load
  reconstruction (wired-up subset), CLI smoke, compile-time
  assertions (`Send + Sync`, object-safety, no `Box<dyn Error>`,
  schema-file secrets scan).

### Security

- `restore_from` stages the source via `tempfile::NamedTempFile`
  with an unguessable filename in the destination's parent
  directory, then `persist`s atomically — eliminates the TOCTOU
  symlink-plant window on a predictable temp path.
- `restore_from` try-acquires an exclusive file lock on the
  destination (via `fs2`) before staging; surfaces
  `RestoreDestinationLocked` if another process holds the file.
- `restore_from` raises `SchemaVersionUnsupported` when the source
  DB's schema version exceeds what this build's embedded migrations
  cover — prevents silent downgrades on cross-version restores.
- `delete_wallet` checks `wallet_metadata` existence BEFORE writing
  the pre-delete backup — refusal on an unknown id no longer leaves
  an orphaned `.db` in the auto-backup directory.

[Unreleased]: https://github.com/dashpay/platform/tree/v3.1-dev
