# Kotlin/Swift SDK parity and shared-logic consolidation

**Status:** REVIEWED v2 — approved after Swift/Rust FFI, Android/JNI/Room, and
adversarial architecture reviews
**Baseline:** PR #3999, `feat/kotlin-sdk-and-example-app` at
`6dbc72a54df72d26eb9c4a014b425d2b95134e4e`
**Scope:** `rs-platform-wallet`, `rs-platform-wallet-ffi`, `rs-unified-sdk-jni`,
`swift-sdk`, `kotlin-sdk`, SwiftExampleApp, and KotlinExampleApp
**Related specifications:** `docs/dashpay/DIP15_INVITATIONS_SPEC.md`,
`docs/dashpay/KOTLIN_MIGRATION_SPEC.md`,
`docs/dashpay/KOTLIN_MIGRATION_FOLLOWUPS_SPEC.md`, and
`docs/dashpay/PENDING_CONTACT_CRYPTO_RELOCATION_SPEC.md`

**Implementation state on this branch:** partial, not release-complete. The
release-blocker slices are implemented at source/test level, but the executable
manifest remains authoritative for unclosed device, process-restart, and legacy
store gates. Android invitations and the S1–S4 shared-policy moves are explicitly
permitted follow-ups; this commit must not be presented as completing those later
slices.

---

## 1. Problem

PR #3999 establishes an Android/Kotlin SDK and ports SwiftExampleApp. The port is
large enough that file- or screen-count parity is no longer a reliable correctness
measure. Review found three kinds of drift:

1. Host persistence callbacks do not implement the same Rust persistence contract.
   Some Android omissions disable shared features; one Android callback writes
   incorrect authoritative data.
2. Protocol or wallet policy is duplicated in Kotlin and Swift even when Rust
   already owns, or should own, the rule. The copies have started to diverge.
3. The parity document records source-file presence rather than executable feature
   capability, restart behavior, and protocol-domain coverage.

The goal is not literal host-code equality. Kotlin/Swift UI, lifecycle, secure
storage, and database adapters remain platform-native. The goal is one shared
implementation of protocol/wallet decisions and equivalent host persistence,
recovery, and example-app capability.

### 1.1 Baseline and already-integrated work

Implementation starts from exact PR head `6dbc72a54d` or a descendant. That head
already contains these squash integrations:

- `516b265bf5` / #4106 — shared wallet deduplication and dead FFI/JNI removal;
- `f74465227f` / #4041 — shared and iOS DIP-13 invitation implementation;
- `97477d1c88` / #4093 — generalized existing-asset-lock top-up and iOS resume;
- `aba6af2420` / #4126 — shared and iOS Orchard viewing-key persistence.

The `refactor/platform-wallet-dedup`, `feat/dip15-dashpay-invitations`, and
`feat/kotlin-sdk-dashpay-migration` worktrees are historical design/review
references. They are behind the baseline and **must not be merged or cherry-picked**.
This effort adds the missing Android adapters and new shared APIs on the exact
baseline while preserving all later seed-binding, provider, and rollback fixes.

## 2. Non-negotiable invariants

1. **Rust owns protocol and wallet policy.** Coin selection, reservation,
   authorization, pricing, network endpoint interpretation, protocol constants,
   proposal-rule selection, and recovery state machines must not be independently
   reimplemented by Kotlin and Swift.
2. **Host bridges adapt; they do not orchestrate.** JNI and Swift wrappers may map
   types, dispatch queues, callbacks, cancellation, and errors. A host wrapper must
   not compose multiple fallible Rust calls into a new wallet transaction state
   machine.
3. **Persistence contracts are capability-checked.** Backends attest individual
   capabilities such as atomic changesets, invitations, Orchard FVKs, provider
   restore, and deferred contact crypto. A feature requiring durability must fail
   closed before broadcast when any required capability is absent. A present no-op
   callback violates the contract.
4. **Persisted identity is immutable.** Balance/update callbacks may not rewrite an
   address's derivation identity unless the callback explicitly represents a
   derivation-map mutation.
5. **Protocol integer domains survive every ABI.** Every protocol field whose valid
   consensus domain spans full `u64` must round-trip across Kotlin and Swift. A
   narrower carrier is permitted only where Rust enforces a narrower consensus
   bound.
6. **Recovery is part of feature parity.** A flow is not “ported” until it can resume
   after process death at every point where durable state or on-chain value exists.
