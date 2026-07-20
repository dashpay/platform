# Identity-Key Scalar Elimination — derive-sign-destroy for discovered keys

Status: draft, rev 2 (review must-fixes folded)
Scope: removes the carried 32-byte ECDSA scalar
(`IdentityKeyEntry.private_key` / `KeyWithBreadcrumb.verified_scalar`) from the
identity-key discovery → persist → sign flow, replacing it with a
derive-sign-destroy model in which the per-key secret only ever exists in the iOS
Keychain (as the wallet seed), is derived on demand at sign time, and is never
carried across the Rust→FFI→Swift boundary or stored per-key.

Supersedes the earlier carried-scalar storage posture (the carry-the-verified-scalar
fix this spec reverses). Aligns with the seed-elimination §4.9-blocker item 3 design
and the sibling decision to stop persisting the DashPay friendship xpub and re-derive
on load.

> **Review outcome (rev 2).** Four independent reviewers (feasibility, scope,
> security, crypto/domain) audited rev 1 against the code. The crux correctness
> claim — pubkey-compare is byte-for-byte equivalent to
> `validate_private_key_bytes(scalar)` — was **verified exact** for both
> `ECDSA_SECP256K1` (33-byte compressed compare) and `ECDSA_HASH160`
> (`ripemd160_sha256(pubkey)` vs `key.data()`; the double-hash gotcha does **not**
> apply because we compare `key.data()`, not `entry.public_key_hash`). No key is
> wrongly authorized and no wallet-derivable key is wrongly dropped to watch-only.
> The must-fixes folded below are all about the **migration**, where removing the
> carried scalar trades an *intrinsic* scalar↔pubkey binding for a *trusted-path*
> binding — the real lockout surface. The five load-bearing corrections:
> **(MF-1)** the backfill reads the Keychain **metadata blob** (named fields), not
> a parse of the account-string label; **(MF-2)** the backfill **and** the sign
> path **re-derive the pubkey at the path and compare to the row's
> `publicKeyData`** before trusting/signing — a present-but-wrong path otherwise
> signs silently and consensus rejects it (silent lockout); **(MF-3)** the
> scalar-field-deletion gate is a **runtime migration stamp**, not a dev-time
> check, and the field-deleted build still runs the Keychain-driven backfill on
> first launch (the Keychain survives a SwiftData store rebuild); **(MF-4)** the
> SwiftData column addition must be a verified-clean lightweight migration (or an
> explicit `MigrationStage`), since this app has historically rebuilt the V1 store
> from scratch; **(MF-5)** the schema delta is exactly `walletId` +
> `identityDerivationPath`, and the FFI layout guard recomputes to exactly **184**.

---

## 1. Problem

Identity-key discovery carries a verified 32-byte ECDSA scalar **Rust → FFI →
Swift** so the iOS Keychain stores it directly:

- `discovery.rs::derive_key_breadcrumbs` derives a candidate scalar per on-chain
  key; `breadcrumb_decisions` gates each via
  `IdentityPublicKey::validate_private_key_bytes(scalar, network)` and carries the
  reproducing scalar as `KeyWithBreadcrumb.verified_scalar`
  (`changeset.rs:391`).
- It rides `IdentityKeyEntry.private_key` (`changeset.rs:434`, `#[serde(skip)]`,
  redacting `Debug`), is copied by value into `IdentityKeyEntryFFI.private_key:[u8;32]`
  (`identity_persistence.rs:312`, with `private_key_is_some`), and Swift writes the
  32 bytes to the Keychain via `storeCarriedIdentityKey` →
  `KeychainManager.storeIdentityPrivateKey` under account
  `identity_privkey.<walletId>.<derivationPath>`.
- At sign time the `keyType < 5` branch reads that stored scalar back out
  (`KeychainSigner.swift::lookupIdentityPrivateKey` → `ffiSign`).

This is not a resident keystore — the scalar transits same-tick, is `Zeroizing`,
never serialized, never written to SQLite, and **no Rust signing path reads it**
(Rust signs via the external `VTableSigner`). It exists because the imported-wallet
flow ran `createWallet` (→ discovery) *before* `storeMnemonic`, so the old
Swift re-derive-from-mnemonic produced 23/23 watch-only keys; carrying the
already-verified scalar removed Swift's mnemonic dependency (commit `c567981c46`).

