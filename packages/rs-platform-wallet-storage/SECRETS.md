# Secret storage and the private-key boundary

## Why secrets are handled this way

A wallet's public state and its signing material have very different risk
profiles. The persister's SQLite file is meant to be copied, backed up, and
restored freely — so the one thing it must never contain is a key that could
move funds. Keeping signing material out of that file by construction is what
makes the rest of the crate safe to operate casually: you can back up the
`.db` without backing up your keys.

So secrets get their own home, their own crypto, and their own typed,
secret-free error surface — separate from the persister entirely.

## The value: a hard private-key boundary

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

The rest of this document is the technical detail behind that boundary: the
`secrets` backends, the `SecretStore` API, the error surface, and the threat
model.

### Exception: the KV metadata API stores caller-supplied plaintext

The boundary above is about the persister's own domain state. The
separate `KvStore` API (`kv` feature) is a deliberate, explicit exception:
it stores **arbitrary caller-supplied `Vec<u8>` values as PLAINTEXT** in
`meta_*` BLOB columns of the same `.db` (and therefore in every backup).
There is no encryption and no runtime content guard — the safety is
**caller-policed**. Callers MUST NOT put key or signing material through
`KvStore`; that is what `SecretStore` is for. The `KvStore` /
`KvStore::put` rustdoc carries the same `# Security` warning.

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
arm, so `WrongPassphrase` vs `Corruption` vs `AlreadyLocked` stay distinct.

```rust
use platform_wallet_storage::secrets::{SecretBytes, SecretStore, SecretString, WalletId};

let store = SecretStore::file("/var/lib/wallet/secrets.pwsvault", SecretString::new("pw"))?;
let wallet = WalletId::from(wallet_id);

// Tier-1 only (unprotected by an object password). `set`/`get` are
// `..,None` wrappers over `set_secret`/`get_secret`.
store.set(&wallet, "mnemonic", &SecretBytes::from_slice(b"abandon ability ..."))?;
let plaintext: Option<SecretBytes> = store.get(&wallet, "mnemonic")?; // never a bare Vec

// Tier-2: protect a critical object under an extra OBJECT PASSWORD that
// the backend never sees. Reading it back REQUIRES the password.
let pw = SecretString::new("a strong object password");
store.set_secret(&wallet, "seed", &SecretBytes::from_slice(b"<seed>"), Some(&pw))?;
let seed = store.get_secret(&wallet, "seed", Some(&pw))?; // Some(secret)
// Reading a protected object WITHOUT the password fails closed:
assert!(store.get_secret(&wallet, "seed", None).is_err()); // NeedsPassword

// Add / change / remove an object password in one atomic same-slot flow:
store.reprotect(&wallet, "seed", Some(&pw), None)?; // remove → now unprotected
store.delete(&wallet, "mnemonic")?; // idempotent
```

`SecretStore::file` takes the vault FILE path (operator picks the
filename); the parent directory is materialized on the first write.
Use `SecretStore::os()` for the platform OS keyring arm instead of
`SecretStore::file(..)`.

See **Two-tier secret protection** below for the model, the envelope
format, which tier defeats which adversary, and the strict fail-closed
read that is the heart of the opt-in scheme.

### Two-tier secret protection

Secret protection comes in two layers. Tier-1 is always on (it is just
"which backend you opened"); Tier-2 is opt-in, per critical object, and
backend-independent.

| Tier | Provided by | Defeats | Mechanism |
|---|---|---|---|
| **1 — backend baseline** | the *backend* | another local user, a lost laptop, the vault at rest | OS keychain ACLs **or** Argon2id + XChaCha20-Poly1305 vault under a **real** passphrase |
| **2 — per-object password** | the *library*, above `SecretStore`, over **both** arms | **backend compromise** — the keychain scraped, or the vault stolen *and* its passphrase cracked | the object's bytes are Argon2id + XChaCha20-Poly1305 **enveloped under a per-object password BEFORE they reach the backend** |