7. **Parity is executable.** Every capability row names an automated test or an
   explicit device/testnet gate. Counts derived from source files are informational
   only.

## 3. Ownership boundary

| Concern | Shared Rust | Host SDK | Example app |
| --- | --- | --- | --- |
| Transaction input selection and reservation | Own | Invoke one composite API | Collect user intent |
| Masternode endpoint discovery | Own | Map endpoint result | Display diagnostics only |
| Token authorization, group rules, price quote | Own | Map typed decision/quote | Render decision and collect inputs |
| Protocol constants and amount validation | Own | Preserve exact domain | Format values |
| Recovery state machine | Own | Persist/restore required records and invoke resume | Present resumable operations |
| Persistence schema and callbacks | Define callback semantics/capabilities and ABI version | Implement in Room/SwiftData/Keychain/Keystore | Never bypass SDK persistence |
| UI/navigation/lifecycle | No | Expose async/cancellation-safe API | Own |

The following duplication is intentional: Room versus SwiftData entities and DAOs,
Android Keystore versus Apple Keychain, Compose versus SwiftUI, and ABI type mapping.
Those implementations must nevertheless satisfy the same Rust callback semantics.

## 4. Correctness blockers

**Current status (manifest reconciliation):** C1-C3 are the release-blocker
slices described in the implementation-state note above as implemented at
source/test level. For Kotlin, the manifest marks all three SDK capabilities
supported: C1 (`persistence.platform_address_identity`) has a restart-covering
Room regression; C2 (`core.atomic_send`) has shared reservation/failure unit
coverage plus the host `CORE-05` manual case, with restart correctly marked
`not_applicable`; and C3 (`tokens.full_u64_domain`) has Kotlin/JNI/Room/app
coverage plus a restart-covering device migration test. Swift still has open
restart gates for C1 and C3, so the manifest remains authoritative. The
“Required change” text below is retained as historical design rationale, not as
a claim that these Kotlin changes remain unimplemented.

### C1 — Android platform-address balance persistence corrupts derivation indices

The Android balance callback currently copies callback `accountIndex` and
`addressIndex` into the durable address row. Conflict-removal events may carry the
index of a competing address while intentionally leaving the authoritative
address/index bijection unchanged. Swift already ignores those two callback fields.

**Required change**

- Update only balance, nonce, used state, and height in the Android balance path.
- Preserve the stored account/address indices for an existing address.
- Add a regression test that seeds address A at index A, delivers a zero-balance
  callback for A carrying index B, restarts/restores, and asserts A still maps to A.
- Document the callback semantics beside the JNI callback declaration.
- C1 is prevention-only on the unreleased PR #3999 baseline and requires no Room
  migration. If a distributed beta database must later be repaired, recovery must
  come from a Rust-authoritative address-pool re-emit, never SQL or host parsing of
  derivation paths.

**Acceptance:** the pre-existing row's account/address tuple and derivation path are
unchanged; Rust's restored bijection maps the canonical address at index A; and a
later valid credit to A is accepted. Database-level uniqueness or cleanup of benign
zero-balance conflict remnants is not required.

### C2 — Core transaction construction is not atomic

The shared builder separates funding from signing. Two concurrent calls can select
the same UTXO. Android serializes its wrapper, but Swift exposes the split public API
and other consumers can bypass the Android mutex.

**Required change**

- Add the composite to `rs-platform-wallet` first and expose it through thin C/JNI
  adapters. Selection and recording in the account `ReservationSet` must be one
  indivisible operation: no competing selection may observe the chosen inputs as
  available.
- The design must not depend on holding the wallet-manager lock across a host
  mnemonic-resolver callback. If key-wallet cannot reserve before signing, add the
  required reservation primitive there or use an explicit per-wallet atomic gate in
  platform-wallet.
- Add a finalize API that consumes an unfunded/configured builder, atomically funds
  and reserves it, then signs. Validation/signing failure and explicit abandon
  release the reservation; definitive broadcast rejection releases it; ambiguous
  `MaybeSent` retains it under the existing TTL/reconciliation policy.
- Return a new opaque/V2 signed-transaction handle containing fee, funding account,
  and reservation metadata. Do not extend `FFICoreTransaction` in place or alter an
  existing C layout. Add explicit broadcast and abandon functions.
- Route Kotlin and Swift convenience sends through the composite. Deprecate the old
  split builder symbols and public Swift sequence without removing their ABI in this
  release.

