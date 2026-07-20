# DashPay — Kotlin/Android Migration Spec

**Historical baseline:** This document preserves the reviewed pre-migration plan
and its then-current 88/90 inventory. It is not the current open-work tracker:
subsequent implementation changed the contact-crypto persistence boundary,
shipped invitation support on iOS, and closed the cited transition FFI gaps. See
[`docs/sdk/PARITY_SUMMARY.md`](../sdk/PARITY_SUMMARY.md) and
[`docs/sdk/sdk-parity-manifest.json`](../sdk/sdk-parity-manifest.json) for current
capability status.

Port the complete DashPay feature (as shipped for iOS in PR #3841, merged to
`v4.1-dev` 2026-07-06) to the Kotlin/Android SDK and KotlinExampleApp
(PR #3999, branch `feat/kotlin-sdk-and-example-app`).

Status: **v2 — post five-lens review** (feasibility, scope, security,
adversarial, domain-fit); all must-fixes folded in.
Reference implementation: `packages/swift-sdk` + SwiftExampleApp.
Feature spec: `docs/dashpay/SPEC.md` (+ companion specs in `docs/dashpay/`).

---

## 1. Problem

The Kotlin SDK (PR #3999) is a one-for-one Android port of SwiftExampleApp,
snapshotted **before** PR #3841 landed. #3841 completed DashPay on iOS:

- Replaced the utilitarian `FriendsView` (which the Kotlin `FriendsScreen`
  mirrors — now a port of a **deleted** Swift view) with a first-class
  **DashPay tab**: 10 views, ~3.8k LOC (`Views/DashPay/`).
- Added a recurring **DashPay background sync** service
  (`platform_wallet_manager_dashpay_sync_*`, 7 FFI fns).
- Added **payment history**, **cached contact profiles**, **contactInfo**
  (alias/note/hidden, cross-device), **ignore tombstones**, **seedless
  unlock + deferred contact-crypto drain**, and **DIP-15 auto-accept QR**.
- Net: 23 new C exports in `rs-platform-wallet-ffi`; 6 old exports removed
  (incl. the `*reject_contact_request` pair — reject became ignore).

Today the Kotlin stack has: 3 of 5 DashPay Room entities (field-exact with
SwiftData), the send/accept/ignore/sync contact-request pipeline bridged
(17 JNI exports in `tokens.rs`), and one compressed `FriendsScreen`. It
lacks: profile/contactInfo writes, payments, the contact-profile cache, the
sync service, QR, seedless unlock, wallet-scoped DPNS search, and the
DashPay tab. `PARITY.md` still claims 88/90 ported against the stale
pre-#3841 view list.

**Compatibility baseline:** the existing 17 exports were reconciled with
the #3841 Rust in commit `2298a2059f` (reject→ignore, `coreSignerHandle`
threading, contacts-vtable ignored-sender deltas + contactInfo metadata
fields, Room v1→v2). That reconciliation was verified by compile + 3
Robolectric tests only — no runtime exercise of the Android
send/accept/ignore path against a network has been recorded since the
merge. K1 therefore opens with a runtime revalidation gate (§4).

**Goal:** feature parity with iOS DashPay per the parity doctrine
(`kotlin-sdk/CLAUDE.md`): cite Swift sources in KDoc, reuse iOS
accessibility identifiers as Compose `testTag`s, keep all orchestration in
Rust.

## 2. What does NOT need porting

All DashPay *logic* is shared Rust, already compiled into this branch and
consumed by iOS through the same C ABI:

- Contact-request crypto (ECDH, AES-256-CBC, DIP-15 69-byte compact xpub),
  DIP-14/15 derivation, accountReference — `rs-platform-wallet` +
  `platform-encryption` + `key-wallet`.
- Accept → reciprocal request → `register_external_contact_account`
  (friendship address derivation) — automatic inside Rust.
- The recurring sync sweep (`manager/dashpay_sync.rs`), seed binding
  verification, deferred contact-crypto queue, auto-accept QR
  build/parse/proof.
- The bundled DashPay contract.

Kotlin re-implements **none** of this. Per doctrine, no JNI function may
stitch Rust calls together — any composite gap found during the port goes
into `rs-platform-wallet-ffi` as its own reviewed change (none are
currently known to be needed; iOS ships on the existing exports).

**Deliberately not bridged** (zero non-README callers in the Swift app —
the app reads this data from persisted rows, not handles; verified
function-by-function during review):

- The 8 `contact_request_*` field getters + `contact_request_create`.
- The 14 `established_contact_*` accessors +
  `managed_identity_get_established_contact` /
  `_get_sent_contact_request` — `ContactDetailView` reads alias/note/
  hidden/paymentChannelBroken off `PersistentDashpayContactRequest` rows
  and writes via `set_dashpay_contact_info_with_signer`.
- `managed_identity_is_contact_established`,
  `platform_wallet_pubkey_hash_from_private_key` (no app consumers),
  and the `managed_identity`-level send/accept/ignore variants (Kotlin
  uses the `platform_wallet_*` composites, as iOS does).

Consequence: **no new handle-wrapper types.** The existing
`ContactRequestRef` / `EstablishedContactRef` (`tokens/Dashpay.kt`) stay
as-is for the accept path; new screens are driven by Room Flows and
snapshot data classes. Rule: never retain a native handle in Compose
composition — snapshot fields at the JNI boundary (a Cleaner can free a
handle mid-read otherwise).

## 3. Approach

Three milestones, re-cut after review so that **every bridged function
lands in the milestone that first consumes and tests it** (the original
bottom-up "bridge everything first" cut concentrated all marshaling
defects at final UAT — rejected).

### Alternatives rejected

- **Re-orchestrating DashPay flows in Kotlin**: forbidden by doctrine;
  Swift ships proof that single-call composites suffice.
- **Keeping `FriendsScreen` and bolting features onto it**: its Swift
  counterpart was deleted; keeping a dead view's port violates parity.
- **uniffi / JNA instead of hand-rolled JNI**: the crate has 160
  hand-rolled exports with established support patterns (`support::guard`,
  handle-as-jlong, GlobalRef callbacks); mixing binding generators adds
  toolchain cost for no capability gain.
- **Bridging the full ~47-function FFI sweep**: 24 of them have no
  consumer anywhere in the reference app (see §2); bridging them would
  add dead surface + a speculative handle-wrapper abstraction.

## 4. Work plan

New JNI exports go in a new `rs-unified-sdk-jni/src/dashpay.rs` +
`ffi/DashpayNative.kt`; the 17 existing DashPay exports stay in
`tokens.rs` (moving them is a follow-up refactor). Array-free FFI
counterparts (`dashpay_payment_array_free`, `dpns_search_results_free`,
`platform_wallet_manager_free_account_balances`) are consumed Rust-side
inside the JNI wrappers, as the existing exports do.

⚠ Lockstep rule: extending the identity-entry persist callback changes the
`onPersistIdentityUpsert` JNI method descriptor (hand-written string,
`persistence.rs:866-893`) — the Rust descriptor and the Kotlin
`NativePersistenceBridge` signature must change in the same commit or
every identity persist fails at runtime.

### Milestone K1 — Persistence completion + the read surface it can prove

**Entry gate:** one runtime revalidation of the existing 17-export
pipeline — the FriendsScreen send→accept→ignore flow against testnet
(`-Ptestnet=true`), or its instrumented equivalent — before any new
bridging. Also the 15-minute PARITY.md interim fix: drop the stale
`FriendsView.swift` row claims, mark the DashPay section
"in migration — see KOTLIN_MIGRATION_SPEC.md".

**Bridge (6 exports):**

| Group | Functions |
|---|---|
| Payments | `managed_identity_get_dashpay_payments` |
| Profile reads | `platform_wallet_get_contact_profile`, `managed_identity_get_dashpay_profile`, `managed_identity_get_dashpay_sync_state` |
| DPNS search | `platform_wallet_search_dpns_names` (wallet-scoped; the existing `QueriesNative.dpnsSearch` wraps the *SDK-scoped* `dash_sdk_dpns_search` — a different call path; AddContactScreen must use the wallet-scoped one for parity) |
| Account balances | `platform_wallet_manager_get_account_balances` (DashPayTabView's per-account balance display; not DashPay-prefixed, easy to miss) |

**Persistence** (mirrors `PersistentDashpayContactProfile.swift` /
`PersistentDashpayPayment.swift`):

- New Room entities `DashpayContactProfileEntity`
  (`(networkRaw, ownerIdentityId, contactIdentityId)` unique, `checkedAtMs`
  backoff) and `DashpayPaymentEntity`
  (`(networkRaw, ownerIdentityId, txid)` unique) + DAOs + Flows;
  `DashDatabase` v2→v3 additive migration, exported schema `3.json`.
- **Persist direction — contact profiles:** extend identity-entry
  marshaling to carry `IdentityEntryFFI.contact_profiles` (slot exists,
  currently skipped at `persistence.rs:825-895`). **Tombstone semantics
  are load-bearing:** the projection emits `is_present == false` rows that
  mean DELETE the persisted row (`identity_persistence.rs:600-608`) — an
  upsert-only implementation compiles, passes upsert tests, and leaves
  stale contact names/avatars forever. Required: `is_present=false` → DAO
  delete (with test) + `checkedAtMs` round-trip fidelity test. (Note: the
  outer doc comment at `identity_persistence.rs:146` contradicts the
  projection code and should be corrected in passing.)
- **Persist direction — payments:** pull-based, mirroring iOS exactly.
  **Invariant:** payment rows reach Room *only* via the
  `refreshDashPayPayments` equivalent (FFI read → Room upsert);
  `dashpay_sync_now` reconciles payments **in-memory without persisting**
  (identity persist skips payments, `identity_persistence.rs:37-39`).
  Android process death is aggressive, so K3's send flow must call
  refresh-after-send, and the K1 test suite pins the invariant.
- **Restore direction:** populate the null-stubbed `contact_profiles` /
  `payments` arrays in `rs-unified-sdk-jni/src/persistence.rs`
  (staging comment :1476-1480, stubs :1848-1853, free-path :2049-2051),
  mirroring the Swift restore blocks
  (`PlatformWalletPersistenceHandler.swift` ~4918, ~4978).

**Gate (re-tiered after review):** the persist→wipe→restore→re-read
round-trip runs as an **instrumented test** (extend
`sdk/src/androidTest/.../WalletManagerRoundTripTest.kt`, which already
drives the real native lib on the CI emulator) — payload includes payments
with memo/direction/status, a contact-profile `is_present=false` tombstone,
and ignored senders; the new K1 getters read back restore-injected fixtures
and assert field equality. JVM/Robolectric tests cover handler↔Room mapping
only (they never load the native lib, so they cannot see marshaling bugs —
this was the original spec's top-listed risk guarded by the wrong tier).

### Milestone K2 — Sync service, seedless unlock, writes

**Pre-req (security must-fix): mnemonic-handling discipline.** The
existing Kotlin resolver materializes the mnemonic as an immutable JVM
`String` (`WalletStorage.retrieveMnemonic` does `decodeToString()` and
scrubs only the ByteArray; `MnemonicResolverAndPersister.kt:36-42` returns
the String to native). iOS never creates a string-shaped copy: it keeps
XOR-masked UTF-8 bytes (`MaskedMnemonicUTF8`) and writes into Rust's
out-buffer, scrubbing on every access. K2 multiplies resolver calls (the
drain runs it once per queued entry), so **before** wiring the automatic
drain: port the out-buffer + masked-bytes discipline to `WalletStorage.kt`
/ `MnemonicResolverAndPersister.kt` / `mnemonic.rs` (whose "same residual
exposure as iOS" comment is false and must be corrected), and zeroize the
`mnemonic_str`/`mnemonic_c` copies in `signer.rs:342-422` (currently only
`sig_buf` is scrubbed).

**Bridge (13 exports):** the 7 `platform_wallet_manager_dashpay_sync_*`
fns; the seedless trio `platform_wallet_verify_seed_binds_to_wallet`,
`platform_wallet_pending_contact_crypto_count`,
`platform_wallet_drain_pending_contact_crypto` (takes **both** a
`SignerHandle` and a `MnemonicResolverHandle` —
`rs-platform-wallet-ffi/src/dashpay.rs:757-761`);
`platform_wallet_create_or_update_dashpay_profile_with_signer`,
`platform_wallet_set_dashpay_contact_info_with_signer`;
`dash_sdk_resolver_supports_key_type` (rs-sdk-ffi; consumed by the
production signer — Swift `KeychainSigner.swift`).

**`DashpaySyncService`** (`sdk/services/`, mirroring
`PlatformWalletManagerDashPaySync.swift`):

- Owned by the `PlatformWalletManager` instance; started when platform
  wallets are present (after load / on the rebind path), stopped and
  disposed on the `WalletManagerStore` manager swap. **Not gated on
  process lifecycle** — iOS keeps the sweep running while backgrounded
  (the OS suspends the process; Android freezes/kills similarly under
  modern app-standby), and this matches the manager-owned ownership
  doctrine. (The v1 spec's "mirrors scenePhase" claim was wrong — Swift
  drives start/stop off wallet-presence/rebind `.onChange`, not
  scenePhase. No `lifecycle-process` dependency needed.)
- `isSyncing` / `lastSync` / `pendingAccountBuilds` exposed as `StateFlow`,
  updated by a **1 Hz poll with change-gated assignment and stale-key
  pruning** — pinned now, not "verify later": that is exactly how iOS does
  it (`PlatformWalletManager.swift:1140-1186`; the comment at :1133-1139
  documents why naive re-assignment burned CPU). Natural home: the
  `SpvProgressPublisher` pattern.

**Seedless unlock — invocation topology (domain must-fix; the API method
alone is not the feature):**

- `unlockWalletFromKeystore(walletId)` = scoped verify → drain, and is
  called **automatically, per restored wallet, inside
  `loadPersistedWallets`**, best-effort/never-throwing (mirrors
  `PlatformWalletManager.swift:477-506`; Kotlin's
  `PlatformWalletManager.kt:457` currently documents the absence). This is
  load-bearing: the deferred contact-crypto queue is **in-memory by
  design** (no persisted table on either platform — do not "fix" that) and
  recovery is self-healing only if every launch runs
  load → unlock (verify+drain) → sweep. Banner-triggered-only unlock would
  leave contacts never finishing establishment after process restart.
- **Seed-mismatch contract:** Rust `SeedMismatch` surfaces as
  `ErrorInvalidParameter` (`rs-platform-wallet-ffi/src/dashpay.rs:960-965`);
  like Swift, Kotlin disambiguates *only* by scoping the catch to the
  verify call (the JNI error code arrives as
  `code + PWFFI_CODE_OFFSET`). Publish per-wallet
  `draining` / `seedMismatch` / `pendingAccountBuilds` as `StateFlow`
  (Swift: `dashPayUnlockStatus`).
- **Re-entrancy guard:** a second unlock while `draining == true` returns
  immediately (Swift :622-624) — load-time auto-drain and a user banner
  tap must not double-run the ECDH work.
- **Biometric interaction (Android-only failure mode):** Kotlin's
  identity-key Keystore alias is auth-gated (30 s validity +
  `BiometricGate`) — *stricter than iOS*, whose identity keys are not
  auth-gated at all. The reciprocal-accept signing inside a background
  drain can therefore throw on an expired auth window with no Activity to
  re-prompt. Required behavior: catch, leave the entry queued (the sweep
  self-heals), reflect it in the unlock status; instrumented test for the
  auth-expired path.
- **Breadcrumb backfill — explicitly not ported.** Swift's unlock also
  schedules `scheduleBackfillIdentityKeyBreadcrumbs`, an iOS-legacy
  Keychain healing step for pre-breadcrumb installs. The Kotlin SDK is new
  — every identity key it has ever created is breadcrumbed at creation —
  so there is nothing to heal. Recorded here so its absence is a decision,
  not an oversight.

**Gate:** unit tests + instrumented sync-service lifecycle tests
(start/stop/isRunning; manager-swap disposal; double-start), unlock state
machine with a wrong-seed fixture (seedMismatch path) and the
auth-expired-drain path; write paths against testnet behind
`-Ptestnet=true`.

### Milestone K3 — DashPay tab UI + parity bookkeeping

**Bridge (2 exports):** `platform_wallet_build_auto_accept_qr`,
`platform_wallet_send_contact_request_from_qr` (first consumed here).

Navigation restructure (mirrors Swift `ContentView.swift`):

- `RootTab`: `SYNC, WALLETS, IDENTITIES, DASHPAY, SETTINGS` — **Contracts
  tab is demoted into Settings** (`ContractsHome` becomes an entry in the
  Settings screen's Platform section, as on iOS).
- Retire `FriendsScreen` + its route; entry points repoint at the DashPay
  tab/contact flows.
- Reset the DashPay tab's identity-picker selection on network switch —
  retained Compose state would otherwise query a wallet absent on the new
  network-locked manager.

Port the 10 Swift views (Compose screens under `ui/dashpay/`, one file per
Swift file, testTags = iOS accessibility identifiers, screens driven by
Room Flows / snapshot data classes — never by retained native handles):

| Swift (`Views/DashPay/`) | Compose target | Notes |
|---|---|---|
| DashPayTabView (909) | `DashPayTabScreen` | identity picker, per-account balances, pull-to-refresh → `syncNow`, unlock banner (reads unlock-status Flow), sub-sheet navigation |
| ContactsView (299) | `ContactsScreen` | Room Flow over established contacts (both-direction join) |
| ContactRequestsView (391) | `ContactRequestsScreen` | incoming accept/ignore + outgoing pending |
| AddContactView (497) | `AddContactScreen` | wallet-scoped DPNS prefix search (300 ms debounce), raw id, QR entry |
| ContactDetailView (561) | `ContactDetailScreen` | payment history (refresh→Room), alias/note editors **surfacing the `ContactInfoPublishOutcome`** — `DeferredUntilTwoContacts` means local-only until ≥2 established contacts and the UI must say so (parity), `SkippedWatchOnly` likewise; hide; send-payment entry |
| SendDashPayPaymentSheet (386) | `SendDashPayPaymentSheet` | amount/memo → `sendDashPayPayment` → txid, then **refresh-after-send** (payments-durability invariant, §K1) |
| DashPayProfileView (188) | `DashPayProfileScreen` | own profile display/edit + auto-accept QR render (ZXing — already a dependency) |
| IgnoredContactsView (181) | `IgnoredContactsScreen` | unignore |
| HiddenContactsView (240) | `HiddenContactsScreen` | unhide |
| DashPayContactMeta (183) | `DashPayContactMeta.kt` | meta store (UserDefaults → SharedPreferences/DataStore; plaintext is parity — iOS documents UserDefaults as "the honest backing" for device-local data), display-name precedence, avatar composable |

QR scan reuses the already-ported `QrScannerScreen`; the auto-accept
scan-to-send path calls `sendContactRequestFromQR`.

**New dependency:** Coil (`coil-compose`) for avatar loading — not
currently in `libs.versions.toml`; flagged here because it is the plan's
only new third-party dependency. (ZXing is already present.)

Parity bookkeeping: full `PARITY.md` DashPay rewrite — a `Views/DashPay/`
section with one row per view, corrected totals (interim stale-claims fix
already landed in K1).

**Gate:** `:app:assembleDebug` + Compose UI tests mirroring
`DashPayTabUITests.swift`; manual UAT next to the iOS simulator per
`QA_TESTCASES_SPEC.md` flows, including the end-to-end
send→accept→pay testnet run.

## 5. Interfaces & data flow (summary)

```
Compose UI ── StateFlow/Room Flow ── PlatformWalletManager / Dashpay.kt
      │                                     │ (thin, marshal-only)
      │                               DashpayNative.kt (external fun)
      │                                     │ JNI
      ▼                               rs-unified-sdk-jni/src/dashpay.rs
 Room (Dashpay* entities)                   │ rlib call
      ▲                               rs-platform-wallet-ffi (C ABI)
      │ persistence callbacks               │
      └── NativePersistenceBridge ◄── rs-platform-wallet (all logic)
```

- Writes require two callback handles: identity signer (`signer.rs`) and
  mnemonic resolver (`mnemonic.rs`) — both exist; DashPay adds no new
  callback *types*, only new call sites. Handles are kept strongly
  referenced for the duration of each call (GC hazard).
- Contact profiles ride the identity persist/restore callback path
  (with tombstone-delete semantics); payments are pull-persisted and
  array-restored. The deferred contact-crypto queue is deliberately
  not persisted (in-memory + sweep self-heal, both platforms).
- Cold-start contract: `loadPersistedWallets` → per-wallet best-effort
  unlock (verify → drain) → sync service start → recurring sweep.
- Threading: FFI calls on `Dispatchers.IO`; Rust→Kotlin callbacks attach
  as JVM daemon threads; persistence-handler read-modify-writes run in
  Room transactions (callback threads race UI-triggered refreshes
  otherwise).

## 6. Failure modes / risks

- **Restore-path marshaling bugs** corrupt Rust wallet state on load.
  Guard: the K1 instrumented round-trip (real native lib) — JVM tests
  cannot see this code.
- **Contact-profile staleness:** upsert-only persist misses tombstones →
  stale names/avatars forever. Guard: `is_present=false` delete test (K1).
- **Payment loss on process death:** `syncNow` does not persist payments.
  Guard: refresh-after-send + kill/relaunch test (K1/K3).
- **Never-draining wallets:** unlock not wired into load → contacts stuck
  pending after every restart. Guard: unlock topology spec (§K2) +
  restore→unlock instrumented test.
- **Background drain vs biometric gate:** auth-expired signing during
  drain must requeue, not fail silently. Guard: auth-expired test (K2).
- **GC vs callback lifetime** during long drains: strong refs on
  signer/resolver bridges per call site; drain stress test.
- **Sync-service leak across network switch:** manager-owned lifecycle,
  disposed on `WalletManagerStore` swap; double-start test. UI-side:
  identity-picker reset on network change.
- **JNI descriptor lockstep** on the identity persist callback (§4 note):
  Rust descriptor + Kotlin signature in one commit.
- **Room migration:** additive-only v3; migration test against `2.json`.
- **Build env:** exFAT gotcha (`build_android.sh` sparse image), NDK r28+,
  16 KB alignment — K1 ends with `./build_android.sh --verify` passing.

## 7. Test plan

1. **Instrumented (`connectedDebugAndroidTest`, CI emulator)** — the
   load-bearing tier: extend `WalletManagerRoundTripTest` with the DashPay
   persist→wipe→restore→re-read round-trip (payments incl. memo/direction/
   status, contact-profile tombstone, ignored senders); K1 getters read
   restore-injected fixtures; K2 sync-service lifecycle, unlock
   state machine (wrong-seed → seedMismatch; auth-expired drain).
2. **JVM unit (`:sdk:testDebugUnitTest`)** — handler↔Room mapping only
   (explicitly *not* the marshaling tier): contact-profile
   upsert/delete/backoff, payment row mapping, Room v2→v3 migration.
3. **Compose UI tests** — port `DashPayTabUITests.swift` flows using the
   shared testTags (tab presence, add-contact form, requests accept path
   with a fake bridge).
4. **Testnet opt-in (`-Ptestnet=true`)** — K1 entry gate: existing
   FriendsScreen send→accept→ignore revalidation. K3 exit: end-to-end
   send→accept→pay between two fixture identities, mirroring iOS UAT.
5. **CI** — existing `kotlin-sdk-build.yml` runs tiers 1–3.

## 8. Out of scope

- Invitations (SPEC.md Milestone 5) — not implemented on iOS either.
- The 24 unconsumed FFI functions listed in §2 (and any new handle-wrapper
  types for them).
- `managed_identity_get_contested_dpns_names` — its consumers
  (SelectMainName / WalletMemoryExplorer / IdentityDetail) are outside
  `Views/DashPay/`; it belongs to the existing non-DashPay PARITY-partial
  bucket, not this migration.
- Identity-key breadcrumb backfill (justified in §K2 — no pre-breadcrumb
  Android installs can exist).
- Migrating the 17 pre-existing DashPay JNI exports out of `tokens.rs`.
- The 5 non-DashPay `TransitionDetailView` FFI gaps and other PARITY
  "partial" items.
- Any change to Rust crates other than `rs-unified-sdk-jni` (a genuine
  composite gap, if found, becomes its own reviewed `rs-platform-wallet-ffi`
  change).

## 9. Decisions taken in this spec (previously open)

- **One PR per milestone** — the milestones carry independent gates by
  design; review units should match.
- **Sync lifecycle: manager-owned, not process-lifecycle-gated** (§K2).
- **Poll (1 Hz, change-gated), not events, for sync/unlock status** (§K2).
- **Coil added** as the single new dependency (§K3).

## 10. Open questions (for Ivan)

1. **Branch/PR strategy:** land the K-milestones as stacked PRs on top of
   `feat/kotlin-sdk-and-example-app` (PR #3999 is already ~50k insertions),
   or fold into #3999? Recommendation: **stacked PRs**.
2. **Tab restructure confirmation:** mirroring Swift means demoting the
   Contracts tab into Settings on Android too. Confirm parity wins over
   Android-specific navigation taste.