**Why Tier-2 is more than key granularity.** Its value is not a sub-key —
it is (a) an **independent human password the backend never sees** and (b)
**envelope-before-backend ordering**, so for a protected object the backend
only ever stores ciphertext. That is the first and only control that keeps
a chosen critical object confidential across a *full* backend compromise
(the A2/A3/A6 gap Tier-1 leaves open).

Tier-2 has two guarantees of different strength:

- **Confidentiality** (an attacker cannot *read* a protected secret) is
  **unconditional** — the object password never enters any backend, so a
  full backend dump yields only ciphertext + a per-object salt to
  offline-Argon2id-crack against the password's entropy.
- **Integrity / anti-downgrade** is delivered by the **strict fail-closed
  read** below and is **conditional on the caller's trusted model staying
  intact** (see the documented residual).

#### The envelope (wire format)

Every value written through `set_secret`/`set` is wrapped in a
self-describing, authenticated envelope before it reaches the backend. The
backend (file vault or OS keychain) stores only these opaque bytes.

```text
magic    b"PWSEV"        (5)
version  u8 = 1          (envelope version — independent of the vault FORMAT_VERSION)
scheme   u8              (0 = unprotected passthrough, 1 = password)
── scheme 0 ──  payload: the raw secret bytes
── scheme 1 ──  kdf(id u8 ‖ m_kib u32 LE ‖ t u32 LE ‖ p u32 LE)  (13)
                ‖ salt[32] ‖ nonce[24] ‖ ciphertext+tag
```

- **AAD (scheme 1)** binds `domain ‖ magic ‖ version ‖ scheme ‖ kdf ‖ salt
  ‖ wallet_id ‖ label` (length-prefixed), mirroring the vault's own
  `aad()`/`verify_aad()`. A protected blob relocated to another slot — or
  any in-place header edit — fails the tag (relocation/header-tamper
  resistance). On the file arm this AAD is *in addition* to the vault's own
  per-entry AAD + tag; on the OS arm it is the only authentication layer.
- **KDF ceiling before derivation (anti-DoS).** The KDF params live in the
  (attacker-controllable) header, so on a read the Argon2 **ceiling is
  enforced before** any derivation/allocation — a forged `m_kib`/`t` cannot
  force a giant allocation or an unbounded stall on the victim's unlock.
- **No vault format bump.** The envelope lives *inside* the entry bytes,
  identical over File and Os, so there is no vault-parser or migration
  change.
- **Size cap.** The plaintext is capped at `MAX_PLAINTEXT_LEN`
  (`MAX_SECRET_LEN − MAX_ENVELOPE_OVERHEAD` = 64 KiB − 128 = 65 408 bytes),
  uniformly for both schemes, so the enveloped bytes always fit the
  backend's own `MAX_SECRET_LEN` cap and the user-visible limit is stable
  regardless of scheme. Oversize → `SecretTooLarge { found, max }` with
  `max = MAX_PLAINTEXT_LEN` (re-exported as `secrets::MAX_PLAINTEXT_LEN`).
- **Unknown version/scheme** (magic present) → `UnsupportedEnvelopeVersion`
  — fail closed **regardless of the password**: an unparseable future
  format can be neither safely unwrapped nor treated as unprotected.

#### The strict, fail-closed read

The defining risk of any opt-in "some objects are extra-protected" scheme
is **strip / downgrade**: an attacker who can WRITE the backend replaces a
protected blob with a fresh, internally-valid *unprotected* (scheme-0) blob
carrying a chosen seed/xpriv. There is nothing in that blob alone to prove
an envelope was *expected*, so inferring protection from the stored bytes
would silently return the attacker's secret — funds redirection, password
prompt bypassed.

The fix: **the "expected-protected" bit lives in the CALLER's trusted
model, surfaced solely by whether a password is supplied to `get_secret` —
NEVER inferred from the blob.** The library does not guess and does not
persist the expectation. A supplied password *is* the assertion "this
object must be protected":