**Acceptance:** a barrier-forced concurrent same-UTXO test produces disjoint
reservations or one typed reserved/insufficient-funds failure. Tests also cover
validation/sign failure, explicit abandon, definitive rejection, ambiguous send,
builder consumption, and double-free safety.

### C3 — Host SDKs preserve protocol `u64` values end to end

Kotlin previously narrowed token amounts and direct-purchase costs to signed
`Long`. SwiftData uses its original signed `Int64` balance column as a
schema-neutral raw-bit carrier and exposes unsigned values at the SDK/UI boundary;
no alternate iOS balance column or migration-only code path is introduced.

**Required change**

- Public Kotlin token APIs use `ULong` (or one `TokenAmount` value type backed by
  `ULong`). Internal native declarations remain `Long`/`jlong` raw-bit carriers so
  existing JNI names and descriptors remain stable. Kotlin passes `value.toLong()`;
  Rust reinterprets the bits with `as u64` and does not reject the sign bit.
- `ULong` and `Long` erase to the same JVM carrier. Any deprecated checked-`Long`
  compatibility adapter must therefore have a distinct Kotlin name or explicit
  `@JvmName`, reject negative values before the raw-bit native call, and be listed in
  a per-method compatibility table.
- Java callers use one documented `BigInteger` or eight-byte adapter layered on the
  same native path. Do not create a parallel native implementation.
- Audit mint, burn, transfer, set-price, purchase amount/cost, max supply, distribution
  values, results, persistence, comparisons, and UI formatting. Preserve the existing
  unsigned-decimal max-supply JSON behavior.
- In Room v5, replace signed SQL semantics such as `TokenBalanceEntity.balance` plus
  `WHERE balance > 0` with an order-preserving unsigned representation. Use a fixed
  eight-byte big-endian BLOB (lexicographically unsigned-order-preserving) and
  explicit zero comparison, or document an equally lossless/order-preserving schema;
  values at/above `2^63` must not disappear from DAO results.
- Separate codec-boundary tests (`0`, `Long.MAX_VALUE`, `2^63`, `u64::MAX`) from
  operation semantics where zero may still be invalid.

**Acceptance:** every full-domain token `u64` round-trips through JNI, Room, DAO
queries, and UI formatting without loss or signed-order errors.

Direct-purchase quotes additionally follow Drive's operation semantics rather
than applying a generic full-`u64` arithmetic rule: amount is limited to DPP's
`2^48 - 1` distribution maximum; single-price multiplication saturates at
`u64::MAX`; set-price multiplication rejects overflow; and a configured zero
price remains valid.

**Kotlin source-compatibility accounting:** PR #3999 introduces an unreleased
Kotlin SDK surface, so the signed declarations below have no published consumer
contract to deprecate. They are intentionally corrected in place; adding signed
overloads would preserve an invalid negative-value domain and, because `Long` and
`ULong` erase to the same JVM carrier, would complicate the public ABI. Java uses
`JavaTokenActions`/`BigInteger`; JNI descriptors remain unchanged.

| Kotlin operation | Corrected parameter(s) | Compatibility disposition |
| --- | --- | --- |
| `mint`, `burn`, `transfer` | token amount `Long` → `ULong` | unreleased source break; same raw `jlong` JNI ABI |
| `setPrice` | price `Long` → `ULong` | unreleased source break; same raw `jlong` JNI ABI |
| `purchase` | amount and expected cost `Long` → `ULong` | unreleased source break; same raw `jlong` JNI ABI |
| `updateConfig` | optional max supply `Long?` → `ULong?` | unreleased source break; JSON/native encoding remains unsigned |
| distribution/claim numeric values | protocol `u64` values → `ULong` | unreleased source break; Java checked adapter where exposed |

## 5. Android persistence and restart parity

### P1 — Orchard full-viewing-key persistence

Implement Room storage and JNI callbacks for persist/load/free of shielded viewing
keys, matching the existing Swift persistence contract.

The exact shared preflight is `atomic_changesets + shielded_viewing_keys`. The
`shielded_viewing_keys` bit is backend-attested and then intersected with the
complete persist/load/free callback triplet; generic wallet-list `wallet_restore`
is not part of this contract because seedless rebind loads FVK rows directly.

- Key rows uniquely by `(walletId, accountIndex)`; wallet IDs are already
  network-specific, so do not add a redundant network key.
- Store exactly 96 FVK bytes. A present malformed row fails closed instead of
  silently falling back to the mnemonic.
