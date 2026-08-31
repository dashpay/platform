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

let store = SecretStore::file(
    "/var/lib/wallet/secrets.pwsvault",
    SecretString::new("correct-horse-battery-staple"),
)?;
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

The canonical wire format is **bincode-encoded** under a single
`WIRE_CONFIG = standard().with_big_endian().with_no_limit()` against
two `pub(crate)` types whose shapes are the source of truth — see
[`src/secrets/wire/envelope.rs`](src/secrets/wire/envelope.rs) and
[`src/secrets/wire/mod.rs`](src/secrets/wire/mod.rs):

```rust
struct Envelope { version: u32, payload: Payload }
enum Payload {
    Unprotected(Vec<u8>),                                            // scheme 0
    Password {                                                       // scheme 1
        kdf: KdfParamsEncoded, // id u8 ‖ m_kib u32 ‖ t u32 ‖ p u32
        salt: [u8; 32], nonce: [u8; 24],
        ciphertext: Vec<u8>,   // includes the 16-byte Poly1305 tag
    },
}
```

`ENVELOPE_VERSION = 1` is bumped only on a breaking layout change,
independent of the vault `FORMAT_VERSION`. Decoding goes through a
budget-limited `DECODE_CONFIG = WIRE_CONFIG.with_limit::<N>()` so a
hostile blob declaring a multi-GiB length prefix is rejected before
allocation (security-positive deviation from the no-limit encoder
config). Trailing bytes after a valid decode are also refused —
`consumed == blob.len()` is a fail-closed invariant.

- **AAD (scheme 1)** is bincode-encoded from `Tier2Aad`
  ([`src/secrets/wire/aad.rs`](src/secrets/wire/aad.rs)), which binds
  `domain (PWSEV-TIER2-AAD-v2) ‖ envelope_version ‖ scheme_discriminant
  ‖ kdf ‖ salt ‖ wallet_id ‖ label`. The vault's own per-entry AAD goes
  through `EntryAad` (`domain (PWSV-ENTRY-AAD-v2) ‖ format_version ‖
  wallet_id ‖ label`) and the vault verify-token AAD through `VerifyAad`
  (`domain (PWSV-VERIFY-AAD-v2) ‖ format_version ‖ salt ‖ kdf`). All
  three domain tags are pair-wise byte-disjoint by construction. A
  protected blob relocated to another slot — or any in-place header
  edit — fails the tag (relocation/header-tamper resistance). On the
  file arm this AAD is *in addition* to the vault's own per-entry AAD
  + tag; on the OS arm it is the only authentication layer.
- **KDF ceiling before derivation (anti-DoS).** The KDF params live in
  the (attacker-controllable) header, so on a read the Argon2 ceiling
  is enforced **before** any derivation/allocation — both the wider
  `enforce_bounds` (algorithm id + floors/ceilings) AND a tighter
  per-read gate that refuses any `m_kib > default_target().m_kib` OR
  `t > default_target().t`. A forged header cannot inflate memory by
  more than the shipped default or CPU by more than the shipped
  iteration count.
- **No vault format bump.** The envelope lives *inside* the entry
  bytes, identical over File and Os, so there is no vault-parser or
  migration change.
- **Size cap.** The plaintext is capped at `MAX_PLAINTEXT_LEN`
  (`MAX_SECRET_LEN − MAX_ENVELOPE_OVERHEAD`), uniformly for both
  schemes, so the enveloped bytes always fit the backend's own
  `MAX_SECRET_LEN` cap and the user-visible limit is stable regardless
  of scheme. Oversize → `SecretTooLarge { found, max }` with
  `max = MAX_PLAINTEXT_LEN` (re-exported as `secrets::MAX_PLAINTEXT_LEN`).
- **Unknown envelope version** → `UnsupportedEnvelopeVersion` — fail
  closed **regardless of the password**: an envelope tagged for a
  future layout can be neither safely unwrapped nor treated as
  unprotected.
- **Unparseable bytes / unknown scheme tag / trailing garbage** →
  `Corruption`. There is no magic-byte peek — every blob runs through
  the bincode decoder, and anything that does not round-trip cleanly
  with `consumed == blob.len()` fails closed.

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
| **`Some(pw)`** | **valid scheme-0 envelope** | **`ExpectedProtectedButUnsealed` — FAIL CLOSED** |
| `Some(pw)` | scheme-1 but truncated/corrupt | `Corruption` |
| `Some/None` | unknown envelope version | `UnsupportedEnvelopeVersion` |
| `Some/None` | unparseable / non-envelope bytes / trailing garbage | `Corruption` |
| `None` | valid scheme-1 | `NeedsPassword` (never ciphertext) |
| `None` | valid scheme-0 envelope | the secret |
| any | absent entry | `Ok(None)` (deletion = DoS, never injection) |