| `password` arg | stored blob | result |
|---|---|---|
| `Some(pw)` | valid scheme-1 | the secret, or `WrongPassword` on tag fail |
| **`Some(pw)`** | **scheme-0 / legacy magic-less raw** | **`ExpectedProtectedButUnsealed` — FAIL CLOSED** |
| `Some(pw)` | scheme-1 but truncated/corrupt | `Corruption` |
| `Some/None` | magic present, unknown version/scheme | `UnsupportedEnvelopeVersion` |
| `None` | valid scheme-1 | `NeedsPassword` (never ciphertext) |
| `None` | scheme-0 | the secret |
| `None` | legacy magic-less raw | the secret (+ a one-time warning; re-wrapped on next write) |
| `None` | magic present but truncated header | `Corruption` |
| any | absent entry | `Ok(None)` (deletion = DoS, never injection) |

The load-bearing row is **`Some(pw)` + non-envelope ⇒
`ExpectedProtectedButUnsealed`**: with a password in hand, a non-protected
blob can only mean a strip, so it is refused and **no bytes are returned**.
A consumer bug alone — over- or under-supplying a password — fails closed
in *every* direction.

**Arm asymmetry.** On the file arm the stored bytes are themselves sealed
under the vault key, so producing a *readable* stripped blob at a slot
requires the vault key; a cold/backup-swap actor can only corrupt
(→ DoS), not inject-to-readable. On the OS-keychain arm the stored item is
the bare envelope with no second seal, so the strip defence there leans
entirely on the `Some(pw)` strict rule plus the consumer's metadata
integrity — this is where the residual bites hardest.