**Why change it anyway.** The carried scalar is still a raw secret crossing the FFI
ABI and stored per-key at rest. The clean model — already proven for platform
addresses — keeps the only secret (the seed) in the Keychain and derives each
signing key on demand. Removing the carry yields: the raw scalar never crosses the
FFI; one fewer class of secret at rest (no per-key scalar); Rust discovery never
materializes the scalar at all (verify via public key).

**The hard part.** This is the single most safety-critical path in the wallet: a
wrong key-storage/resolution change **locks users out of signing**, and the change
is only *validatable* against the iOS Keychain signer (iOS-gated). The design must
make the cutover non-lockout **by construction**.

---

## 2. Current vs. target architecture

### Current (carried scalar)

```
discovery.rs  derive candidate scalar  ── validate_private_key_bytes(scalar) ──┐
                                                                               │ verified_scalar: Some
changeset     KeyWithBreadcrumb.verified_scalar ─► IdentityKeyEntry.private_key│
FFI           IdentityKeyEntryFFI.private_key[32] + private_key_is_some  (by value)
Swift store   storeCarriedIdentityKey ─► Keychain item  identity_privkey.<wid>.<path>  (32 raw bytes)
Swift sign    keyType<5 ─► lookupIdentityPrivateKey (read scalar back) ─► ffiSign
```

### Target (derive-sign-destroy)

```
discovery.rs  derive candidate PUBLIC key ── compare to on-chain pubkey ──┐  (no scalar materialized)
                                                                          │ breadcrumb: Some, scalar: ABSENT
changeset     KeyWithBreadcrumb{ key, breadcrumb }            (no verified_scalar)
FFI           IdentityKeyEntryFFI{ …, wallet_id, identity_index, key_index } (breadcrumb only — already crosses)
Swift store   persistIdentityKeys ─► PersistentPublicKey.{walletId, identityDerivationPath}  (queryable columns)
Swift sign    keyType<5 ─► resolveIdentityKeyContext ─► dash_sdk_sign_with_mnemonic_resolver_and_path
                           (resolve mnemonic in-callback ─► derive ─► sign ─► zeroize; only the signature returns)
```

The breadcrumb `(wallet_id, identity_index, key_index)` **already crosses the FFI**
(`identity_persistence.rs` `wallet_id`/`identity_index`/`key_index`, gated behind
`wallet_id_is_some` / `derivation_indices_is_some` — present whenever the entry has
a breadcrumb, independent of the scalar). It is currently used only to build the
Keychain account label and is then discarded. So `persistIdentityKeys` must guard
the new-column write on `entry.derivationIndices != nil` (the snapshot already
exposes `derivationIndices` and `walletId`).

---

## 3. Chosen approach

Mirror the **platform-address** derive-sign-destroy path, which already works
end-to-end, and reuse its Rust primitive.

### 3.1 Discovery verifies via public key (no scalar)

The DIP-9 identity-auth path `m/9'/coin'/5'/0'/ECDSA'/identity_index'/key_index'`
is fully hardened, so the candidate pubkey must still be derived from a master
xpriv (resolved on demand inside the FFI, wiped before return — already the case
for external-signable wallets). The change is local to discovery: compute the
candidate **compressed public key**
(`derive_ecdsa_identity_auth_keypair_from_master(..).public_key`) and compare to
the on-chain key — `key.data() == pubkey` for `ECDSA_SECP256K1`,
`ripemd160_sha256(pubkey) == key.data()` for `ECDSA_HASH160` — instead of calling
`validate_private_key_bytes(scalar)`. **Do not populate `candidate_scalars`.**

This is byte-for-byte the same decision `validate_private_key_bytes` makes
internally; it just never needs the scalar to leave the derive function. The
transient master resolution in discovery is **not** what we eliminate — the
persisted/carried per-key scalar is.

