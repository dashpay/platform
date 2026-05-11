# Private-key boundary

`platform-wallet-sqlite` is the canonical persistence backend for the
data carried by `PlatformWalletPersistence` — UTXOs, identities,
identity public keys, contacts, asset locks, token balances, DashPay
overlays, address-pool snapshots. **None of that is secret material.**

Mnemonics, seeds, raw private keys, and any other long-lived signing
material live exclusively on the client side (iOS Keychain, Android
Keystore, OS keyring, encrypted file vault). They are re-derived as
needed via the wallet's BIP-32/BIP-39 plumbing and never touch this
SQLite file.

## Sibling crate sketch

A future `platform-wallet-secrets` crate will host the `SecretStore`
trait:

```rust
trait SecretStore: Send + Sync {
    fn put(&self, wallet_id: WalletId, label: &str, bytes: &[u8]) -> Result<()>;
    fn get(&self, wallet_id: WalletId, label: &str) -> Result<Option<Vec<u8>>>;
    fn delete(&self, wallet_id: WalletId, label: &str) -> Result<()>;
}
```

Reference backends to plan for:

- `KeyringStore` (default) — OS-native keyring; recoverable across
  reinstalls when the keyring is.
- `EncryptedFileStore` — Argon2id + XChaCha20-Poly1305 over a passphrase.
- `MemoryStore` — tests only.

## What this crate WILL refuse to store

The `identity_keys` table is for **public** material only — DPP
public keys, public-key hashes, optional DIP-9 derivation breadcrumbs.
If a sub-changeset ever gains a `private_key_bytes`-style field, the
trait conversation must reopen: the persister boundary stays
secret-free.

## Audit hooks

- **`tests/secrets_scan.rs`**: greps every file under `src/schema/`
  and `migrations/` for the substrings `private`, `mnemonic`, `seed`,
  `xpriv`, `secret`. A new column, blob field, or comment that uses
  any of those words breaks the test — forcing the author to either
  rename, or add their phrase to the file's allow-list with a
  rationale.
- NFR-4 / TC-082 (`tests/persist_roundtrip.rs::tc082_no_box_dyn_error_in_src`):
  all public method signatures use concrete error types
  (`SqlitePersisterError`, `PersistenceError`) — never
  `Box<dyn Error>` — so a future leak is caught by `grep`.

## Backup retention and secrets

Manual / auto backups are byte-for-byte copies of the live DB. They
inherit the same "no secrets in the file" invariant. Operators may
still want to encrypt backups at rest using a file-system level tool
(GnuPG, age, encfs); this crate does not do that for them and never
ships SQLCipher.