- Implement callback allocation/free pairing, duplicate-account upsert, wallet purge,
  and wrong-wallet/network isolation.
- Reserve Room schema v6 for this entity and export its schema JSON.

**Acceptance:** create/bind shielded state, terminate, make the mnemonic unavailable,
restart, and successfully rebind/sync from the stored viewing key. Unit/instrumented
tests cover corrupt length, multiple wallets/accounts, callback allocation/free, and
wallet deletion.

### P2 — Invitation persistence and Android invitation UX

The shared/iOS invitation protocol, durability ordering, and reclaim semantics are
defined by `DIP15_INVITATIONS_SPEC.md`; this specification does not fork them.

Implement Android:

- Push-only invitation persistence callbacks and Room UI schema, including failure
  propagation. Rust does not rehydrate the invitation list; tracked asset-lock
  restore preserves reclaimability.
- Create, parse/claim, sent-invitations, and reclaim SDK wrappers.
- Deep-link/QR handling with the canonical legacy-compatible envelope.
- Create, Claim, Sent Invitations, and Reclaim screens.
- Test-plan cases `DP-12...DP-19`, including interrupted create/reclaim and
  already-consumed ambiguity.

Creation must remain gated by Rust's exact invitation capability set:
`atomic_changesets + asset_lock_funding_indices + invitations + wallet_restore`.
The narrower `persists_durably()` compatibility wrapper is not sufficient for new
feature code. A no-op callback is not an acceptable compatibility mode. A callback
failure returns nonzero inside the changeset begin/end round and rolls it back.
Persist `reclaimInFlight` transactionally before consume; write `Reclaimed`
only after observed success and use the canonical conservative `Claimed` ambiguity
classification. Room is v8 in the serialized migration chain. Purge by wallet and
never log URI/WIF secrets.

### P3 — Provider-special-transaction restoration

Android already stores raw transaction bytes, but payload-only provider transactions
create no TXOs. A TXO join therefore cannot reconstruct wallet/account ownership.

**Required change**

- Preserve existing C layouts: `TransactionRecordFFI` already carries
  `block_position`/`has_block_position`, and `AccountChangeSetFFI` already surrounds
  its transactions with the full typed account identity. Change only the JNI/Kotlin
  transaction callback parameters so each call explicitly forwards the enclosing
  account fields and the transaction's existing position fields. Do not add C POD
  fields or rely on mutable “current account” callback state.
- Add nullable/default block-position columns and a transaction↔account involvement
  cross-reference. Reserve Room v7 for these additions and exported schema.
- On cold load, select provider transaction kinds 2...5 through wallet/account
  involvement, marshal raw consensus bytes plus context/block metadata into the
  existing `ProviderSpecialTxRestoreEntryFFI`, and keep every backing byte buffer
  alive until the restore release callback.
- Rust re-decodes the payload and rebuilds ownership/masternode grouping; decoded
  host columns are not restore authority.

**Acceptance:** a payload-only provider transaction belonging to wallet A restores
only to A, survives without a TXO, preserves same-block ordering and optional block
position, and malformed raw data is diagnosed/skipped without crashing.

### P4 — Deferred contact-crypto queue

Do not add write-only host persistence. Implement restore in the shared wallet
start-state path first, then add the callback contract and both Room and SwiftData
stores in the same slice. After the relocation described by
`PENDING_CONTACT_CRYPTO_RELOCATION_SPEC.md`, restore hydrates identities first and
then fans rows into the owning `ManagedIdentity` across both identity buckets.
Unknown-owner rows are retained/quarantined or explicitly diagnosed, never silently
dropped. Define POD ownership/free semantics, idempotent operation identity, clear
tombstones, wallet scoping, and corrupt-row behavior. Until then, document that the
recurring sweep re-enqueues work and parity is delayed rather than durable.

`PersistenceCallbacks` currently has no struct-size negotiation. New callback slots
must use a versioned `PersistenceCallbacksV2`/constructor unless the release explicitly
declares framework and wrapper lockstep source ABI. In either case add cbindgen header,
C layout, and Swift `MemoryLayout` pins; never silently insert fields into the current
layout.

## 6. Recovery parity

### R1 — Existing asset-lock resume on Android

Generic Platform-address (`ADDR-03`) and shielded resume are already implemented and
remain unchanged. Bridge only the missing identity operations:

- `platform_wallet_resume_identity_with_existing_asset_lock_signer` for registration;
- `platform_wallet_topup_identity_with_existing_asset_lock_signer` for top-up.