An uncompressed externally-registered ECDSA key (65-byte on-chain `data()`)
correctly stays watch-only because the wallet only ever derives the **compressed**
form, so the compare gracefully fails — *not* because Platform forbids uncompressed
identity keys (it does not; `UncompressedPublicKeyNotAllowedError` is an asset-lock
constraint only). Stating the real reason avoids a future "optimization" that
assumes uncompressed identity keys can't exist on-chain.

### 3.2 Sign via the existing resolver primitive (no new FFI)

`dash_sdk_sign_with_mnemonic_resolver_and_path`
(`rs-platform-wallet-ffi/src/sign_with_mnemonic_resolver.rs`) is **generic over the
derivation path and ECDSA-only**. Every wallet-derivable identity auth key is ECDSA
(guaranteed by the discovery verify gate), so the identity signing branch calls
this primitive **unchanged** with the DIP-9 identity-auth path string. No new FFI
signing entry point is required.

**Sign-time binding check (MF-2).** Today's stored-scalar lookup is *intrinsically*
correct — the scalar it returns was the one verified to reproduce that exact pubkey
at discovery. The resolver path loses that: it routes by `wallet_id_bytes` and
derives at `identityDerivationPath`, but never confirms the result matches the key
being signed for. A mis-mapped resolver slot or a stale path would derive a
*different, valid* scalar and produce a signature consensus silently rejects. So the
identity sign path **must verify the derived compressed pubkey equals the row's
`publicKeyData` before signing** (derive-and-compare inside the FFI, or a
pubkey-preview call before `sign`), and fail loud on mismatch rather than emit a
wrong-key signature. This restores the intrinsic binding the stored scalar gave for
free.

`signIdentityKeyOnDemand` and `signPlatformAddressOnDemand` differ only in their
SwiftData lookup; the `sigBuf` setup + FFI call + error handling are identical.
Extract a shared private `signOnDemandWithContext(walletId:path:expectedPubKey:data:)`
so the two branches don't duplicate ~30 lines (the identity branch passes
`expectedPubKey`, enabling the MF-2 check; the address branch passes `nil`).

### 3.3 Persist the breadcrumb as queryable columns

`PersistentPlatformAddress` carries `walletId: Data` + `derivationPath: String` and
the signer reads them in `resolvePlatformAddressContext`. `PersistentPublicKey`
carries only `privateKeyKeychainIdentifier` — no breadcrumb. Add **exactly two**
columns: `walletId: Data?` and `identityDerivationPath: String?`. Both are required
by the resolver FFI — `wallet_id_bytes` is a mandatory parameter (it keys the
mnemonic-resolver callback), and the full path string is what
`dash_sdk_sign_with_mnemonic_resolver_and_path` derives at. **Do not** add separate
`identityIndex`/`keyIndex` columns — they are redundant with the path string (the
inverse of `getIdentityAuthenticationPath`) and the path is authoritative.
`persistIdentityKeys` writes the two columns, building the path with
`KeyDerivation.getIdentityAuthenticationPath` — the same path
`storeCarriedIdentityKey` already computes and currently throws away — and **always
overwrites** when the FFI breadcrumb is present, so a backfilled value and a
fresh-persister value for the same row are byte-identical (a string-format drift
between the two would otherwise desync the stored path from what the resolver
re-derives).

**Migration safety (MF-4).** Two optional columns are the additive shape SwiftData
lightweight migration handles — *but* this app's `DashModelContainer` runs
`DashSchemaV1` with `stages: []` and has historically rebuilt the dev store from
scratch on any model-hash change. Adding columns must be confirmed to
lightweight-migrate **a real persisted production store on upgrade** (not just a
fresh install); if SwiftData instead rebuilds the store, every
`PersistentPublicKey` row vanishes and the §5 backfill has no rows to heal. Either
verify the clean lightweight path on a real upgrade or bump to `DashSchemaV2` with
an explicit additive `MigrationStage`. Because the backfill is **Keychain-driven**
(§5) and the Keychain survives a SwiftData rebuild, a wiped row set degrades to
re-materialization from the Keychain rather than to lockout — but the migration
shape must still be pinned, not assumed.

### 3.4 Alternatives rejected

