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
feature set. The SPI is upstream's
`keyring_core::api::{CredentialApi, CredentialStoreApi}` shipped by
`keyring-core 1.0.0`; this crate contributes backends and zeroizing
wrappers, not the trait surface.

```rust
use keyring_core::api::{CredentialApi, CredentialStoreApi};
use platform_wallet_storage::secrets::{
    EncryptedFileStore, SecretBytes, SecretString, WalletId, SERVICE_PREFIX,
};

let store = EncryptedFileStore::open("/var/lib/wallet/vault", SecretString::new("pw"))?;
let service = format!("{SERVICE_PREFIX}{}", WalletId::from(wallet_id).to_hex());
let entry = store.build(&service, "mnemonic", None)?;
entry.set_secret(b"abandon ability ...")?;
let plaintext = SecretBytes::new(entry.get_secret()?); // re-wrap at the consumer seam
```

### Key shape

| upstream field | this crate's mapping |
|---|---|
| `service` | `"dash.platform-wallet-storage/" + hex(wallet_id)` (`SERVICE_PREFIX` + 64 hex chars) — one keyring "service" namespace per wallet |
| `user` | `label`, validated against `^[A-Za-z0-9._-]{1,64}$` (SEC-REQ-4.3) before reaching the SPI; allowlist excludes `/`, `:`, space, NUL, non-ASCII |

`WalletId` is a fixed 32-byte newtype. `validated_label` runs at
`CredentialStoreApi::build` time AND at every `CredentialApi`
operation (defence in depth — credentials are long-lived).

### Memory hygiene at the seam

The upstream SPI returns plaintext as `Vec<u8>` from
`CredentialApi::get_secret`. The contract: callers MUST wrap that
result into [`SecretBytes::new(...)`] **immediately**, with no named
intermediate `Vec` binding (Smythe EDIT-1). `SecretBytes::new` takes
the `Vec<u8>` by value and `std::mem::take`s it into a
`Zeroizing<Vec<u8>>` — no copy of the bare buffer ever survives past
the constructor expression, so the bare-`Vec` exposure window is zero
statements. The wrapper is also best-effort `mlock`ed and `Debug` is
redacted.

`CredentialApi::set_secret` accepts `&[u8]` (a borrow); no long-lived
unwrapped copy is allocated.

### Backends

- **`EncryptedFileStore`** — Argon2id (memory ≥ 19 MiB, t ≥ 2, defaults
  64 MiB / t=3) + XChaCha20-Poly1305 AEAD with random 24-byte XNonce
  per entry. AAD binds ciphertext to
  `format_version ‖ wallet_id ‖ label` so a blob moved between slots
  fails the tag. A header-stored passphrase-verification token is
  unsealed before any entry is touched (mixed-key-corruption guard).
  Vault file created at mode 0600 via `O_EXCL`+`fchmod`; writes
  temp→fsync→rename→dir-fsync; rekey replaces atomically with no
  `.bak` (SEC-REQ-2.2.x). Construction errors surface as
  [`FileStoreError`]; the `CredentialApi` seam bridges them through
  the unit-only [`FileStoreFailure`] marker boxed inside
  `keyring_core::Error::{NoStorageAccess, BadStoreFormat}` payloads.
  Consumers recover the marker via `secrets::downcast_failure(&err)`.
- **OS keyring** — `secrets::default_credential_store()` returns an
  `Arc<dyn CredentialStoreApi + Send + Sync>` over the platform's
  default credential store (`linux-keyutils-keyring-store` →
  `dbus-secret-service-keyring-store` on Linux/FreeBSD;
  `apple-native-keyring-store` on macOS; `windows-native-keyring-store`
  on Windows). Fail-closed with `keyring_core::Error::NoDefaultStore`
  on headless / unknown OS (SEC-REQ-2.1.3 / AR-4) — never a silent
  plaintext fallback. The returned `Arc` is suitable for
  `keyring_core::set_default_store(...)`.
- **`MemoryCredentialStore`** — gated behind `__secrets-test-helpers`;
  unreachable from production builds.

Backend selection is an explicit operator decision; there is no
automatic fallback between backends.

### The cross-SPI error bridge

`keyring_core::Error` does not name file-backend-unique failure modes
(wrong passphrase, malformed vault, insecure permissions, KDF
failure). The file backend boxes a unit-only [`FileStoreFailure`]
inside `keyring_core::Error::NoStorageAccess` (for `WrongPassphrase`,
matching the operator UX of `KeyringLocked`) or renders it into
`BadStoreFormat`'s static `String` payload (for `Decrypt`,
`KdfFailure`, `VersionUnsupported`, `MalformedVault`,
`InsecurePermissions`). `secrets::downcast_failure(&err)` recovers the
typed variant; the bridge is the single recovery path consumers use.

[`FileStoreFailure`] is **unit-variants only** (Smythe EDIT-3): no
field may carry a user-supplied path, secret byte, passphrase, label,
or stringified payload. Numeric correlation fields are acceptable; the
current taxonomy needs none. The constraint is enforced via a
compile-time `Copy` assertion in the bridge module.

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
  (`SqlitePersisterError`, `PersistenceError`) — never
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

[`SecretBytes::new(...)`]: ./src/secrets/secret.rs
[`FileStoreError`]: ./src/secrets/file/error.rs
[`FileStoreFailure`]: ./src/secrets/file/error_bridge.rs
