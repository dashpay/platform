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

`platform_wallet_storage::secrets` is part of the crate's default
feature set. The consumer entry point is `SecretStore`; the upstream
`keyring_core::api::{CredentialApi, CredentialStoreApi}` (shipped by
`keyring-core 1.0.0`) is the internal backend SPI. This crate
contributes backends and zeroizing wrappers, not the trait surface.

### Consumer API: `SecretStore`

`SecretStore` is the public, never-leaking front door. `get` yields a
zeroizing `SecretBytes` (a raw `Vec<u8>` never crosses the boundary);
`set` takes `&SecretBytes` so a caller cannot pass an unwrapped buffer.
Errors surface as the typed `SecretStoreError` — losslessly for the file
arm, so `WrongPassphrase` vs `Corruption` vs `Busy` stay distinct.

```rust
use platform_wallet_storage::secrets::{SecretBytes, SecretStore, SecretString, WalletId};

let store = SecretStore::file("/var/lib/wallet/secrets.pwsvault", SecretString::new("pw"))?;
let wallet = WalletId::from(wallet_id);
store.set(&wallet, "mnemonic", &SecretBytes::from_slice(b"abandon ability ..."))?;
let plaintext: Option<SecretBytes> = store.get(&wallet, "mnemonic")?; // never a bare Vec
store.delete(&wallet, "mnemonic")?; // idempotent
```

`SecretStore::file` takes the vault FILE path (operator picks the
filename); the parent directory is materialized on the first write.
Use `SecretStore::os()` for the platform OS keyring arm instead of
`SecretStore::file(..)`.

### Internal SPI

Below `SecretStore`, `EncryptedFileStore` and `default_credential_store`
expose the raw `keyring_core` SPI directly; their `keyring_core::Error`
projection is **lossy and string-only** (the typed distinction lives on
the `SecretStore` path). SPI consumers re-wrap the bare `Vec<u8>` from
`CredentialApi::get_secret` via `SecretBytes::new(...)` at the seam.

### Key shape

| upstream field | this crate's mapping |
|---|---|
| `service` | `"dash.platform-wallet-storage/" + hex(wallet_id)` (`SERVICE_PREFIX` + 64 hex chars) — one keyring "service" namespace per wallet |
| `user` | `label`, validated against `^[A-Za-z0-9._-]{1,64}$` (SEC-REQ-4.3) before reaching the SPI; allowlist excludes `/`, `:`, space, NUL, non-ASCII |

`WalletId` is a fixed 32-byte newtype. `validated_label` runs at
`CredentialStoreApi::build` time AND at every `CredentialApi`
operation (defence in depth — credentials are long-lived).

### Memory hygiene at the seam

`SecretStore::get` returns `Option<SecretBytes>` — a raw `Vec<u8>`
never crosses the public boundary. Internally, the upstream SPI returns
plaintext as `Vec<u8>` from `CredentialApi::get_secret`; that result is
wrapped into `SecretBytes::new(...)` **immediately**, with no named
intermediate `Vec` binding (Smythe EDIT-1). `SecretBytes::new` takes the
`Vec<u8>` by value and `std::mem::take`s it into a `Zeroizing<Vec<u8>>` —
no copy of the bare buffer ever survives past the constructor
expression, so the bare-`Vec` exposure window is zero statements. The
wrapper is also best-effort `mlock`ed and `Debug` is redacted.

`SecretStore::set` takes `&SecretBytes`, exposing the wrapped bytes to
the SPI's `set_secret(&[u8])` only at the last moment; no long-lived
unwrapped copy is allocated.

### Backends

