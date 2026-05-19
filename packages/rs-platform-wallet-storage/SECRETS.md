# Private-key boundary

The SQLite persister in `platform-wallet-storage::sqlite` is the
canonical persistence backend for the data carried by
`PlatformWalletPersistence` — UTXOs, identities, identity public keys,
contacts, asset locks, token balances, DashPay overlays, address-pool
snapshots. **None of that is secret material.**

Mnemonics, seeds, raw private keys, and any other long-lived signing
material live exclusively on the client side (iOS Keychain, Android
Keystore, OS keyring, encrypted file vault). They are re-derived as
needed via the wallet's BIP-32/BIP-39 plumbing and never touch the
SQLite file the persister writes.

## The `secrets` submodule

`platform_wallet_storage::secrets` is gated behind the opt-in `secrets`
Cargo feature (never enabled by `default`). Enabling the feature
activates the module: it pulls the pinned crypto/keyring dependencies
and compiles `src/secrets/`. Secrets reach a backend only through this
trait — never through the SQLite persister DTO.

```rust
pub trait SecretStore: Send + Sync {
    fn put(&self, wallet_id: WalletId, label: &str, bytes: &[u8])
        -> Result<(), SecretStoreError>;
    fn get(&self, wallet_id: WalletId, label: &str)
        -> Result<Option<SecretBytes>, SecretStoreError>;
    fn delete(&self, wallet_id: WalletId, label: &str)
        -> Result<(), SecretStoreError>;
}
```

`get` returns `Option<SecretBytes>` — a zeroize-on-drop wrapper, never
a bare `Vec<u8>`. `label` is validated against
`^[A-Za-z0-9._-]{1,64}$`; `wallet_id` is a fixed 32-byte newtype.
`SecretStoreError` is a concrete `thiserror` enum carrying no secret
bytes.

Backends:

- `KeyringStore` — OS-native keyring (`keyring-core 1.0.0` + the
  per-platform store crates). Recommended default on desktop OSes;
  fails closed (`BackendUnavailable`) on headless Linux with no Secret
  Service — never a silent plaintext fallback.
- `EncryptedFileStore` — Argon2id + XChaCha20-Poly1305 vault file with
  a header-stored passphrase-verification token. Recommended default
  on headless / server hosts.
- `MemoryStore` — tests only, gated behind `__secrets-test-helpers` so
  it is unreachable from production builds.

Backend selection is an explicit operator decision; there is no
automatic fallback between backends.

## What the SQLite backend WILL refuse to store

The `identity_keys` table is for **public** material only — DPP
public keys, public-key hashes, optional DIP-9 derivation breadcrumbs.
If a sub-changeset ever gains a `private_key_bytes`-style field, the
trait conversation must reopen: the persister boundary stays
secret-free.

## Audit hooks

- **`tests/secrets_scan.rs`**: greps every file under
  `src/sqlite/schema/` and `migrations/` for the substrings `private`,
  `mnemonic`, `seed`, `xpriv`, `secret`. A new column, blob field, or
  comment that uses any of those words breaks the test — forcing the
  author to either rename, or add their phrase to the file's
  allow-list with a rationale. The future `src/secrets/` directory is
  exempt by design.
- NFR-4 / TC-082 (`tests/sqlite_persist_roundtrip.rs::tc082_no_box_dyn_error_in_src`):
  all public method signatures use concrete error types
  (`SqlitePersisterError`, `PersistenceError`) — never
  `Box<dyn Error>` — so a future leak is caught by `grep`.

## Backup retention and secrets

Manual / auto backups are byte-for-byte copies of the live DB. They
inherit the same "no secrets in the file" invariant. Operators may
still want to encrypt backups at rest using a file-system level tool
(GnuPG, age, encfs); this crate does not do that for them and never
ships SQLCipher.