- **Keep the carried scalar (status quo).** Rejected: leaves a raw secret crossing
  the ABI and a per-key secret at rest; diverges from the platform-address model
  and the broader "derive on load, don't persist secrets" direction.
- **Re-derive in Swift from the mnemonic at sign time (the pre-`c567981c46`
  path).** Rejected: this is exactly the anti-pattern `swift-sdk/CLAUDE.md` forbids
  (Swift running `mnemonic → seed → path → key`), and it was the original
  imported-identity bug. The resolver primitive keeps the derive inside Rust with
  only the path crossing.
- **New identity-specific FFI signing call carrying `(identity_index, key_index)`.**
  Rejected as unnecessary now: the generic path primitive already covers every
  ECDSA identity key. (A non-ECDSA wallet-derivable identity key — none exist today
  — would be the only reason to add one.)

---

## 4. Phased delivery

**Hard ordering invariant:** the FFI/changeset scalar field is deleted **last**,
only after every already-materialized identity key is proven to sign via the
resolver path on a real device. Deleting it earlier — even though Rust still
compiles — bricks the still-scalar-based Swift signer = lockout.

### Phase 1 — headless-safe (Rust + FFI only; additive, removes nothing Swift reads)

Gate: `cargo test -p platform-wallet` + `-p rs-platform-wallet-ffi` green;
cross-compile `aarch64-apple-ios-sim`. No ABI change.

1. Compute the pubkey-compare decision in `breadcrumb_decisions` and **assert in
   tests** it is byte-equivalent to the scalar decision (`reproduces`). This is
   *not* a second parallel derive: the candidate pubkey is already a byproduct of
   the existing keypair derivation, so the only change is what the decision logic
   compares. Production emission is unchanged — `verified_scalar: Some` still
   ships in Phase 1 because Swift still reads it; the switch to pubkey-only happens
   in Phase 2 step 6.
2. Test the identity DIP-9 path through `dash_sdk_sign_with_mnemonic_resolver_and_path`
   (the existing happy-path test already uses `m/9'/1'/5'/0'/0'/0'/0'` — confirm it
   covers the identity case or augment it; this step may be test-only, no new code).
3. Test that the FFI breadcrumb round-trips with `private_key_is_some == false`.

Steps 1–3 have no internal ordering dependency and can land as one atomic Rust commit.

### Phase 2 — iOS-gated (Swift + on-device), ordered

1. Add `walletId` + `identityDerivationPath` columns to `PersistentPublicKey` (both
   optional ⇒ SwiftData lightweight migration; existing rows get `nil`; no data
   loss).
2. Write the breadcrumb columns in `persistIdentityKeys` — **both** old (Keychain
   scalar) and new (columns) during the transition window.
3. **Backfill migration (the lockout defense — see §5).** One-time, Keychain-driven,
   self-verifying pass populating the new columns from each `identity_privkey.*`
   item's `IdentityPrivateKeyMetadata` blob, re-deriving the pubkey at the path and
   comparing to the row's `publicKeyData` before trusting it — no network, no seed.
4. Re-route the `keyType < 5` signer branch through `signIdentityKeyOnDemand` +
   `resolveIdentityKeyContext` (mirroring the platform-address pair), calling the
   existing resolver primitive. **Resolver-first with legacy fallback:** a row with
   no `identityDerivationPath` falls back to `lookupIdentityPrivateKey → ffiSign`;
   every fallback hit is logged (count only, no key material).
5. **Validation gate:** transitional build on a funded testnet wallet; exercise
   signing for every identity (DPNS register, profile set, contact-request
   send+accept, payment); confirm **zero fallback hits** after backfill.
6. **Only after the gate:** flip discovery to pubkey-only; delete
   `storeCarriedIdentityKey`, the Swift scalar copy/scrub, and the legacy signer
   path; then delete the scalar field from FFI (recompute the layout guard from
   `const _: [u8; 224]` to exactly `const _: [u8; 184]` — removing
   `private_key_is_some` at offset 184 + `private_key: [u8; 32]` + trailing padding
   drops bytes 184–223, alignment stays 8) and changeset; regenerate the header;
   rebuild.