**Documented residual (out of the library's reach).** If an attacker ALSO
rewrites the consumer's trusted DB so the consumer calls `get_secret(X,
None)` for a stripped object, the `(scheme-0, None)` quadrant returns the
attacker's bytes. The library only ever sees the blob and the caller's
`Some/None`; the "should be protected" fact lives entirely in the
consumer's metadata store. **Anti-downgrade strength therefore equals the
tamper-resistance of the consumer's protection-status record** — store it
as integrity-protected, security-critical state (it is one more field
alongside the addresses/policy the wallet DB must already protect).

**Value rollback is NOT defended.** Restoring an *older valid* scheme-1
envelope under the *current* password decrypts cleanly. The strict read
closes the strip/downgrade injection, not value rollback; if
backup-swap/restore-old is in scope, anchor a monotonic version in
integrity-protected consumer metadata. Do not mistake the strict read for
rollback protection.

#### Add / change / remove an object password

`reprotect(service, label, current, new)` does it in one same-slot
unwrap→rewrap→overwrite: read under the `current` expectation (so a strip
is caught before any rewrite), then write under `new` — `None`→`Some` adds,
`Some`→`Some` changes, `Some`→`None` removes. An absent object is a no-op
(`Ok(())`). The rewrite is a same-slot overwrite — atomic on the file arm,
and on the OS arm inheriting the backend's single-item-replace contract —
so a crash between the read and the commit leaves the prior value intact
and readable under `current`. **After a successful call the consumer MUST
update its own protection-status record** (the protection expectation lives
there). There is **no password recovery** — losing an object password
bricks that object (an availability trade-off the UX must state plainly).

#### Entropy policy is the consumer's

The library enforces only **non-blank** at enrol (and a coarse
`MIN_PASSPHRASE_LEN` floor, `1` today = merely non-blank) for both the
vault passphrase and the Tier-2 object password. It ships **no**
password-strength estimator: real entropy policy (zxcvbn-style strength,
dictionary checks, UX feedback) is locale- and threat-specific and is the
**consumer's responsibility**. For a protected object the password's
entropy is the *whole* guarantee against an offline Argon2id attacker who
already holds the backend — choose it accordingly.

#### Greenfield / legacy entries

The envelope is net-new, so post-feature reads/writes go through it. A
decrypted entry that lacks the `PWSEV` magic is treated as a **legacy
unprotected** value: returned on a `None` read (with a one-time warning,
and re-wrapped on the next write) and refused (`ExpectedProtectedButUnsealed`)
on a `Some(pw)` read — so legacy tolerance never weakens the strict read.
(A pre-feature build that persisted vault files is a deployment fact outside
this crate; the legacy-tolerant read makes the transition seamless either
way.)

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
| `user` | `label`, validated against `^[A-Za-z0-9._-]{1,64}$` before reaching the SPI; allowlist excludes `/`, `:`, space, NUL, non-ASCII |

`WalletId` is a fixed 32-byte newtype. `validated_label` runs at
`CredentialStoreApi::build` time AND at every `CredentialApi`
operation (defence in depth — credentials are long-lived).

### Memory hygiene at the seam

`SecretStore::get` returns `Option<SecretBytes>` — a raw `Vec<u8>`
never crosses the public boundary. Internally, the upstream SPI returns
plaintext as `Vec<u8>` from `CredentialApi::get_secret`; that result is
wrapped into `SecretBytes::new(...)` **immediately**, with no named
intermediate `Vec` binding. `SecretBytes::new` takes the
`Vec<u8>` by value and `std::mem::take`s it into a `Zeroizing<Vec<u8>>` —
no copy of the bare buffer ever survives past the constructor
expression, so the bare-`Vec` exposure window is zero statements. The
wrapper is also best-effort `mlock`ed and `Debug` is redacted.

`SecretStore::set` takes `&SecretBytes`, exposing the wrapped bytes to
the SPI's `set_secret(&[u8])` only at the last moment; no long-lived
unwrapped copy is allocated.

### Backends

- **File vault (`SecretStore::file` / `EncryptedFileStore`)** — Argon2id
  (memory ≥ 19 MiB, t ≥ 2, p = 1; defaults 64 MiB / t=3; ceilings 1 GiB /
  t=16 — header parameters above the ceiling are refused before any
  derivation or allocation runs, so a crafted vault cannot force a
  multi-GiB allocation or unbounded-time derivation) + XChaCha20-Poly1305
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
  store under a fresh passphrase + salt atomically with no `.bak`.
  One file, one passphrase, one lock — a multi-wallet
  store cannot lock its other wallets out by construction. Errors
  surface as the typed `SecretStoreError` through `SecretStore`.
  On Unix the vault's parent directory must not be group/other writable
  (`mode & 0o022`): directory write access governs rename/replace of the
  vault, so a writable parent is refused at `open` with
  `SecretStoreError::InsecureParentDir` (the A1 guarantee depends on it).
  A read-only group-accessible parent (`0o750`) is accepted — it only
  leaks filenames, never the 0600-protected vault contents.
  Each secret is capped at `MAX_SECRET_LEN` (64 KiB) at the write
  boundary — generously above any mnemonic/seed/xpriv — so a single
  oversized entry cannot inflate the shared document past the read-side
  128 MiB ceiling and brick every wallet on the next open. (Through
  `SecretStore::set_secret`/`set` the user-facing plaintext cap is the
  slightly lower `MAX_PLAINTEXT_LEN`, leaving room for the envelope
  overhead; see **Two-tier secret protection**.)
  **Blank passphrase is rejected.** `open` (and `rekey`) refuse a blank
  (empty / all-whitespace) passphrase with `SecretStoreError::BlankPassphrase`
  — a blank passphrase derives a key from a public salt only, i.e.
  obfuscation, not confidentiality. This is an **intended behavioural
  break** for any caller that relied on `SecretString::empty()`. A
  deliberate keyless vault uses the explicit
  `EncryptedFileStore::open_unprotected(path)` /
  `SecretStore::file_unprotected(path)` door instead (use it only where the
  stored secrets carry their own Tier-2 object password, or as a staging
  step before `rekey` to a real passphrase — the empty→real migration).
- **OS keyring (`SecretStore::os` / `default_credential_store`)** —
  returns an `Arc<dyn CredentialStoreApi + Send + Sync>` over the
  platform's default credential store. The backend on Linux/FreeBSD is
  `dbus-secret-service-keyring-store`; on macOS
  `apple-native-keyring-store`; on Windows
  `windows-native-keyring-store`. Fail-closed with
  `keyring_core::Error::NoDefaultStore` on headless / unknown OS
  — never a silent plaintext fallback. Through
  `SecretStore`, keyring failures project to
  `SecretStoreError::OsKeyring { kind }`, a non-secret discriminant.

  **Headless caveat (Linux/FreeBSD).** Secret Service requires a D-Bus
  session and an unlocked collection; headless / SSH / CI hosts
  frequently lack it, in which case `SecretStore::os()` fails closed
  with `NoDefaultStore`. Callers that need durable storage on a
  headless host should pin `SecretStore::file(...)` (encrypted-file
  vault) instead of relying on the OS keyring.

  **Enumerable metadata (OS arm).** Each entry is keyed by
  `service = SERVICE_PREFIX + hex(wallet_id)` and `user = label`, stored
  as **plaintext, enumerable** keyring metadata: same-user list-only
  tooling can see which wallet ids exist and which slot kinds (labels)
  each has, without unlocking any secret. This is dominated by the
  already-accepted same-user (A2/A3) residual. The `keyring-core` 1.0.0
  `build` modifiers are vendor-specific creation hints, not a replacement
  for the `(service, user)` identity, so there is no portable knob to
  redact the pair; operators who need metadata hiding should use the file
  vault, whose `(wallet_id, label)` map lives only inside the sealed
  vault. Prefer non-descriptive labels on the OS arm regardless.
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
is **lossless**: `WrongPassphrase`, `Corruption`, `AlreadyLocked`,
`KdfFailure`, `VersionUnsupported`, `MalformedVault`, `InsecurePermissions`,
`InsecureParentDir`, `SecretTooLarge`, `VaultTooLarge`, `Encrypt`, and
`InvalidLabel` are distinct typed variants. The Tier-2 layer adds five more:
`ExpectedProtectedButUnsealed` (the fail-closed strip refusal),
`NeedsPassword` (a protected object read with no password), `WrongPassword`
(object-password tag fail — distinct from the Tier-1 `WrongPassphrase`),
`BlankPassphrase` (a blank vault passphrase or object password), and
`UnsupportedEnvelopeVersion { found }` (a future envelope format, fail
closed regardless of the password). The four Tier-2 credential/protection
*state* variants project to a recoverable `NoStorageAccess` (boxed,
downcast-recoverable, like `WrongPassphrase`); `UnsupportedEnvelopeVersion`
joins the secret-free `BadStoreFormat` group. `VaultTooLarge` surfaces when
the on-disk vault exceeds the read-side ceiling; `SecretTooLarge` rejects an
oversized secret at the write boundary before it can inflate the shared
vault; `InsecureParentDir` refuses a vault whose parent directory is
group/other-writable (a writable parent governs rename/replace despite the
file's own `0600`); `Encrypt` is the (effectively unreachable) AEAD
encrypt-side failure, kept typed so a write failure is never mislabeled a
key-derivation error. For the OS arm,
`keyring_core::Error` projects best-effort into
`SecretStoreError::OsKeyring { kind: OsKeyringErrorKind }`, a payload-free
discriminant — keyring variants carrying raw bytes (`BadEncoding`,
`BadDataFormat`) are collapsed so their bytes never enter the error
(CWE-209/CWE-532).

**`WrongPassword` on the OS arm is ambiguous.** A Tier-2 envelope AEAD tag
failure surfaces as `WrongPassword`, but on the OS-keyring arm the stored
item is the bare envelope with no second authentication layer, so a tag
failure can mean EITHER a wrong object password OR a corrupted keychain
item — one AEAD tag cannot disambiguate the two. Treat `WrongPassword` on
the OS arm as "wrong password or corrupted item." On the file arm it is
unambiguous: the vault's own per-entry tag has already authenticated the
stored bytes before the envelope is parsed.

The internal SPI projection `From<SecretStoreError> for
keyring_core::Error` keeps the `WrongPassphrase` / `AlreadyLocked` variants
recoverable: they ride in `NoStorageAccess` with the typed
`SecretStoreError` boxed as the source, so an SPI-only consumer can recover
them via `err.source().and_then(|s| s.downcast_ref::<SecretStoreError>())`.
The `BadStoreFormat` group (`Corruption`, `KdfFailure`,
`VersionUnsupported`, `UnsupportedEnvelopeVersion`, `MalformedVault`,
`InsecurePermissions`, `InsecureParentDir`, `SecretTooLarge`,
`VaultTooLarge`, `Decrypt`, `Encrypt`, `OsKeyring`) has no box slot and
carries only a secret-free
string; those remain fully typed on the `SecretStore` path (so e.g.
`VaultTooLarge` / `SecretTooLarge` are not losslessly recoverable through
the SPI downcast).

`keyring_core::Error` is safe to `Display` (`{ }`-format), but
`{:?}`-format embeds `BadEncoding(Vec<u8>)` / `BadDataFormat(Vec<u8>, _)`
payloads — those variants are NEVER constructed by our backends with
secret bytes, and `tests/secrets_guard.rs` enforces that no debug-format
pairs with `keyring_core::Error` inside `src/secrets/`.

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
  `expose_secret(...)` on the same logical statement, AND forbids
  `{:?}`-debug-format paired with `keyring_core::Error`.
- **`tests/secrets_api.rs`**: shape guards — `CredentialApi::get_secret`
  re-wraps through `SecretBytes::new`, redacting `Debug` on
  `SecretBytes`/`SecretString`, no `Box<dyn Error>` in `src/secrets/`.
- **`tests/secrets_default_on_compiles.rs`**: build-time guard
  (gated `#![cfg(feature = "secrets")]`) that the default feature set
  exposes the secrets surface as public re-exports. It names
  `EncryptedFileStore`, `SecretBytes`, `SecretString`,
  `SecretStoreError`, `WalletId`, `SERVICE_PREFIX`, and
  `default_credential_store` from the crate root; the body never
  exercises a backend, so the proof is that it compiles. The negative
  direction — `--no-default-features --features sqlite,cli` must build
  the persister without the `secrets` module — is enforced by the
  feature gate plus the CI off-state build, not by a test file.