Also bridge tracked-lock enumeration/status (with paired array free) so UI rows are
not reconstructed from private Room assumptions. Eligible generic rows have funding
type registration (`0`) or top-up (`1`/`2`) and a resumable status from Built through
ChainLocked (`0...3`). Funding type invitation (`3`) is never offered by generic
recovery. The shared wallet retains a consumed lock as a terminal tombstone so an
exact-outpoint retry remains a typed already-consumed error in the same process and
after restoration; actionable recovery lists exclude status `4`.

The registration result's managed-identity handle must be adopted immediately and
freed on every post-call failure. Both operations borrow the mnemonic resolver under
the manager teardown gate. The Android UI uses the same restored outpoint and never
creates a second funding transaction. Generic paths always pass
`consumeInvitationVoucher=false`; only P2 reclaim may pass `true`.

**Acceptance:** separate registration-resume and top-up-resume coverage (`ID-16`
covers top-up), interruption immediately after Core broadcast and before Platform
submission, and typed handling of untracked, foreign, and already-consumed locks. A
`Built` row re-broadcasts the same transaction and never creates a second distinct
funding transaction. A type-3 row is rejected by generic resume.

### R2 — Compact-filter rescan on Android

Expose the shared SPV rescan operation through JNI/Kotlin and add the height picker
and rescan state to the Android sync screen. The call rewinds an in-memory compact
filter checkpoint; it does not itself scan. A running SPV manager acts on the next
tick, a stopped manager acts on next start, equal/forward requests are harmless
no-ops, and unknown wallets return typed errors. Per-wallet failures are collected.
The rewind is not durable: process death before the filter loop consumes and persists
progress loses the rescan request, so the host/user must reissue it. Correct the
misleading shared Rust documentation when R2 is implemented. Do not promise
cancellability or durable rewind without adding a durable rescan-intent contract.

### R3 — Contested usernames by identity

Bridge both existing shared operations:

- `platform_wallet_sync_contested_dpns_names`, which performs one network fetch and
  full-snapshot persistence so resolved contests disappear;
- `managed_identity_get_contested_dpns_names`, which returns the cached array, with
  its paired free function.

Alternatively add one Rust composite returning an owned `DpnsNameArray`. Replace
Android's bounded local-label probing with this path.

**Acceptance:** an identity with more than eight locally unknown contested names is
shown completely with one logical query.

## 7. Shared-policy consolidation

### S1 — Masternode discovery

Discovery currently bootstraps DAPI before an SDK handle exists. Add a standalone
shared Rust entry point taking `(network, quorum_base)` and returning owned typed
records containing both Core peer address and DAPI URL, with an explicit free
function; alternatively move discovery into Rust SDK construction and expose its
cached result. Extract a pure endpoint parser in
`rs-sdk-trusted-context-provider` so the provider and bridge share one parser.

Define explicit-configuration precedence, HTTP status/timeout/failure fallback,
version/status filtering, string ownership, testnet's missing-port default (`1443`),
bracketed IPv6, and malformed/missing port behavior. Delete Kotlin and Swift host
fetch/parsing only after both consume the shared result; hosts must not fetch twice.
Replace the Kotlin test that currently pins the incorrect `443` default.

### S2 — Funding selection and account scope

This concerns DIP-17 Platform credit addresses and nonces, not Core UTXOs from C2.
Add separate account-scoped Rust composites for identity registration/top-up from
Platform addresses. Inputs are `(wallet_id, PlatformPayment account_index, target
credits)`. Rust enumerates hydrated and derived candidates, fetches authoritative
balances/nonces, applies deterministic selection and fee constraints, signs/submits,
and returns the result.

If multiple Platform Payment accounts exist, the account index is mandatory. Tests
cover only-the-chosen-account, fresh-restart hydrated candidates, exact target,
insufficient funds, and concurrent balance/nonce revalidation. Delete every
Kotlin/Swift pre-enumeration and greedy packing loop in the adopting slice.

### S3a — Token authorization and proposal evaluation

After C3, expose a versioned Rust decision result for action + token configuration +
actor context containing allowed/denied, a stable reason discriminant, and zero or
more authorization alternatives/group rules. Include every group-capable action,
especially `maxSupplyChangeRules`; do not collapse alternatives into one “required
key.” Define stable `repr` discriminants or versioned JSON plus typed host decoding
and owned-array/free rules.

### S3b — Direct-purchase quote

