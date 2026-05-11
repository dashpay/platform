# Changelog

All notable changes to this crate are documented here. Format loosely
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the
workspace-level [CHANGELOG.md](../../CHANGELOG.md) is generated from
Conventional Commits and remains the single source of truth for release
notes.

## [Unreleased]

### Added

- Initial implementation of `platform-wallet-sqlite`: SQLite-backed
  `PlatformWalletPersistence` with per-wallet in-memory buffer,
  atomic per-wallet flush (one transaction per call), `FlushMode`
  selection, online backup via the rusqlite Backup API, restore with
  source-integrity + schema-version validation, retention pruning
  with AND-semantics, automatic pre-migration and pre-delete
  backups, `delete_wallet` cascade with typed `DeleteWalletReport`,
  and a `delete_wallet_skip_backup` library entry for the CLI's
  `--no-auto-backup` flag.
- `platform-wallet-sqlite` CLI binary with `migrate`, `backup`,
  `restore`, `prune`, `inspect`, `delete-wallet` subcommands; `-v` /
  `-q` flags wired to `tracing_subscriber`.
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