- **`tests/sqlite_persist_roundtrip.rs::tc082_no_box_dyn_error_in_src`**:
  all public method signatures use concrete error types
  (`WalletStorageError`, `PersistenceError`) — never
  `Box<dyn Error>` — so a future leak is caught by `grep`.

The CI advisory check runs `rustsec/audit-check` over `Cargo.lock`;
because `secrets` is in the default feature set, the pinned
`argon2` / `chacha20poly1305` / `zeroize` / `subtle` / `getrandom`
(the `OsRng` source for the salt + per-entry nonces, specified as the
exact pin `getrandom = "=0.2.17"`) / `region` / `keyring-core` /
per-platform store crate versions are unconditionally in the lockfile
and therefore unconditionally in audit scope.

## Backup retention and secrets

Manual / auto backups are byte-for-byte copies of the live DB. They
inherit the same "no secrets in the file" invariant. Operators may
still want to encrypt backups at rest using a file-system level tool
(GnuPG, age, encfs); this crate does not do that for them and never
ships SQLCipher.

## Future work — maintenance CLI

A unified `platform-wallet-storage secrets <subcommand>` CLI is planned as a follow-up to give operators a way to inspect and manage the secret backends without writing custom code; it is tracked as a separate follow-up work item. Two commands matter:

- **`secrets probe`** — set/get/delete a `__probe__` entry under `SERVICE_PREFIX`. Works uniformly on **all** backends (Secret Service, macOS Keychain, Windows Credential Manager) because it only uses single-entry CRUD. Confirms backend liveness + write-path responsiveness — the canary command for "is the keyring actually wired up on this machine?". Cheap to implement (~30 lines).
- **`secrets list [--filter <prefix>]`** — enumerate `(wallet_id, label)` pairs in the store. Trivial on the file vault (iterate the in-memory `BTreeMap`). On the OS arm: works on Secret Service, macOS Keychain, and Windows Credential Manager via `CredentialStoreApi::search`. Operators on headless Linux without a Secret Service session must select the file vault explicitly.

Other planned subcommands: `secrets put <svc> <label> <hex|@file>`, `secrets delete <svc> <label>`, `secrets rekey <new-passphrase>` (file-vault only). `secrets get` is deliberately omitted (printing a secret to stdout defeats `SecretBytes` zeroize); if added, must require an explicit `--unsafe-print-secret` flag.