- **File vault (`SecretStore::file` / `EncryptedFileStore`)** — Argon2id
  (memory ≥ 19 MiB, t ≥ 2, defaults 64 MiB / t=3) + XChaCha20-Poly1305
  AEAD with a random 24-byte XNonce per entry. AAD binds ciphertext to
  `format_version ‖ wallet_id ‖ label` so a blob moved between slots
  (or across wallets) fails the tag. A header-stored passphrase-
  verification token is unsealed before any entry is touched
  (mixed-key-corruption guard). The vault is ONE `serde_json` document
  covering every wallet in the store — a single passphrase, a single
  KDF salt, a single cross-process advisory lock (`<path>.lock`
  sidecar). Inside, entries are nested `BTreeMap<wallet_id_hex,
  BTreeMap<label, body>>`. The file is written atomically via
  `tempfile::NamedTempFile::persist` (cross-platform
  replace-over-existing) at mode 0600 on Unix; rekey rotates the WHOLE
  store under a fresh passphrase + salt atomically with no `.bak`
  (SEC-REQ-2.2.x). One file, one passphrase, one lock — a multi-wallet
  store cannot lock its other wallets out by construction. Errors
  surface as the typed `SecretStoreError` through `SecretStore`.
- **OS keyring (`SecretStore::os` / `default_credential_store`)** —
  returns an `Arc<dyn CredentialStoreApi + Send + Sync>` over the
  platform's default credential store. The backend on Linux/FreeBSD is
  `dbus-secret-service-keyring-store`; on macOS
  `apple-native-keyring-store`; on Windows
  `windows-native-keyring-store`. Fail-closed with
  `keyring_core::Error::NoDefaultStore` on headless / unknown OS
  (SEC-REQ-2.1.3 / AR-4) — never a silent plaintext fallback. Through
  `SecretStore`, keyring failures project to
  `SecretStoreError::OsKeyring { kind }`, a non-secret discriminant.

  **Headless caveat (Linux/FreeBSD).** Secret Service requires a D-Bus
  session and an unlocked collection; headless / SSH / CI hosts
  frequently lack it, in which case `SecretStore::os()` fails closed
  with `NoDefaultStore`. Callers that need durable storage on a
  headless host should pin `SecretStore::file(...)` (encrypted-file
  vault) instead of relying on the OS keyring.
- **Tests** — integration tests construct a tempdir-backed
  `EncryptedFileStore` directly via
  `EncryptedFileStore::open(tempfile::tempdir()?.path().join("vault.pwsvault"), SecretString::new("..."))`,
  or use the public `SecretStore::file(path, passphrase)` constructor.
  No special feature flag is required; both are available under the default
  `secrets` feature.

Backend selection is an explicit operator decision; there is no
automatic fallback between backends.

### Error surface

`SecretStore` returns the typed `SecretStoreError`. For the file arm this
is **lossless**: `WrongPassphrase`, `Corruption`, `Busy`, `KdfFailure`,
`VersionUnsupported`, `MalformedVault`, `InsecurePermissions`, and
`InvalidLabel` are distinct typed variants. For the OS arm,
`keyring_core::Error` projects best-effort into
`SecretStoreError::OsKeyring { kind: OsKeyringErrorKind }`, a payload-free
discriminant — keyring variants carrying raw bytes (`BadEncoding`,
`BadDataFormat`) are collapsed so their bytes never enter the error
(CWE-209/CWE-532).

The internal SPI projection `From<SecretStoreError> for
keyring_core::Error` keeps the `WrongPassphrase` / `Busy` variants
recoverable: they ride in `NoStorageAccess` with the typed
`SecretStoreError` boxed as the source, so an SPI-only consumer can recover
them via `err.source().and_then(|s| s.downcast_ref::<SecretStoreError>())`.
The `BadStoreFormat` group (`Corruption`, `KdfFailure`,
`VersionUnsupported`, `MalformedVault`, `InsecurePermissions`, `Decrypt`,
`OsKeyring`) has no box slot and carries only a secret-free string; those
remain fully typed on the `SecretStore` path.