**Runtime deletion gate (MF-3) — not a dev-time gate.** Step 6 must not assume every
device passed through the transitional build. A user can upgrade straight from the
scalar-only build to the field-deleted build, skipping the backfill; their rows have
`identityDerivationPath == nil` and there is no legacy signer left → lockout. So:
(a) the field-deleted build **still runs the Keychain-driven backfill on first
launch** (the Keychain items survive any SwiftData rebuild, so the path is
recoverable even with no transitional run); and (b) deleting the *legacy signer
fallback* is gated on a **persisted migration stamp** (set only after a backfill
pass leaves zero un-pathed rows / after the scalar Keychain items are purged), so a
binary without the stamp keeps the fallback and schedules a backfill. The fallback,
not just the field, is the safety net — it lives until the stamp guarantees the
resolver path covers 100 % of the live key set at runtime.

---

## 5. Migration / back-compat — the lockout defense (new design)

**Danger:** existing installs have scalars in the Keychain under
`identity_privkey.<walletId>.<derivationPath>` and `PersistentPublicKey` rows whose
new breadcrumb columns are `nil` after the lightweight migration. If signing flips
to resolver-only and a row has no `identityDerivationPath`, that key is unsignable
→ the user is locked out of an already-working identity.

Three layers, all required:

1. **Keychain-metadata-driven, self-verifying backfill (no network, no seed)
   (MF-1, MF-2).** Each `identity_privkey.*` Keychain item carries a first-class
   `IdentityPrivateKeyMetadata` JSON blob (`kSecAttrGeneric`) with named `walletId`,
   `derivationPath`, `identityIndex`, `keyIndex`, `publicKey` fields —
   `KeychainManager.identityPrivateKeyAccount` already walks every row and decodes
   it. The one-time backfill reads `walletId` + `derivationPath` from the **blob**
   (not from a parse of the `identity_privkey.<walletId>.<derivationPath>` account
   label, which is fragile and has a legacy no-`walletId` variant). It is driven by
   the **Keychain item set**, not the SwiftData row set, so it heals even if the
   SwiftData store was rebuilt (MF-4) — it can re-create the `PersistentPublicKey`
   linkage from the blob's `publicKey`. Crucially it is **self-verifying**: before
   writing `identityDerivationPath`, re-derive the compressed pubkey at that path
   (resolver / pubkey-preview FFI) and require it to equal the row's
   `publicKeyData`; on mismatch leave the column `nil` so the row falls through to
   the fallback / re-discovery rather than to a wrong-key sign. A non-zero count of
   parse-or-verify failures is a **hard blocker** on the deletion gate (it is not
   enough to test that signing works — every existing item must be accounted for).