Expose a separate Rust quote result for schedule + amount containing selected
threshold, unit price, and full-domain `u64` total. Both apps render this result and
delete their local tier/price arithmetic. Rust remains authoritative at broadcast.

### S4 — Protocol constants and codecs

Expose versioned consensus identity-funding denomination sets separately from purely
presentational UI presets. Address validation returns typed family, network, payload,
and failure reason rather than `Bool`. Invitation validation already exists in
`platform_wallet_parse_invitation`; Android bridges that API rather than creating a
second codec. Remove hard-coded protocol tables and app-local Base58/Bech32 validators
only when the shared replacements are adopted.

## 8. Immediate example-app parity fixes

These are independent, low-risk fixes and do not wait for the larger shared APIs:

1. Add `DedicatedTransition.CREATE_DOCUMENT`, route Android `documentCreate` to the
   existing `CreateDocument` screen using selected contract/type, and remove the
   unused `documentFields` catalog input unless it is passed as a real prefill.
2. Include `maxSupplyChangeRules` in Kotlin and Swift pending-proposal discovery
   as an explicitly temporary compatibility fix; delete it when S3a replaces host
   discovery entirely.
3. Display immature Core balance in Swift WalletDetail and do not label a wallet
   containing only immature funds “Empty Wallet.”
4. Correct parity rows for document transitions, invitations, rescan, recovery, and
   contested-name discovery.

## 9. Executable parity manifest

Replace manually maintained totals with a checked-in manifest. Each capability has:

```yaml
id: invitations.reclaim
shared_apis:
  - platform_wallet_topup_identity_with_existing_asset_lock_signer
  - platform_wallet_resume_identity_with_existing_asset_lock_signer
required_persistence_capabilities:
  - atomic_changesets
  - asset_lock_funding_indices
  - invitations
  - wallet_restore
hosts:
  swift:
    sdk: supported
    example_app: supported
    restart: tested
    reason: null
  kotlin:
    sdk: unsupported
    example_app: unsupported
    restart: required
    reason: "P2 not implemented at the PR #3999 baseline"
verification:
  - host: swift
    kind: manual
    file: packages/swift-sdk/SwiftExampleApp/TEST_PLAN.md
    id: DP-19
```

Allowed host states are `supported`, `partial`, `unsupported`, and
`not-applicable`. A capability can be `supported` only when:

- all listed shared APIs are reachable, when the capability needs shared APIs;
- required persistence capabilities are registered;
- restart behavior is tested when value or durable state can exist;
- its automated verification exists and passes, or the manifest records a release
  manual/device gate.

`shared_apis` is optional because some capabilities are host-only. Restart state is
`required`, `tested`, or `not_applicable`, not a boolean. Verification kinds are
`unit`, `integration`, `device`, or `manual` and include a file, stable ID/test name,
and command where automated. CI validates schema, symbols/files/test IDs, reason
requirements, and generated counts; it does not pretend to prove runtime reachability
by static assertion. `PARITY.md` becomes generated prose or a thin index.

## 10. Implementation sequence

### Slice 0 — Spec and regression harness

- Land this reviewed spec.
- Add the parity-manifest schema/checker and initial manifest representing reality.
- Correct stale documentation without claiming missing features are complete,
  including obsolete Swift `core_wallet_send_to_addresses` test-plan references and
  shipped Swift invitation rows still marked as bridge-only.

### Slice 1a — Android address-index safety

- C1 only. No schema migration.

### Slice 1b — Atomic shared Core send

- C2 Rust/key-wallet reservation primitive, new opaque FFI result, both host
  adoptions, and old-ABI deprecation.

### Slice 1c — Lossless token domains

- C3 compatibility table, JNI raw-bit boundary, v5 unsigned token storage, and both
  host/domain tests. C3 gates S3a/S3b.

These are separate PRs/rollback units.

### Slice 2a — Serialized Android schema foundation

- Land/export v5 for C3 unsigned token storage.
- Reserve and land Room v6 for P1 FVK storage.
- Reserve and land Room v7 for P3 block position and transaction-account
  involvement.
- Reserve Room v8 for P2 invitation UI persistence.
- Export each schema; test each adjacent migration and v4→latest. Do not develop
  parallel conflicting migrations from the same schema version.