Per Smythe EDIT-2, `keyring_core::Error` is safe to `Display`
(`{ }`-format), but `{:?}`-format embeds `BadEncoding(Vec<u8>)` /
`BadDataFormat(Vec<u8>, _)` payloads — those variants are NEVER
constructed by our backends with secret bytes, and
`tests/secrets_guard.rs` enforces that no debug-format pairs with
`keyring_core::Error` inside `src/secrets/`.

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
  allow-list with a rationale. The `src/secrets/` directory is exempt
  by design (its own positive guard below covers it).
- **`tests/secrets_guard.rs`**: positive secret-leak guard for
  `src/secrets/`. Forbids logging/formatting sinks that pair with
  `expose_secret(...)` on the same logical statement (SEC-REQ-4.5.1),
  AND forbids `{:?}`-debug-format paired with `keyring_core::Error`
  (Smythe EDIT-2).
- **`tests/secrets_api.rs`**: shape guards — `CredentialApi::get_secret`
  re-wraps through `SecretBytes::new` (EDIT-1), redacting `Debug` on
  `SecretBytes`/`SecretString`, no `Box<dyn Error>` in `src/secrets/`
  (TC-082 parity).
- **`tests/secrets_off_state.rs`**: runtime guard that
  `--no-default-features --features sqlite,cli` builds the persister
  without pulling in the `secrets` module (D4).
- **NFR-4 / TC-082** (`tests/sqlite_persist_roundtrip.rs::tc082_no_box_dyn_error_in_src`):
  all public method signatures use concrete error types
  (`WalletStorageError`, `PersistenceError`) — never
  `Box<dyn Error>` — so a future leak is caught by `grep`.

The CI advisory check runs `rustsec/audit-check` over `Cargo.lock`;
because `secrets` is in the default feature set, the pinned
`argon2` / `chacha20poly1305` / `zeroize` / `subtle` / `getrandom`
(the `OsRng` source for the salt + per-entry nonces, specified as the
semver range `getrandom = "0.2"` and lock-pinned to 0.2.17 by
lock-file convention) / `region` / `keyring-core` / per-platform store
crate versions are unconditionally in the lockfile and therefore
unconditionally in audit scope (SEC-REQ-4.7).

## Backup retention and secrets

Manual / auto backups are byte-for-byte copies of the live DB. They
inherit the same "no secrets in the file" invariant. Operators may
still want to encrypt backups at rest using a file-system level tool
(GnuPG, age, encfs); this crate does not do that for them and never
ships SQLCipher.

## Future work — maintenance CLI

A unified `platform-wallet-storage secrets <subcommand>` CLI is planned as a follow-up to give operators a way to inspect and manage the secret backends without writing custom code. Out of scope for this PR (#3672); tracked separately. Two commands matter:

- **`secrets probe`** — set/get/delete a `__probe__` entry under `SERVICE_PREFIX`. Works uniformly on **all** backends (Secret Service, macOS Keychain, Windows Credential Manager) because it only uses single-entry CRUD. Confirms backend liveness + write-path responsiveness — the canary command for "is the keyring actually wired up on this machine?". Cheap to implement (~30 lines).
- **`secrets list [--filter <prefix>]`** — enumerate `(wallet_id, label)` pairs in the store. Trivial on the file vault (iterate the in-memory `BTreeMap`). On the OS arm: works on Secret Service, macOS Keychain, and Windows Credential Manager via `CredentialStoreApi::search`. Operators on headless Linux without a Secret Service session must select the file vault explicitly.

Other planned subcommands: `secrets put <svc> <label> <hex|@file>`, `secrets delete <svc> <label>`, `secrets rekey <new-passphrase>` (file-vault only). `secrets get` is deliberately omitted (printing a secret to stdout defeats `SecretBytes` zeroize); if added, must require an explicit `--unsafe-print-secret` flag.

[`SecretBytes::new(...)`]: ./src/secrets/secret.rs
[`SecretStoreError`]: ./src/secrets/error.rs
