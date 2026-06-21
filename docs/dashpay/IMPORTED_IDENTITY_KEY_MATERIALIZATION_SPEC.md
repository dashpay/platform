# Imported-Identity Key Materialization — carry the derived scalar

Status: draft for review
Scope: fixes the second, on-device-blocking defect of "imported identity cannot
sign" (the first — discovery emitting a breadcrumb only for the master key — is
already fixed; see `IMPORTED_IDENTITY_SIGNING_SPEC.md`).

## 1. Problem

On a freshly-imported wallet the discovered identities' keys are all persisted
**watch-only**, so the identity cannot sign any state transition
("No PersistentPublicKey row matches the supplied public-key bytes").

On-device diagnosis (testnet, SwiftExampleApp) proved the chain end-to-end:

- Rust discovery derives + verifies every key and emits a breadcrumb for each
  (`candidates_derived=5 breadcrumbed=5`, indices correct).
- The breadcrumb reaches Swift `persistIdentityKeys` with the correct
  `(identity_index, key_index)`.
- `deriveAndStoreIdentityKey` then returns `nil` for **every** key with
  `⚠️ mnemonic missing … Mnemonic not found`.

Root cause: `deriveAndStoreIdentityKey` **re-derives** the scalar by reading the
wallet's BIP-39 mnemonic back out of `WalletStorage` (iOS Keychain) and running
`mnemonic → seed → path → key`. During import, identity discovery (which
materializes keys) runs **before** `CreateWalletView.createWallet` persists the
mnemonic to `WalletStorage`, so the read fails and the key is dropped to
watch-only. Even outside that race the re-derive is fragile: a watch-only-loaded
wallet, a missing/biometric-gated mnemonic, or any keychain hiccup silently
yields watch-only keys.

This is precisely the anti-pattern `packages/swift-sdk/CLAUDE.md` forbids: Swift
must not "fetch the mnemonic from Keychain, hand it back to Rust, … and write
those to Keychain". The same doc states the sanctioned shape: "accept
`(path_string, 32_private_key_bytes)` from a Rust FFI call and write to
Keychain."

## 2. Chosen approach — carry the verified scalar to the client

Discovery already derives **and verifies** each candidate scalar
(`discovery::derive_key_breadcrumbs` → `breadcrumb_decisions`, gated on
`IdentityPublicKey::validate_private_key_bytes`). Today `breadcrumb_decisions`
**discards** that scalar and keeps only `(wallet_id, identity_index, key_index)`,
forcing the client to re-derive. Instead, carry the already-derived, already-
verified 32-byte scalar through the persister changeset to the client, which
stores the bytes directly — no mnemonic read, no re-derivation, no timing
dependency.

Why this over the alternatives:

- **Reorder mnemonic-store-before-discovery** (rejected as the primary fix): the
  network-scoped `walletId` is returned *by* `createWallet`, so storing the
  mnemonic first is a chicken-and-egg, and it leaves the fragile re-derive (and
  the anti-pattern) in place — every future caller that materializes a key still
  depends on a readable keychain mnemonic.
- **Re-run a from-0 discovery after storeMnemonic** (rejected): the default scan
  resumes past known indices, so it would not re-emit; forcing from-0 is hacky
  and still keeps the re-derive.

Precedent: `dash_sdk_derive_and_persist_identity_keys` already passes
`PersistKeyArgs.private_key_bytes: *const u8` (32 bytes) over the FFI to the
Swift persister, which writes them to Keychain. We extend the *changeset*
persister path (`IdentityKeysChangeSet` → `on_persist_identity_keys`) to carry
the same secret, reusing `KeychainManager.storeIdentityPrivateKey(_:derivationPath:metadata:)`.

## 3. Data flow & interface changes

Secret path (new), one secret per re-derivable key, `None` for watch-only:

```
discovery::breadcrumb_decisions      (already holds the verified Zeroizing<[u8;32]>)
  → IdentityKeyWithBreadcrumb         carry Zeroizing<[u8;32]> alongside the indices
  → ManagedIdentity::add_keys         move the secret into the changeset entry
  → IdentityKeyEntry                  + private_key: Option<Zeroizing<[u8;32]>>
  → IdentityKeysChangeSet (persister.store)
  → FFI persistence dispatch (persistence.rs)
  → IdentityKeyEntryFFI               + private_key_is_some: bool, private_key: [u8;32]
       (from_entry copies bytes; free_identity_key_entry_ffi zeroes them)
  → Swift persistIdentityKeysCallback build IdentityKeyEntrySnapshot.privateKey: Data?
  → persistIdentityKeys               store bytes directly (verify-then-store), no re-derive
```