- Define a versioned, explicit backend-attested `PersistenceCapability` bitset and
  intersect/validate it against structurally required callback groups. Callback
  presence alone cannot attest semantic sub-capabilities: for example,
  `provider_transactions` shares the broad wallet-list restore callback but is valid
  only when the backend actually populates and frees its provider restore payload.
  Canonical v1 capabilities are `atomic_changesets`,
  `asset_lock_funding_indices`, `invitations`, `shielded_viewing_keys`,
  `provider_transactions`, `unsigned_token_storage`, `pending_contact_crypto`, and
  `wallet_restore`. `asset_lock_funding_indices` covers account registration and
  address-pool watermark persistence; `pending_contact_crypto` covers both durable
  queue additions and removals. These names are the public manifest/diagnostic
  namespace; source-level compatibility aliases do not create additional bits.
  Expose initialization diagnostics and add missing-capability preflight tests per
  feature, including a wallet-list callback present while provider capability
  remains absent.
- Retain `persists_durably()` only as a compatibility wrapper derived from
  `atomic_changesets + asset_lock_funding_indices + invitations`; new feature code
  checks its exact required capability set. Invitation creation additionally
  requires `wallet_restore`, because a committed voucher is not restart-safe unless
  its originating wallet and funding-index state can both be reconstructed.

### Slice 2b — Android viewing-key callbacks

- P1 behavior atop v6, including native instrumentation round trip.

### Slice 2c — Android provider restoration

- P3 behavior atop v7. This may remain `unsupported` in the manifest until an
  Android masternode consumer exists, but must not be claimed as parity.

### Slice 2d — Android identity recovery

- R1 tracked-lock listing plus registration/top-up resume. No DB migration.

### Slice 2e — Android SPV and DPNS queries

- R2 and R3 as independent commits/PRs. No DB migration.

Each vertical slice includes its JNI descriptor/symbol smoke and native Android
library build; JVM tests alone do not validate native binding.

### Slice 3 — Invitations

- P2 as one vertical feature using v8 and the shipped shared/iOS invitation
  semantics. R1 lands before invitation reclaim.
- Do not combine it with generic persistence cleanup; invitation broadcast ordering
  and durability gates must remain independently reviewable.

### Slice 4 — Shared-policy consolidation

- S1 endpoint discovery.
- S2 account-scoped funding selection.
- S3a proposal/authorization decisions.
- S3b purchase quote.
- S4 constants/codecs.

Each host implementation is deleted in the same slice that exposes and adopts its
shared replacement; do not leave two live paths.

### Slice 5 — Deferred durability

- P4 only after the per-identity queue relocation and shared cold-load restore exist.

### Release gates

PR #3999 release blockers are Slice 0, C1, C2, C3, P1, P3 if provider parity is
advertised, R1, and truthful manifest/docs. Invitation Android parity, R2/R3, and
S1-S4 may ship as explicitly `unsupported`/`partial` follow-ups unless product scope
requires them for the same release. Definition of done for the overall program does
not force every consolidation item into one unbounded merge gate.

## 11. Test matrix

| Risk | Shared Rust | JNI/Kotlin | Swift | Device/testnet |
| --- | --- | --- | --- | --- |
| Address index conflict | provider event fixture | handler restart/restore | existing semantic pin | Android restore smoke |
| Concurrent Core sends | barrier-forced same-UTXO race and reservation lifecycle | composite wrapper + JNI ownership | composite wrapper + C ownership | two live sends |
| `u64` boundary | ABI encode/decode | raw-bit JNI + Room/DAO/UI unsigned ordering | `UInt64` parity | Android JNI symbol smoke |
| Viewing-key restart | bind without seed | v5→v6 + callback/free round trip | existing callback test | Android seedless restart |
| Provider restore | payload decode/ownership | v6→v7, multiwallet membership, malformed bytes | existing restore | Android callback round trip |
| Asset-lock resume | tracked-lock state machine | process-death recovery | existing resume tests | funded testnet |
| Invitations | protocol/persistence ordering | v7→v8 + DP-12...19 | retain DP-12...19 | cross-platform claim |
| Discovery | port/IPv6 fixtures | no host parser remains | no host parser remains | testnet discovery |
| Token proposal rules | all action-rule variants | render typed result | render typed result | group co-sign |
| Deferred crypto | identity-first restore/fan-out | vtable + Room crash restart | vtable + SwiftData crash restart | locked-seed restart |

Required validation per slice:

- `cargo fmt --all -- --check` and targeted Rust tests;
- `cargo clippy` for changed Rust crates and all targets;
- `cargo test -p rs-unified-sdk-jni --lib`, Android native library build, Kotlin JVM
  tests under JDK 17, Room `MigrationTestHelper` adjacent and v4→latest tests, and
  instrumented callback round trips for P1/P3;