The load-bearing row is **`Some(pw)` + scheme-0 envelope ⇒
`ExpectedProtectedButUnsealed`**: with a password in hand, an
unprotected envelope can only mean a strip, so it is refused and **no
bytes are returned**. A consumer bug alone — over- or under-supplying
a password — fails closed in *every* direction.

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
`Some`→`Some` changes, `Some`→`None` removes. An absent object returns
`Err(SecretStoreError::NoEntry)` — `reprotect` is operational, so absence
means the caller's protection-status record disagrees with the backend and
must not be silently dropped. The rewrite is a same-slot overwrite — atomic on the file arm,
and on the OS arm inheriting the backend's single-item-replace contract —
so a crash between the read and the commit leaves the prior value intact
and readable under `current`. **After a successful call the consumer MUST
update its own protection-status record** (the protection expectation lives
there). There is **no password recovery** — losing an object password
bricks that object (an availability trade-off the UX must state plainly).

#### Entropy policy is the consumer's

The library enforces an 8-byte post-trim `MIN_PASSPHRASE_LEN` floor for both
the vault passphrase and the Tier-2 object password. It ships **no**
password-strength estimator: real entropy policy (zxcvbn-style strength,
dictionary checks, UX feedback) is locale- and threat-specific and is the
**consumer's responsibility**. For a protected object the password's
entropy is the *whole* guarantee against an offline Argon2id attacker who
already holds the backend — choose it accordingly.

#### Greenfield only — no legacy tolerance

The envelope is the only on-disk Tier-2 format this build understands.
A decrypted entry that does not bincode-decode to a valid `Envelope`
under `WIRE_CONFIG` (including trailing-byte extension probes) surfaces
as `Corruption` on every read — there is no magic-byte peek and no
magic-less raw legacy path. The shipped wire layer is the source of
truth; older non-enveloped stored values are out of scope.

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
intermediate `Vec` binding.

`SecretBytes::new` takes the `Vec<u8>` by value, **copies** it into
guarded memory, then zeroizes the source before it drops. The copy is
unavoidable and load-bearing: guarded memory comes from a dedicated
allocator, so a `Vec`'s own allocation can never *become* the protected
buffer — wiping the original is the only thing that keeps an
unprotected duplicate off the general-purpose heap. `SecretString::new`
does the same for a moved-in `String`; `From<&str>` and the
`secret-serde` `visit_str` bypass the intermediate allocation entirely.
`Debug` is redacted on both.

`SecretString` is additionally **editable in place**, through the single
`replace_range(range, replacement)` primitive (insertion is an empty
range, deletion an empty replacement, wholesale replacement `..`) — it
backs live text-input widgets downstream without them keeping a
duplicate guarded buffer of their own. No plaintext leaves the wrapper
through it: an edit that outgrows the buffer allocates a fresh guarded
one, copies through a safe slice, and lets the outgrown one wipe itself
on drop; a shrinking edit wipes the bytes it vacates. An invalid range
panics (matching `String::replace_range`) with a message naming **only
indices** — never content, since `str`'s own slicing panic would print
the surrounding plaintext (CWE-209/CWE-532). The buffer is deliberately
**uncapped** here: a value type cannot report a refusal, so enforcement
stays at the UI that accepts the input and at the vault write, which
applies `MAX_PLAINTEXT_LEN`.

**Every secret owns its own guarded pages.** The buffer comes from
`memsec`'s hardened allocator (`src/secrets/guarded.rs`, the crate's
only `unsafe`): page-aligned, fenced by inaccessible `PROT_NONE` guard
pages, canary-checked, `mlock`ed, and excluded from core dumps
(`MADV_DONTDUMP` on Linux). Because the data pages belong to one buffer
outright, **no two live secrets ever share a page**, so freeing one can
never unlock memory another still holds — the failure mode that makes
page-granular locking hazardous over ordinary allocations. The wipe
covers the buffer's full capacity, not just the live length.

The `mlock` remains **best-effort / fail-open**: if the kernel refuses
the lock the secret is still allocated, guard-paged and wiped, merely
swappable. That refusal is logged at `warn` (with no address, length or
content), so a degraded lock is observable rather than silent. An
opt-in fail-closed strict mode is not implemented.

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
  Each secret is capped at `MAX_SECRET_LEN` (8176 B = `2 * 4096 - 16`)
  at the write boundary — still ~30× any mnemonic/seed/xpriv — so a
  single oversized entry cannot inflate the shared document past the
  read-side 128 MiB ceiling and brick every wallet on the next open.
  The value is set by locked memory, not by the document: secrets live
  in `mlock`ed pages, and 8176 is the largest secret whose stored
  envelope still fits two guarded pages instead of a third. The full
  budget — which path peaks, at what, against a 64 KiB
  `RLIMIT_MEMLOCK` — is documented at the constant and measured by
  `store::tests::file_reprotect_peak_matches_the_documented_budget`. (Through
  `SecretStore::set_secret`/`set` the user-facing plaintext cap is the
  slightly lower `MAX_PLAINTEXT_LEN`, leaving room for the envelope
  overhead; see **Two-tier secret protection**.)
  **Short passphrases are rejected.** `open` (and `rekey`) require at least
  8 bytes after trimming and return `SecretStoreError::BlankPassphrase` for a
  shorter input. A
  deliberate keyless vault uses the explicit
  `EncryptedFileStore::open_unprotected(path)` /
  `SecretStore::file_unprotected(path)` door instead (use it only where the
  stored secrets carry their own Tier-2 object password, or as a staging
  step before `rekey` to a real passphrase — the empty→real migration).
  **Over-long passphrases are rejected too.** `open`/`rekey` and both
  sides of the Tier-2 object-password path refuse anything past
  `MAX_PASSPHRASE_LEN` (4080 B, one guarded page) with
  `SecretStoreError::PassphraseTooLong`. This is a memory bound, not a
  policy one: a passphrase stays resident in `mlock`ed pages for its
  store's whole lifetime, and three are live at once during a
  `reprotect`, so an unbounded one would break the locked-memory budget
  above. The `secret-serde` `Deserialize` impl applies the same ceiling,
  since config is the one construction path whose size this crate does
  not control.
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