Layer-by-layer:

1. **`changeset.rs`**
   - `KeyDerivationBreadcrumb` gains the scalar:
     `(wallet_id, identity_index, key_index, Zeroizing<[u8;32]>)`. (It already
     transits only in-process; not serialized in any persisted form that matters
     — see Failure modes #5.)
   - `IdentityKeyEntry` gains `pub private_key: Option<Zeroizing<[u8;32]>>`.
     `PartialEq`/`Clone` keep working (`Zeroizing` is both). The `serde`
     derives must **skip** `private_key` (`#[serde(skip)]`) so a secret never
     lands in any serialized changeset.
2. **`discovery::breadcrumb_decisions`**: on `reproduces`, clone the scalar out
   of `candidate_scalars` into the breadcrumb. The verify gate is unchanged and
   still load-bearing — only a scalar that reproduced the on-chain key is
   carried.
3. **`identity_ops.rs::add_keys`**: move the scalar from the breadcrumb into
   `IdentityKeyEntry.private_key`. `add_key` (single) delegates unchanged.
   `keys_snapshot_changeset` sets `private_key: None` (watch-only snapshot).
4. **`identity_persistence.rs::IdentityKeyEntryFFI`**: add
   `private_key_is_some: bool` + `private_key: [u8; 32]`. `from_entry` copies the
   bytes (or zero + false). `free_identity_key_entry_ffi` zeroes the 32 bytes.
   The struct is `#[repr(C)]`; field order documented in the byte-layout comment.
5. **`persistence.rs`** identity-keys dispatch: unchanged except it now hands the
   FFI struct (with the secret) to the callback; the existing
   `free_identity_key_entry_ffi` loop already runs after the callback returns and
   will zero the secret.
6. **Swift `persistIdentityKeysCallback`**: read `private_key_is_some` → copy the
   32 bytes into a `Data`, build `IdentityKeyEntrySnapshot.privateKey: Data?`.
7. **Swift `persistIdentityKeys`**: when `privateKey != nil`, verify-then-store:
   compute `platform_wallet_pubkey_hash_from_private_key(scalar)` and compare to
   `entry.publicKeyHash` (the §7.2 mirror-check, now applied to the carried
   bytes); on match call `storeIdentityPrivateKey(data, derivationPath:, metadata:)`
   and set `privateKeyKeychainIdentifier`; on mismatch leave watch-only. The
   `derivationPath` string is built in Swift from `(network, identity_index,
   key_index)` via the existing `KeyDerivation.getIdentityAuthenticationPath`
   (a pure string format for the keychain account label — not key derivation),
   matching the account shape `storeIdentityPrivateKey` already uses. Scrub the
   `Data` after storing. `deriveAndStoreIdentityKey` (the mnemonic-re-derive
   path) is **deleted** — no caller remains.

The non-secret breadcrumb (`derivation_indices`) is **retained** on
`IdentityKeyEntry`/the FFI/the snapshot: it still populates the keychain metadata
(identity/key index) and the explorer, and is the watch-only-vs-signable
discriminant for any consumer that doesn't want the bytes.

## 4. Security

- **Bytes over the FFI are sanctioned** for the Keychain-write exception
  (`swift-sdk/CLAUDE.md`) and already precedented (`PersistKeyArgs`). No *new*
  secret class crosses the boundary; the same scalar create-in-app already
  materializes.
- **Verify-before-store stays double-gated**: (a) Rust only carries a scalar that
  passed `validate_private_key_bytes` against the on-chain key; (b) Swift
  re-verifies the carried bytes' pubkey-hash equals the published hash before
  writing — a corrupted/mismatched transfer drops to watch-only, never stores a
  wrong key.
- **No secret at rest outside Keychain**: `IdentityKeyEntry.private_key` is
  `#[serde(skip)]`; SwiftData stores only `privateKeyKeychainIdentifier`
  (account string), never the bytes; the FFI buffer is zeroed in
  `free_identity_key_entry_ffi`; the Swift `Data` is `resetBytes` after store.
- **No secret logging**: no `tracing`/`print` of `private_key`/derived bytes;
  logs carry only `key_id` / public hashes (enforced in review).
- **Confused-deputy / cross-wallet**: unchanged — the carried scalar only exists
  because *this* wallet's seed reproduced *this* identity's key under the verify
  gate.

## 5. Failure modes

1. Scalar absent (watch-only / foreign / non-ECDSA): `private_key: None` →
   `private_key_is_some=false` → Swift leaves the key watch-only (correct, no
   regression).
2. Carried bytes fail the Swift mirror-check: drop to watch-only + warn (public
   hashes only). Loud, fail-safe.
3. Keychain write fails: `storeIdentityPrivateKey` returns `nil` → watch-only;
   next persister upsert retries.
4. Ordering race that caused this bug: **eliminated** — no mnemonic read on the
   materialization path.
5. Serialized changeset: `private_key` is `#[serde(skip)]`, so any
   serialize/deserialize round-trip yields `None` (degrades to watch-only, never
   leaks). Audit: confirm no production path persists `IdentityKeysChangeSet` and
   expects the secret to survive serde (the secret is an in-process,
   same-tick FFI hand-off only).

## 6. Migration

An already-imported (broken, all-watch-only) wallet heals on the next full
from-index-0 discovery (a wipe + re-import does this; the resident wallet path
needs no mnemonic). No persisted-data migration; the change is additive.

## 7. Test / verification plan

- **Rust unit** (`discovery.rs` tests): extend the existing
  `breadcrumb_decisions_*` tests to assert the breadcrumb now carries the
  verified scalar for reproducible keys and `None` for non-reproducible.
  `add_keys` tests assert `IdentityKeyEntry.private_key` is `Some(scalar)` for
  breadcrumbed keys, `None` otherwise.
- **FFI round-trip** (`identity_persistence.rs` tests): `from_entry` sets
  `private_key_is_some` + bytes; `free_identity_key_entry_ffi` zeroes them.
- **Serde guard**: a test that serializes an `IdentityKeyEntry` with a secret and
  asserts the secret is absent from the output / `None` after round-trip.
- **Swift**: unit-test the snapshot mapping (private_key_is_some → Data?) and the
  verify-then-store branch.
- **On-device** (the acceptance test): wipe → fresh import → confirm
  `ZPERSISTENTPUBLICKEY` rows flip to signable (keychain id set) → sign a state
  transition (set DashPay profile / register DPNS) as a discovered identity →
  success, no "No PersistentPublicKey row matches".
- **Red→green**: capture the pre-fix all-watch-only histogram and the post-fix
  signable histogram in the same store (within-store contrast).

## 8. Out of scope

- The first defect (master-only breadcrumb) — already fixed.
- Reworking `CreateWalletView`'s create/store ordering (no longer needed once the
  materialization path is mnemonic-independent).