- Swift package tests and iOS framework build for Swift/FFI slices;
- cbindgen header regeneration/diff, C struct size/layout pins, Swift `MemoryLayout`
  pins for direct structs, and null/zero-count/double-free buffer tests;
- on-device external-function smokes for R1/R2/R3/C3 and resolver-handle teardown;
- `git diff --check`.

## 12. Compatibility and rollout

- Database migrations are additive and serialized as v5 unsigned token storage, v6
  FVK, v7 provider membership/position, and v8 identity-key derivation
  breadcrumbs (pending-repair durability, dashpay/platform#4060); the
  invitations migration shifts to v9. Do not destructively rewrite address
  identity columns.
- New Rust FFI functions are additive and new result PODs are opaque/versioned.
  Existing released split-builder entry points are deprecated before removal;
  existing struct layouts and JNI descriptors do not change silently. The
  unreleased Kotlin token source surface is corrected from signed `Long` to
  `ULong` in place as accounted in C3, while retaining identical raw `jlong`
  descriptors and a checked Java `BigInteger` adapter.
- Persistence exposes feature-specific capabilities. Registration reports missing
  callback sets at initialization where possible, and required feature APIs fail
  closed before broadcast.
- The parity manifest initially records known gaps. CI prevents regression but does
  not require all gaps to close in the first slice.

### Keystore rework divergences and convergences (dashpay/platform#4060)

Recorded in `sdk-parity-manifest.json`; rationale here:

- **`KeySecurityPolicy` + Keystore alias split (Kotlin-only, deliberate).**
  Android Keystore fixes authentication parameters at key generation, so the
  AUTH_GATED/DEVICE_BOUND policies require distinct aliases; iOS Keychain has
  no per-alias auth-parameter analog (item access control covers the same
  ground), so the manifest marks Swift `not-applicable` — no Swift port is
  planned. The lockless-device degradation (AUTH_GATED writes redirect to the
  DEVICE_BOUND alias, surfaced via `effectiveKeySecurityPolicy`) is likewise
  Android-specific: KeyMint rejects gated key generation without a secure
  lock screen.
- **Platform-wallet code 98 is now CONVERGENT.** Kotlin previously collapsed
  the blanket Option-miss code into the top-level `DashSdkError.NotFound`
  while Swift kept it in the wallet family; Kotlin now maps 98 to
  `DashSdkError.PlatformWallet.NotFound` (BREAKING for hosts that caught the
  top-level type from platform-wallet operations).
- **Durable pending-repair surface (Kotlin-only, port candidate).**
  `pendingIdentityKeys` + forced/verified `repairIdentityKey` + the Room v8
  derivation breadcrumbs have no Swift counterpart; the manifest records the
  gap as `unsupported` for Swift.
- **Structured `SigningKeyUnavailable` discriminator (both hosts).** The
  signer completion carries a typed `error_code` (rs-sdk-ffi
  `DashSDKSignerErrorCode`), restored as platform-wallet code 31 on both
  hosts. The Rust-internal segment rides the machine prefix
  `signer_error:key_unavailable: ` through `ProtocolError::Generic` (a typed
  rs-dpp variant was rejected for serialization blast radius — accepted
  residual). The Kotlin `MESSAGE_MARKER` text sniff survives ONLY as a
  deprecated old-native fallback; remove it (and the marker's matcher role)
  in the next minor release once native artifacts are guaranteed current.

## 13. Explicitly out of scope

- Redesigning DIP-13/DIP-15 invitation wire formats.
- Simultaneous multi-account DashPay contacts; that remains governed by
  `MULTI_ACCOUNT_SPEC.md` and its product gate.
- Replacing JNI or C FFI with a new binding generator.
- Making Room and SwiftData schemas structurally identical.
- Treating example-app visual layout differences as parity failures when capability,
  accessibility contract, and behavior are equivalent.

## 14. Definition of done

This effort is complete when:

1. C1-C3 have regression coverage and both host SDKs use the safe/shared paths.
2. Android registers all persistence capabilities required by features it advertises.
3. Android can resume every funded identity/invitation operation already supported
   by iOS.
4. S1-S4 have one live shared implementation with both host copies removed.
5. Every supported capability is represented by the executable manifest and named
   tests; no manual parity count contradicts runtime capability.
6. Cross-platform invitation claim and concurrent-send device smoke tests pass.