#### Tests

Ordinary integration tests use `EncryptedFileStore::open` or
`SecretStore::file` and therefore exercise the production Argon2id target.
Downstream suites that would otherwise pay that cost throughout an end-to-end
flow may enable the dev-only `test-util` feature and use
`EncryptedFileStore::open_mock` or `SecretStore::file_mock`. For a fresh vault,
those constructors select the floor Argon2id parameters; an existing vault
retains the parameters recorded in its header. Per-object wrapping also uses
the floor. `KdfParams::floor_target` is the single choke point for selecting
these weak-but-legal parameters. Accidental production use is blocked twice:
the constructors are compiled only for tests or with `test-util`, and
`KdfParams::floor_target` panics outside debug builds and this crate's own test
harness.

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
`BlankPassphrase` (a blank or sub-floor vault passphrase or object password), and
`UnsupportedEnvelopeVersion { found }` (a future envelope format, fail
closed regardless of the password). The four Tier-2 credential/protection
*state* variants project to a recoverable `NoStorageAccess` (boxed,
downcast-recoverable, like `WrongPassphrase`); `UnsupportedEnvelopeVersion`
joins the secret-free `BadStoreFormat` group. `VaultTooLarge` surfaces when
the on-disk vault exceeds the read-side ceiling; `SecretTooLarge` rejects an
oversized secret at the write boundary before it can inflate the shared
vault; `InsecureParentDir` refuses a vault whose ancestor chain has unsafe
ownership or a group/other-writable component without the sticky bit (an
attacker who can replace an ancestor can replace the `0600` file); `Encrypt`
is the (effectively unreachable) AEAD
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

**`WrongPassphrase` on the file arm is ambiguous at the vault header.** The
Tier-1 header's verification token has no integrity check independent of the
passphrase-derived key. Its AEAD tag therefore cannot distinguish an incorrect
vault passphrase from corruption of the header salt, KDF parameters, nonce, or
ciphertext. Treat file-arm `WrongPassphrase` as "wrong passphrase or corrupted
header." This ambiguity is limited to the Tier-1 header; after the header is
verified, the vault's per-entry authentication keeps Tier-2 `WrongPassword`
unambiguous on the file arm as described above.

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
exact pin `getrandom = "=0.2.17"`) / `memsec` / `keyring-core` /
per-platform store crate versions are unconditionally in the lockfile
and therefore unconditionally in audit scope. `memsec` (exact pin
`=0.7.0`) deserves the closest reading of the set: it performs every
page lock, every guard-page `mprotect`, and backs the crate's only
`unsafe`, all inside `src/secrets/guarded.rs`. `region` is a
**dev-dependency only** — the page-size query for the page-isolation
tests — and is not in the production dependency graph.

## Integration constraints

Guarded allocation is not free, and it constrains what a consuming
binary may do. Three consequences, none of them visible from the public
API:

- **Every non-empty secret costs at least one locked page** (4 KiB),
  plus guard pages of address space, however small it is — a 32-byte
  AEAD key included. That is the price of the no-shared-page guarantee.
  Empty secrets are the one case optimised away: `SecretString::empty()`
  and an empty `SecretBytes` hold no allocation at all. Budget one page
  per live secret and check `RLIMIT_MEMLOCK` against it; if the limit is
  too low the locks fail open (see "Memory hygiene at the seam") and a
  `warn` is logged per affected allocation.
- **No custom global allocator.** `memsec` takes its pages from the Rust
  global allocator and `mprotect`s them in place. A binary installing
  `mimalloc`/`jemalloc`/`snmalloc` may hand it pages whose allocator
  metadata sits inside the protected block; the failure mode is a
  segfault or silently ineffective guard pages, on the secret path.
- **No Miri, ASan, LSan or libFuzzer over this crate.** Miri cannot
  execute the `mprotect`/`mlock` FFI, and the sanitizers segfault on
  memsec's guard pages (memsec issue #14). A sanitizer or fuzz job must
  build without the `secrets` feature. The `unsafe` this forecloses
  verification of is confined to `src/secrets/guarded.rs` and is small
  enough to review by inspection — which is now the only line of
  defence, and the reason it stays that small.

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