- Android/other clients (the changeset/FFI carries the secret generically; only
  the iOS persister is wired here).

## 9. Review outcomes — must-fixes folded in

Four independent reviews (blockchain-security, feasibility, scope, adversarial)
ran against §1–8. Consolidated must-fixes (these REVISE the sections above):

### Correctness — the two ways the fix silently produces watch-only keys

- **MF-A (HASH160 verify is arithmetically wrong).** `entry.publicKeyHash` =
  `ripemd160_sha256(pub_key.data())` (`identity_ops.rs::pubkey_hash_of`). For an
  `ECDSA_HASH160` key `data()` is *already* the 20-byte hash, so the field is
  `hash160(hash160(pubkey))` — a double hash — while
  `platform_wallet_pubkey_hash_from_private_key` returns single `hash160(pubkey)`.
  They never match → a HASH160 key would always drop to watch-only (a regression:
  the current `deriveAndStoreIdentityKey` stores it because it gates the hash
  compare on `keyType == ecdsaSecp256k1`). **Fix:** the new verify-then-store
  branch keeps that exact gate — re-verify only `ECDSA_SECP256K1`; for any other
  carried type store directly and rely on the Rust `validate_private_key_bytes`
  gate (which is type-correct). Document that the Swift mirror-check is
  ECDSA-SECP256K1-only and that a future non-ECDSA carry must grow a per-type
  branch.