2. **Resolver-first with legacy fallback.** During the transition the signer tries
   the resolver path first and falls back to the stored scalar when the breadcrumb
   is absent, so a row can always sign via at least one path. Non-lockout by
   construction. (Note: the fallback covers an *absent* path, not a *present-but-
   wrong* one — MF-2's sign-time binding check is what catches the latter.)
3. **Re-discovery heals the rest.** Any row not covered by (1) re-materializes on a
   from-0 rescan (now writing the breadcrumb columns, needing only the resolver
   mnemonic). Surface a "re-scan identities" affordance.

**Population that blocks deletion (MF-3 / R5).** A wallet with a materialized scalar
but **no readable mnemonic** (the import-flow case that motivated the carried scalar
originally) can never reach zero resolver-fallbacks — the resolver needs the
mnemonic. For that population the legacy scalar fallback must be **retained**, or an
explicit mnemonic-import step required, before its scalar field/path can be removed.
The "zero fallback hits" criterion is otherwise unachievable for exactly the wallets
the carried scalar was introduced to serve.

The legacy fallback and the scalar field are deleted **only** after the runtime gate
(MF-3) confirms, per device, that the resolver path covers the full live key set —
not merely after a dev-time test pass.

---

## 6. Failure modes & risk register

| ID | Risk | Mitigation |
|----|------|------------|
| R1 | Pubkey-verify diverges from scalar-verify → a key wrongly breadcrumbed (signable with an unauthorized key) or wrongly watch-only | Phase-1 byte-equivalence test over `ECDSA_SECP256K1` + `ECDSA_HASH160` + foreign key; assert decision set identical to `breadcrumb_decisions` |
| R2 | Existing rows lack breadcrumb columns → resolver-only signer locks out already-materialized keys | §5: backfill migration + resolver-first-with-fallback + zero-fallback gate before deleting legacy |
| R3 | Wrong network → wrong DIP-9 path → wrong key / sign failure | Resolve network from `PersistentWallet` exactly as `storeCarriedIdentityKey` does; unit-test the built path equals the Keychain account string |
| R4 | ABI/layout drift on field removal → `EXC_BAD_ACCESS` in the callback | Recompute `const _: [u8; N]` + the byte-offset comment; cbindgen regen; round-trip test |
| R5 | Resolver mnemonic missing/locked at sign time (watch-only, biometric-gated, import-only wallet with a scalar but no mnemonic) → sign fails where the stored scalar succeeded; zero-fallback gate unachievable for this population | Existing `mnemonicMissing` UX; **retain the legacy scalar fallback for the no-mnemonic population** (§5) — do not delete its scalar/path until a mnemonic-import step runs |
| R6 | A non-ECDSA wallet-derivable identity key appears (future) → the ECDSA-only resolver rejects it | Pre-existing constraint, **not introduced by this change** (the scalar path is already ECDSA-only via `validate_private_key_bytes`); discovery only breadcrumbs ECDSA; a non-ECDSA key would need a new resolver FFI |
| R7 | Old per-key scalars linger in the Keychain indefinitely → negates "one fewer secret at rest" | **Required (not optional)** purge of `identity_privkey.*` items, gated on the same runtime stamp; also doubles as the MF-3 migration-completed signal |
| R8 | **Skip-version upgrade lockout** — user jumps from scalar-only to field-deleted build, skipping the backfill; rows have `identityDerivationPath == nil` and no legacy signer remains | MF-3: field-deleted build still runs the **Keychain-driven** backfill on first launch (Keychain survives a SwiftData rebuild); legacy-fallback deletion gated on a persisted migration stamp, not a dev-time check |
| R9 | **Wrong-mnemonic / present-but-wrong-path silent signing** — resolver routes by `wallet_id_bytes` and derives a valid-but-wrong scalar; signature fails only at consensus, no local diagnostic | MF-2: derive-and-compare the pubkey to the row's `publicKeyData` before signing (and in the backfill before trusting a path); fail loud on mismatch |
| R10 | SwiftData store rebuild on column add wipes `PersistentPublicKey` rows → backfill has nothing to heal | MF-4: verify clean lightweight migration on a real upgrade or declare a `MigrationStage`; backfill is Keychain-driven so a wiped row set degrades to re-materialization, not lockout |

---

## 7. Test / verification plan (red→green)

**Phase 1 (headless):**
- `discovery.rs` `#[cfg(test)] mod tests` — `breadcrumb_via_pubkey_equivalence`:
  derive a multi-key identity, run both the scalar path and the new pubkey path,
  assert identical `(breadcrumb, key)` decisions and that the pubkey path carries no
  scalar; extend the existing HASH160 + non-reproducible-key tests. **Red first**
  (new path wrong), then green. This is the most important Rust correctness gate (R1).
- `sign_with_mnemonic_resolver.rs` tests — `signs_with_dip9_identity_auth_path`
  (identity path string, verify signature). Confirms no new FFI is needed.
- `identity_persistence.rs` tests — breadcrumb survives `from_entry` with
  `private_key_is_some == false`; (Phase-2 step 6) update the size guard to the new
  `N` and assert no scalar field.
- `rs-platform-wallet-storage` round-trip — `IdentityKeyWire` still has no secret
  field; compiles after `private_key` removal.

**Phase 2 (iOS/sim):**
- `KeychainSignerIdentityResolveTests` — `signIdentityKeyOnDemand` resolves
  `(walletId, identityDerivationPath)` from a seeded row and signs via a mock
  resolver; `canSign` is true with breadcrumb+mnemonic, false without. **Plus the
  MF-2 binding test:** a resolver returning a *wrong* mnemonic (or a row with a
  *wrong* path) yields a **sign-failure, not a wrong-key signature** — assert the
  pre-sign pubkey compare rejects it.
- `PersistentPublicKeyBreadcrumbMigrationTests` — seed a Keychain
  `identity_privkey.*` item (with its `IdentityPrivateKeyMetadata` blob) + a row;
  run backfill; assert columns are populated **from the blob** and that the
  backfilled path **re-derives to the row's `publicKeyData`** (MF-2 self-verify);
  assert a blob whose path does *not* re-derive to its pubkey leaves the column
  `nil`; assert a row whose backfilled value and a fresh-persister value are
  byte-identical; assert a row without a Keychain item falls back to legacy during
  the transition; assert backfill works with the **SwiftData row set empty**
  (Keychain-driven, MF-4).
- `BackfillCoverageTests` — every existing `identity_privkey.*` item is accounted
  for; a non-zero parse-or-verify-failure count blocks the deletion gate (MF-1).
- `persistIdentityKeys` writes the two columns from a breadcrumb-only
  (scalar-absent) entry, guarded on `derivationIndices != nil`.

**On-device acceptance (the real gate):**
- Transitional build over an existing store with already-materialized identities →
  backfill runs → exercise signing for every identity (DPNS / profile / contact
  request / payment) → **zero legacy-fallback hits** logged.
- Fresh wipe → import funded testnet seed → discover (pubkey-verify, no scalar
  carried) → sign → success.
- Wrong-seed rejection (`verify_seed_binds`) still holds.

**Field deletion is gated:** `git grep verified_scalar` /
`IdentityKeyEntry.private_key` empty only after the zero-fallback on-device gate
passes. If fallbacks > 0, **do not delete** — the scalar is the safety net until the
resolver path is proven for 100 % of the live key set.

---

## 8. Critical files

- `packages/rs-platform-wallet/src/wallet/identity/network/discovery.rs` —
  pubkey-verify in `breadcrumb_decisions` / `derive_key_breadcrumbs`; equivalence test.
- `packages/rs-platform-wallet-ffi/src/sign_with_mnemonic_resolver.rs` — reuse the
  signing primitive; add an **optional `expected_pubkey` param** for the MF-2
  derive-and-compare-before-sign check (the address path passes none); add a
  DIP-9-path sign test + a wrong-seed-rejects test.
- `packages/rs-platform-wallet-ffi/src/identity_persistence.rs` — (Phase 2 step 6)
  delete `private_key` / `private_key_is_some`; recompute the layout guard
  `const _: [u8; 224]` → `const _: [u8; 184]` (drops bytes 184–223; align stays 8);
  update the byte-offset comment.
- `packages/rs-platform-wallet/src/changeset/changeset.rs` — (Phase 2 step 6) delete
  `KeyWithBreadcrumb.verified_scalar` + `IdentityKeyEntry.private_key`.
- `packages/rs-platform-wallet/src/wallet/identity/state/managed_identity/identity_ops.rs`
  — `add_keys`: drop the scalar from the destructure + entry literal.
- `packages/swift-sdk/Sources/SwiftDashSDK/Persistence/Models/PersistentPublicKey.swift`
  — add `walletId` + `identityDerivationPath`.
- `packages/swift-sdk/Sources/SwiftDashSDK/Persistence/DashModelContainer.swift` —
  pin the lightweight migration / `MigrationStage` (MF-4); host the persisted
  migration stamp gating legacy-path deletion (MF-3).
- `packages/swift-sdk/Sources/SwiftDashSDK/Security/KeychainManager.swift` —
  Keychain-driven backfill reads the `IdentityPrivateKeyMetadata` blob (`walletId`,
  `derivationPath`); required purge of `identity_privkey.*` after the gate (R7).
- `packages/swift-sdk/Sources/SwiftDashSDK/FFI/KeychainSigner.swift` —
  `signIdentityKeyOnDemand` + `resolveIdentityKeyContext` mirroring the
  platform-address pair; fallback dispatch; delete `lookupIdentityPrivateKey` /
  `ffiSign` in step 6.
- `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletPersistenceHandler.swift`
  — write breadcrumb columns; delete `storeCarriedIdentityKey` + the scalar
  copy/scrub.

---

## 9. As-built notes (what shipped vs. this spec)

The additive, fallback-protected work (Phase 1 + Phase 2 steps 1–5) shipped on
`feat/dashpay-identity-key-scalar-elimination`. Two deliberate deviations from the
rev-2 design above, plus what is explicitly **not** done:

- **Resolver FFI accepts `ECDSA_HASH160` (key_type 2), not just SECP256K1.** Full
  scalar deletion is impossible otherwise — discovery breadcrumbs both ECDSA key
  types. The MF-2 binding disambiguates by `expected_key_data` length: 33 bytes =
  compressed-pubkey equality, 20 bytes = `ripemd160_sha256(pubkey)` equality. The
  param is nullable (the address path passes none). This widens §3.2's "reuse the
  primitive unchanged."
- **The backfill self-check is canonical-path-from-indices, not a pubkey
  re-derivation.** §5 layer 1 specified re-deriving the pubkey at the path and
  comparing to `publicKeyData`; that needs the seed, which the backfill
  deliberately avoids. Instead it rebuilds the canonical DIP-9 path from the
  metadata's `(network, identityIndex, keyIndex)` and requires it to equal the
  stored `derivationPath` (rejecting format drift), and **relies on the sign-time
  MF-2 binding as the real guard** — a present-but-wrong path yields
  `ERR_PUBKEY_MISMATCH` at sign time → a logged `IDENTITY_SIGN_FALLBACK`, never a
  wrong-key signature. A non-zero backfill failure count is still surfaced.
- **Phase 2 step 6 — the carried scalar IS deleted; the legacy fallback signer is
  KEPT (partial by design).** `KeyWithBreadcrumb.verified_scalar`,
  `IdentityKeyEntry.private_key`, and `IdentityKeyEntryFFI.{private_key,
  private_key_is_some}` are removed (FFI layout guard recomputed `224` → `184`; the
  `from_entry` copy + free-scrub gone). Discovery now derives-verifies-**drops** the
  candidate scalar instead of emitting it — verification is still
  `validate_private_key_bytes`, the scalar just never leaves discovery. The legacy
  Keychain-scalar signer (`lookupIdentityPrivateKey` / `ffiSign`) is **retained** so
  keys already materialized on existing installs still sign: the §5 lockout defense
  is preserved *without* removing the legacy path. New keys are resolver-only.
  Merged into `feat/dashpay-m1-sync-correctness` (`fb11783706`; commits `930c100c64`
  deletion + `fa1098c081` test).
- **The funded-testnet zero-`IDENTITY_SIGN_FALLBACK` gate (§4 step 5 / §7) was
  un-runnable and was substituted.** idb cannot actuate this app's SwiftUI
  confirmation controls (the toolbar Create/Cancel, the backup-seed "I wrote it
  down" switch), and the macOS-click fallback needs Accessibility / Automation /
  Screen-Recording TCC the tmux-hosted shell lacks — so the automated UAT could not
  even create a wallet. With explicit sign-off ("delete the field if it passes"),
  the gate was replaced by a HEADLESS Swift integration test
  (`IdentityResolverSignIntegrationTests`): it seeds a real mnemonic in
  `WalletStorage` + a consistent breadcrumb row and asserts `signIdentityKeyOnDemand`
  → resolver → on-demand derive → MF-2 binding → valid 65-byte signature (plus a
  wrong-path → `.failure`). It stays green *after* the deletion — proof the resolver
  path signs through the Swift layer with no stored scalar. Verified end to end:
  platform-wallet 314 + FFI (125/26/9) tests, Swift IdentityResolverSign 2/2 +
  IdentityKeyBreadcrumb 4/4 + 30 Identity tests, clippy clean, SwiftExampleApp sim
  BUILD SUCCEEDED. Because the legacy fallback was retained, the `DashModelContainer`
  migration-stamp (MF-3) stays deferred (it only matters once the fallback is
  removed). Still unproven: a live on-device discover→materialize→sign over an
  existing store.