- **MF-B (clear-after-set race).** `persistIdentityKeys`'s no-secret branch sets
  `privateKeyKeychainIdentifier = nil` unconditionally. In `discover_inner` the
  order is `add_identity` (watch-only snapshot of *all* keys) → `add_keys`
  (secret-bearing) — so the secret currently wins only by emit order, and a
  watch-only snapshot of a key cannot be told apart on the wire from a genuinely
  watch-only key. **Fix:** the Swift no-secret branch must **preserve** an
  existing `privateKeyKeychainIdentifier` rather than nil it — a key that was
  materialized stays materialized (the Keychain item is durable; a genuinely
  watch-only key never had an id to preserve). This makes materialization
  order-INDEPENDENT. (The "drop the snapshot from `add_identity`" alternative was
  rejected: `add_identity` is shared by flows that do NOT follow with `add_keys`
  — `platform_wallet.rs:791`, `register_from_addresses.rs:131`,
  `payments.rs:885/950` — so its `keys_snapshot_changeset` is load-bearing there
  and can't be removed wholesale.) **Test:** emit a watch-only snapshot *after*
  the secret upsert and assert the key stays signable.

### Security — secret hygiene (revises §4)

- **MF-C (volatile zeroize, by-value precedent).** Embed the secret by value
  (`private_key: [u8; 32]` + `private_key_is_some: bool`) mirroring the existing
  `IdentityKeyPreviewFFI` (`derive_identity_key_at_slot.rs:241`), NOT the
  `PersistKeyArgs` pointer. `free_identity_key_entry_ffi` MUST scrub via
  `zeroize::Zeroize::zeroize(&mut entry.private_key)` (volatile — the codebase
  already replaced non-volatile `*byte = 0` scrubs for exactly this reason; see
  `identity_keys_from_mnemonic.rs:53` / `zeroize_and_free_row`), and scrub
  **unconditionally** (even when `private_key_is_some == false`, the 32 bytes are
  still present). The post-callback `free_identity_key_entry_ffi` loop in
  `persistence.rs:912-914` then covers the `Vec<IdentityKeyEntryFFI>` copy.
- **MF-D (strip secret from the `pending` copy).** `FFIPersister::store` clones
  the whole changeset into `self.pending` (`persistence.rs:1506-1511`) — a
  long-lived, un-zeroized second copy of the scalar. `pending` is never replayed
  to the callbacks (only `flush` consumes it). **Fix:** do not carry
  `private_key` into the `pending` accumulator — strip/replace it with `None`
  before the `merge`/insert (the secret is only needed for the immediate
  synchronous callback dispatch). This keeps the "no secret at rest outside
  Keychain" guarantee true; update §4/§5's "same-tick only" wording accordingly.
- **MF-E (Debug leak).** `Zeroizing<Z>`'s derived `Debug` prints the inner bytes
  (it is a tuple-struct derive, not redacting). `IdentityKeyEntry` derives
  `Debug` and rides inside `IdentityKeysChangeSet`/`PlatformWalletChangeSet`,
  which ARE logged on persist errors. **Fix:** hand-write `Debug` for
  `IdentityKeyEntry` redacting `private_key` (or wrap the scalar in a newtype with
  a redacting `Debug`); unit-test that `format!("{:?}", entry)` contains no secret.
- **MF-F (scrub every Swift copy).** The intermediate tuple decode
  (`var t = e.private_key; Data(...)`) is an un-scrubbed stack copy (the existing
  by-value precedent at `ManagedPlatformWallet.swift:956` never scrubs it).
  Scrub the intermediate tuple immediately after the `Data` copy, and
  `resetBytes` `IdentityKeyEntrySnapshot.privateKey` for EVERY entry at the end of
  `persistIdentityKeys` — including the watch-only-skip and mismatch-drop
  branches, not only the stored one.

### Feasibility / scope (revises §3)

- **MF-G (don't widen `KeyDerivationBreadcrumb`).** Keep
  `KeyDerivationBreadcrumb = ([u8;32], u32, u32)` (a shared navigation token).
  Carry the scalar by replacing the `IdentityKeyWithBreadcrumb` tuple with a
  named struct, e.g. `KeyWithBreadcrumb { key: IdentityPublicKey, breadcrumb:
  Option<KeyDerivationBreadcrumb>, verified_scalar: Option<Zeroizing<[u8;32]>> }`.
  `breadcrumb_decisions` produces it; `add_keys` consumes it.
- **MF-H (enumerate ALL construction sites).** Adding the non-defaulted
  `IdentityKeyEntry.private_key` breaks every struct literal — beyond the two the
  spec named: `rs-platform-wallet-storage/src/sqlite/schema/identity_keys.rs:70`
  (`into_entry`, production), `identity_persistence.rs` FFI tests
  (~1069/1114/1147/1179), and storage tests (`sqlite_structural_hardening.rs:310`,
  `sqlite_persist_roundtrip.rs:225`). All get `private_key: None`. The
  `rs-platform-wallet-storage` crate was missing from the §3 scope list — add it.
- **MF-I (serde + on-disk audit).** `#[serde(skip)]` on `private_key` is required
  for **compilation** (`Zeroizing` impls neither `Serialize` nor `Deserialize`).
  The one production serde-ish persister, `IdentityKeyWire`
  (`rs-platform-wallet-storage/.../identity_keys.rs:34-79`), is secret-free *by
  construction* (field-selective transcription, never reads `private_key`). Add a
  regression asserting `IdentityKeyWire` has no secret field so a future
  "serialize `IdentityKeyEntry` straight to the blob" refactor can't start
  persisting it.
- **MF-J (FFI layout guard + no hand Swift mirror).** Recompute the
  `const _: [u8; 184] = [0u8; size_of::<IdentityKeyEntryFFI>()]` guard
  (`identity_persistence.rs:335`) and the byte-offset comment; place
  `private_key_is_some` + `private_key` **last** to minimize padding churn. There
  is NO hand-maintained Swift mirror struct — cbindgen regenerates the header at
  build (`build.rs`), so Swift auto-sees the fields after `build_ios.sh`; the
  Rust comment claiming a "Swift mirror in PlatformWalletFFI.swift" is stale —
  correct it. Only `persistIdentityKeysCallback` needs updating to read the new
  fields.

### Confirmed sound / no change

- The Rust verify gate (`validate_private_key_bytes`) is type-correct and
  load-bearing; only a scalar that reproduces the on-chain key is ever carried.
  No path stores a wrong key and signs with it.
- `deriveAndStoreIdentityKey` has exactly one caller (`persistIdentityKeys`) —
  safe to delete. `retrieveMnemonicUTF8Bytes` / `Mnemonic.toSeed` stay used by
  `MnemonicResolverAndPersister.swift` (resolver path) — no dead-import fallout.
- Keeping BOTH `derivation_indices` (label/explorer/discriminant for watch-only
  wallets) and `private_key` (the secret) is justified — neither is redundant.
- Keychain account `identity_privkey.<walletId>.<path>` is unchanged; build the
  path label from `entry.walletId ?? scopeWalletId` + the carried indices via the
  mnemonic-free FFI path-formatter `getIdentityAuthenticationPath`.
- §6 heal requires an explicit from-index-0 rescan (`start_index: Some(0)`) or
  wipe+reimport — a default resident `sync()` resumes past known indices and will
  NOT re-materialize an already-known-but-watch-only identity. The durable
  signability marker across restarts is the `privateKeyKeychainIdentifier`
  column (the Keychain item persists), so a healed wallet stays healed.

## 10. Implementation review — fixes folded in

A second four-agent panel (blockchain-security, swift-ios, adversarial, rust-quality)
reviewed the implemented diff. All ten spec must-fixes verified correctly applied;
the Rust side passed clean. Additional fixes applied from this round:

- **IR-1 (must-fix, secret hygiene).** The end-of-loop `resetBytes` scrub in
  `persistIdentityKeys` was defeated by copy-on-write: the C-shim's `upserts`
  array still referenced the buffers, so the subscript mutation zeroed a CoW
  fork and left the scalar handed to the Keychain un-wiped in freed heap.
  Fixed: `persistIdentityKeys` is now strictly read-only over `upserts`, and the
  scrub moved to `persistIdentityKeysCallback` AFTER the call returns, where that
  array is the sole owner — an in-place wipe of the actual bytes.
- **IR-2 (should-fix, registration regression).** Deleting `deriveAndStoreIdentityKey`
  removed the only setter of `PersistentPublicKey.privateKeyKeychainIdentifier`
  for self-registered (not imported) identities (signing still worked via the
  keychain pubkey-hex fallback scan, but the `hasPrivateKey` UI marker + fast
  path regressed). Fixed: when a key carries a breadcrumb but no scalar (the
  registration case, whose keychain item is written by its own path), Swift
  adopts the existing keychain account via a public-key-hex lookup
  (`KeychainManager.identityPrivateKeyAccount`) — no derivation, no secret loaded.
- **IR-3 (low, footgun).** Coupled the carried scalar to the breadcrumb in
  `add_keys` so a `verified_scalar`-without-breadcrumb is dropped (can't reach
  the client without the indices it needs); pinned by
  `add_keys_drops_scalar_without_breadcrumb`.
- **IR-4 (nit).** `unwrap()` → `expect()` in the new utils test.

Confirmed correct, no change: volatile zeroize, `pending`-strip, redacting Debug,
`#[serde(skip)]`, layout guard, the ECDSA-only verify gate (HASH160 relies on the
Rust gate). On-device re-verified after these fixes: clean import → 23/23 signable.
