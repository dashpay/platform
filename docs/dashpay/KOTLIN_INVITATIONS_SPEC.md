# DashPay Invitations (DIP-13 3') — Kotlin/Android Port Spec

Port the shipped iOS invitation feature (create + claim + reclaim + sent-invitations
persistence, PR #4041, `docs/dashpay/DIP15_INVITATIONS_SPEC.md`) to the Kotlin SDK and
KotlinExampleApp, on branch `feat/kotlin-dashpay-invitations` (base `v4.1-dev`).

Status: **v3 — synced with owner (2026-07-22), all open questions resolved (§10);
slices 1–5 implemented** (JVM + cargo gates green; instrumented + funded-testnet
QA are the remaining environment-bound gates). (v2 was the post-review draft.)
Reference implementation: `packages/swift-sdk` + SwiftExampleApp (source of truth per the
parity doctrine in `packages/kotlin-sdk/CLAUDE.md`).
Feature behavior spec: `docs/dashpay/DIP15_INVITATIONS_SPEC.md` §0/§0A/§0B (as-built truth).

---

## 1. Problem

The Kotlin DashPay migration (K1–K3, `KOTLIN_MIGRATION_SPEC.md`) ported all 10 DashPay
screens and declared invitations out of scope (§8) because iOS hadn't shipped them yet.
iOS has since shipped the full feature. Today the Kotlin stack is deliberately
**fail-closed**: `rs-unified-sdk-jni/src/persistence.rs:174` sets
`on_persist_invitations_fn: None`, so the backend never attests the `INVITATIONS`
capability and `create_invitation`'s up-front
`persistence_capabilities().contains(INVITATION_CREATION)` gate refuses to run on
Android — rather than mint a voucher whose one-time bearer key could be re-exported
after a restart (the defect class fixed by 55937e15c1 on iOS).

Every layer is pre-seamed for this port:
- All three invitation FFI entry points (`create`/`claim`/`parse`) plus the two reclaim
  exports already exist in `rs-platform-wallet-ffi` and are consumed by iOS through the
  same C ABI.
- The JNI natives for resume/top-up already declare a `consumeInvitationVoucher`
  parameter (currently guard-rejected when `true`); the Kotlin *public* wrappers hardcode
  `false` and structurally gate invitation locks out (see §3.1 — they need new variants,
  not a flag flip).
- Room schema v8 is reserved for invitations (`PARITY.md:49`; DB is at v7).
- The handler already attests 3 of the 4 capability bits in `INVITATION_CREATION`
  (`ATOMIC_CHANGESETS 0x01 | ASSET_LOCK_FUNDING_INDICES 0x04 | WALLET_RESTORE 0x80`);
  only `INVITATIONS (0x02)` and its callback are missing.

**Goal:** feature parity with iOS invitations — same flows, same persisted shape, same
crash-safety semantics, same testTags — with **zero changes to `rs-platform-wallet` and
`rs-platform-wallet-ffi`** (shared logic stays shared; the port is bindings + persistence
+ UI only).

## 2. What does NOT need porting (all shared Rust, already on this branch)

- Link codec (`dashpay://invite?du=…&assetlocktx=…&pk=…&islock=…`, emit-strict /
  parse-lenient, applink host, WIF + byte-order leniency) — `crypto/invitation.rs`.
- Create orchestration: amount caps (`MIN=300_000` / `MAX=5_000_000` duffs), capability
  gate, funding-index persist+flush **before broadcast** (abort on failure), invitation
  row persisted **after broadcast, before the proof wait** (hard error if it fails),
  Instant-proof requirement, path-gated voucher-key export (`9'/coin'/5'/3'/idx'`),
  key scrubbing — `network/invitation.rs`.
- Claim orchestration: claim-by-fetch with bounded retry + byte-reversed retry,
  credit-output selection by script match, Instant/Chain proof reconstruction,
  raw-key identity registration — `network/invitation.rs`.
- Reclaim authorization: `AssetLockFunding::FromExistingAssetLock` +
  `authorized_invitation_reclaim` (invitation-typed locks are consumable **only** with
  `consume_invitation_voucher = true`, and only into register/top-up; every generic
  path refuses them regardless of the flag) — `wallet/asset_lock/orchestration.rs:493,511`.
- The persistence wire type `InvitationEntryFFI` (repr(C), ABI-pinned 64 B, all-POD)
  and the `on_persist_invitations_fn` callback slot + `INVITATIONS` capability bit —
  `rs-platform-wallet-ffi/src/invitation_persistence.rs`, `persistence.rs:655-664,822`.

Kotlin re-implements **none** of this. Per doctrine, no JNI function stitches Rust calls
together; each new export is a thin marshaler over exactly one existing C-ABI entry point.

## 3. Design decisions

1. **Reclaim: forward the flag in JNI + add outpoint-taking reclaim wrappers.**
   Two layers, two different changes:
   - *JNI*: the resume/top-up bindings (`identity.rs:496` → hardcoded `false` at `:552`,
     guard at `:524`; `credits.rs:286` → `:331`, `:311`) forward the already-declared
     `jboolean` verbatim and drop the `generic_asset_lock_recovery_allowed` call — an
     interim fail-closed measure pending this port. Rust core enforces the real policy
     independently (verified: `authorized_invitation_reclaim` requires
     invitation-typed lock ∧ flag ∧ register/top-up target; all other
     `FromExistingAssetLock` constructors hardcode `false` Rust-side), and Swift's
     wrappers forward the identical boolean with no extra guard — this reproduces the
     already-reviewed iOS trust model, not a weaker one.
   - *Kotlin wrappers*: the existing public `resumeWithExistingAssetLock` /
     `resumeTopUpWithExistingAssetLock` **cannot be reused** — they hardcode `false`
     (`IdentityRegistration.kt:158`, `IdentityCredits.kt:115`), `require()` non-invitation
     funding types, and `TrackedAssetLock.FundingType` has no `INVITATION(3)` (unknown
     types are silently dropped at `TrackedAssetLock.kt:62`). Add two **outpoint-taking
     reclaim variants** mirroring Swift (`resumeIdentityWithAssetLock:3939`,
     `resumeTopUpWithAssetLock:4128`): raw txid+vout + `consumeInvitationVoucher`, no
     funding-type gate, freely chosen `identityIndex`; each still one FFI call — no
     doctrine violation. Generic-recovery wrappers and all existing call sites stay
     exactly as they are.
   *Alternative rejected:* adding `FundingType.INVITATION` and relaxing the three
   `require()` gates on the generic-recovery wrappers — that weakens crash-recovery
   invariants shared by non-invitation flows to serve one caller.
2. **Status transitions are client-written, exactly as on iOS.** Rust emits only
   `Created` rows (create is the sole `InvitationChangeSet` emitter today). `Claimed`
   and `Reclaimed` are written locally by the app via the reclaim-outcome classifier.
   Room is the UI's source of truth; there is **no Rust→Kotlin rehydrate** (a Room wipe
   loses list visibility only — never funds; `funding_index` re-derives the key).
3. **Claim entry = paste + QR scan first-class, deep link additionally** (decided,
   §10.2). Add a `dashpay://invite` `VIEW` intent-filter routing to the claim sheet,
   mirroring `SwiftExampleAppApp.swift:148`, **plus an intent-filter for the legacy
   AppsFlyer host `https://invitations.dashpay.io/applink`** (the form production
   dashwallet-iOS emits; our parser already accepts it) — unverified until the domain
   serves `assetlinks.json` (dashpay/platform#4096), so it participates in the app
   chooser rather than auto-opening; upgradeable to a verified App Link when #4096
   lands. Two further deliberate deviations from iOS:
   - **Walletless parking (iOS behavior is a bug, not parity):** iOS clears
     `pendingInviteURL` before its no-wallet guard returns
     (`DashPayTabView.swift:149-153`), silently discarding the link — on Android the
     intent-filter's headline scenario *is* a fresh install tapping an invite. Kotlin
     parks the pending URI until a wallet exists (cleared only when the claim sheet is
     actually seeded) and shows "create a wallet to claim this invitation". Flag the
     drop as an upstream iOS bug to fix separately.
   - **Honest Android framing:** a custom scheme can't use App Links auto-verify; any
     app may register the same filter. Android shows a chooser on collision (per-tap
     visibility iOS doesn't have), **but** one "Always" tap for a malicious app silently
     routes every future invite link to it. Documented as an Android-specific
     persistence risk, not "the same caveat iOS documents".
   The mid-claim deferral gate (analog of `invitationClaimInFlight`) is ported as-is.
4. **Persistence writes go through the round buffer.** On FFI hosts the durable boundary
   is `onChangesetEnd`'s single `withTransaction` replay of the per-wallet
   `ChangesetBuffer` (`PlatformWalletPersistenceHandler.kt:298-353`); `onFlush` stays the
   inherited base-class no-op (verified: Rust's `flush()` after a successful `store()`
   round is a post-commit bookkeeping notification — the round's commit result is
   honored, not advisory). Hard rule: the invitation handler **stages via `stage {}`
   into the round buffer; never writes in its own transaction and never defers past the
   round** (no `launch`/`post`) — an immediate standalone write would break round
   atomicity, a deferred one would break durability.
5. **One PR, sliced commits** (§5 order). The feature is cohesive; the slices carry
   independent compile/test gates. (If review load demands, slice 4 can split
   screens+nav vs deep-link+classifier — optional.)

## 4. Interfaces per layer

### 4.1 JNI (`rs-unified-sdk-jni`) — new exports in `src/dashpay.rs`

All under `support::guard`, errors via `take_pwffi_error` → `DashSDKException(code+1000)`.
FFI structs are read via the rlib types directly (no manual offset math).
**Secret-hygiene rule (normative): the `uri` argument is the bearer secret — it must
never be interpolated into any exception message, log line, or debug output from
`createInvitation`, `claimInvitation`, or `parseInvitation`** (the existing
`"$field must be N bytes"` convention covers byte params only; no precedent exists for
a String param that *is* the secret, so this is easy to get wrong).

| Export (`Java_…_DashpayNative_…`) | Wraps | Notes |
|---|---|---|
| `parseInvitation(uri: String) -> String` | `platform_wallet_parse_invitation` | Returns the preview per the crate's compact-JSON convention (`structurallyValid`, `isInstant`, `hasInviter`, `inviterUsername?`); frees `inviter_username` after copy. Malformed link ⇒ `structurallyValid=false`, not an exception. |
| `createInvitation(walletHandle: Long, amountDuffs: Long, fundingAccountIndex: Int, inviterIdentityId: ByteArray?, inviterUsername: String?, nowUnix: Int, coreSignerHandle: Long) -> String` | `platform_wallet_create_invitation` | Returns the URI (bearer secret). The out-outpoint (caller-owned POD, nothing to free) is ignored — the persistence callback records the row, as on iOS. `now_unix` from a real clock read (Rust rejects 0). |
| `claimInvitation(walletHandle: Long, uri: String, identityIndex: Int, pubkeyRowsBlob: ByteArray, signerHandle: Long, nowUnix: Int)` | `platform_wallet_claim_invitation` | Pubkeys via the existing `decode_registration_pubkeys_blob` (`pubkey_rows.rs:320`); returns id + handle via the **`resumeIdentityWithExistingAssetLock` convention** — `IdentityRegistrationNativeResult` (`[BJ)V`) + `ManagedIdentityHandleGuard` (`identity.rs:569-576, :57-76`). (Not `registerIdentityWithFunding`, which destroys the handle and returns only the id.) |

Plus the reclaim-flag forwarding change in `identity.rs` / `credits.rs` (§3.1).

### 4.2 JNI persistence bridge (`src/persistence.rs`)

- New trampoline `tramp_persist_invitations`, modeled on `tramp_persist_asset_locks`
  (`:1233`): loop the `InvitationEntryFFI` upsert slice and the `[u8;36]` removal slice,
  calling **per-row** bridge methods (the bridge has no array-of-struct convention):
  `onPersistInvitationUpsert(walletId, outPoint: ByteArray(36), fundingIndex: Int,
  amountDuffs: Long, expiryUnix: Int, createdAtSecs: Int, hasInviter: Boolean,
  status: Int)` and `onPersistInvitationRemoval(walletId, outPoint: ByteArray(36))`.
  Any nonzero Kotlin return fails the round so `create_invitation` surfaces
  funded-but-unrecorded instead of silently losing the row.
- Set `on_persist_invitations_fn: Some(tramp_persist_invitations)` (replacing the `None`
  at `:174` and its fail-closed comment).
- ⚠ Lockstep rule: the Rust `call_method` descriptors and the Kotlin bridge signatures
  must land in the same commit — a mismatch is a runtime failure, not a compile error.
  **Descriptor coverage is tested, not eyeballed (adversarial M4):** an instrumented
  test resolves every `NativePersistenceBridge` (name, signature) pair via a test-only
  JNI export (`GetMethodID` over the loaded class — covering all slots, not just the
  new ones), so a descriptor typo fails CI instead of failing the first funded create.
- **Address-pool silent-skip fix (adversarial M1, security-critical):**
  `onPersistAccountAddressPoolEntry` currently early-returns when the parent account row
  is missing (`fetchAccount(...) ?: return@stage`,
  `PlatformWalletPersistenceHandler.kt:509`). For the invitation flow that skip is a
  **lie to the pre-broadcast durability gate**: Rust treats the round's success as
  "funding index durably recorded" and broadcasts; on restart `next_unused` resets and
  the already-exported bearer key is re-exported — the 55937e15c1 bug class with no
  crash needed. Fix in slice 1: for asset-lock funding account types, a missing parent
  account row **fails the round** (nonzero) or upserts the row — never silently skips.
  Add the upgrade-path test: a v7-era wallet with no invitation-account Room row →
  first `createInvitation` must abort **before broadcast**, not succeed non-durably.
  (Audit whether Swift's handler shares the skip; if so, flag upstream.)

### 4.3 Kotlin SDK

- `ffi/DashpayNative.kt`: three new `external fun`s matching §4.1.
- `ffi/NativePersistenceBridge.kt`: the two per-row slots from §4.2
  (`open fun … : Int = 0`).
- `persistence/PlatformWalletPersistenceHandler.kt`:
  - implement both slots — stage upsert/delete of `InvitationEntity` rows via `stage {}`
    keyed by `outPointHex` (the upsert-key ↔ removal-key seam, pinned by test as on
    iOS). **Upserts are partial**: only the FFI-fed columns are written on conflict —
    client-written columns (`statusRaw`, `reclaimInFlight`) are preserved, so a future
    Rust re-emit of an existing outpoint can't reset local status;
  - add `CAPABILITY_INVITATIONS: Long = 0x02` and OR it into
    `persistenceCapabilitiesBits()` **in the same commit that wires the full path**
    (bit + path are inseparable; attesting early re-opens the fail-closed hole).
- New wrapper surface (idiom of `IdentityRegistration.kt` / `IdentityCredits.kt`, suspend
  on `Dispatchers.IO`, KDoc citing the Swift source):
  - `createInvitation(amountDuffs, fundingAccountIndex, inviterIdentityId: ByteArray?, inviterUsername: String?): String` (Swift `ManagedPlatformWallet.createInvitation:2062`)
  - `claimInvitation(uri, identityIndex, identityPubkeys, signer): ByteArray /* identityId */` (Swift `:2145`) — adopts then releases the managed handle per the resume idiom (the claimed identity is already folded into the identity manager + persisted Rust-side); key set = the existing `RegistrationKeys` **6-key** layout (4 base + DashPay enc/dec at keyIds 4–5), pre-persisted via the same path registration uses.
  - `parseInvitation(uri): InvitationPreview` (Swift `:2211`)
  - **New reclaim variants (§3.1):** `reclaimInvitationAsNewIdentity(outPointTxid: ByteArray(32), outPointVout: Int, identityIndex, identityPubkeys /* 4-key set — iOS reclaim-register uses authKeyCount=4, no DashPay pair */, signer)` and `reclaimInvitationAsTopUp(identityId: ByteArray(32), outPointTxid, outPointVout): ULong /* new balance */` — both passing `consumeInvitationVoucher = true`; the only call sites that ever do.

### 4.4 Room (`persistence/`) — DB v7 → v8

`InvitationEntity` — field-exact port of `PersistentInvitation.swift` (and of
`InvitationEntryFFI` for the callback-fed fields):

| Column | Type | Notes |
|---|---|---|
| `outPointHex` | String, `@Unique` | `<txid display hex>:<vout>` via the same encode as the asset-lock entity — the upsert/removal join key |
| `rawOutPoint` | ByteArray(36) | raw `txid_le ‖ vout_le`; reclaim rebuilds the outpoint without re-parsing hex |
| `walletId` | ByteArray, indexed | |
| `fundingIndexRaw` | Int | display metadata; the key is re-derived Rust-side, **no secret column** |
| `amountDuffs` | Long | |
| `expiryUnix` | Int | inviter-side display only (not on the wire) |
| `createdAtSecs` | Int | |
| `hasInviter` | Boolean | |
| `statusRaw` | Int | **0=Created, 1=Claimed, 2=Reclaimed** — pinned Rust-side by `status_to_u8`; discriminants must match byte-for-byte; client-written after create |
| `reclaimInFlight` | Boolean, default false | crash-forensics marker, §4.6 — never a concurrency guard |
| `createdAt` / `updatedAt` | Long | |

Additive migration `MIGRATION_7_8` + exported schema `8.json` + migration test against
`7.json` (instrumented tier, following `DashDatabaseMigrationTest`). DAO exposes a
`Flow` sorted by `createdAtSecs` desc for the list screen.

### 4.5 KotlinExampleApp UI (Compose, `app/…/ui/dashpay/`)

One screen/sheet per Swift file; testTags = iOS accessibility identifiers verbatim;
screens driven by Room Flows + snapshot data — never retained native handles.
**Coroutine-scope rule (all three network flows):** create/claim/reclaim run in an
app-/container-scope (not `rememberCoroutineScope`, which cancels on leaving
composition), with `withContext(NonCancellable)` around the
marker-write → consume → status-write sequence — a mid-flow dismissal must not strand
a half-done reclaim (mirrors the `performDashPaySend` double-send guard precedent).

| Swift | Compose target | Notes |
|---|---|---|
| `InvitationsView.swift` | `InvitationsScreen` | Room Flow over all invitations filtered to loaded wallets (multi-wallet aware — each row reclaims via its own `walletId`); rows show amount, short outpoint, contact-request badge, expiry, status badge (Created/Claimed/Reclaimed); create entry hidden only when no wallet (an active identity is NOT required). Tags `dashpay.invitations.{list,create,reclaim}`, entry `dashpay.openSentInvitations` on `DashPayTabScreen`. |
| `CreateInvitationSheet.swift` | `CreateInvitationSheet` | amount field default **0.03 DASH**, UI range [0.003, 0.05] mirrored for display (Rust enforces); "Send a contact request back to me" toggle (default on, disabled without a username — inviter id+username passed only when opted in); result = QR (ZXing, in-memory bitmap only) + **share as text** (no image export → no `FileProvider` temp file holding the secret) + copy per the clipboard rules in §6; UI single-flight (submit disabled while creating). Tags `dashpay.invite.create.{amount,sendBack,submit,share,copy,done}`. |
| `ClaimInvitationSheet.swift` | `ClaimInvitationSheet` | URI via paste, the existing `QrScanner` route (`savedStateHandle` result), or a parked deep link; `parseInvitation` preview gated only on `structurallyValid` (amount shows "—"); claim wallet selection pins the iOS rule: the active identity's wallet, else the first loaded wallet, entry disabled when none (`DashPayTabView.swift:131-134`); identity index = next unused (reuse the existing registration index logic); claim pre-persists the 6-key `RegistrationKeys` set then calls `claimInvitation`; on success, if `hasInviter && inviterUsername != null` → "Add \<username\>?" prompt → DPNS resolve → `sendContactRequest` (both already ported); works with **no active identity** (fresh invitee); back/dismiss gated while claiming. Tags `dashpay.invite.claim.{uriField,submit}`. |
| `ReclaimInvitationSheet.swift` | `ReclaimInvitationSheet` | reachable only from `statusRaw == 0` rows; segmented target Top-up existing (identity picker) / Register new (**4-key set**); calls the new reclaim wrappers (§4.3); **in-memory `isReclaiming` single-flight gating submit AND dismissal** (Swift `:37,92-109,168-181`) — the persisted marker is crash forensics, never the concurrency guard (an unguarded recomposition off the Room Flow re-emit could double-consume and let the loser's classifier overwrite Reclaimed with Claimed); marker + classifier per §4.6. Tags `dashpay.invite.reclaim.{target,identityPicker,submit}`. |

Navigation: new routes in `Routes.kt` + `AppNavHost.kt`; entry points on
`DashPayTabScreen` ("Sent invitations" + "Claim invitation", ids as on iOS). Deep link
per §3.3: `dashpay`/`invite` `VIEW` intent-filter on `MainActivity` → pending-invite
state → parked until a wallet exists → claim sheet, with the claim-in-flight deferral.

### 4.6 Reclaim crash-safety: marker + classifier (verbatim port)

- **Marker discipline:** capture `hadPriorReclaimInFlight`; persist
  `reclaimInFlight = true` (the Room write **must succeed** — abort the reclaim if it
  doesn't) only **immediately before** the on-chain consume; the register arm pre-persists
  its identity keys **before** setting the marker. On observed success: `statusRaw = 2`,
  clear the marker, save.
- **Classifier:** a pure `internal fun classifyReclaimFailure(error, hadPriorReclaimInFlight)`
  in the app layer, arms identical to Swift (`ReclaimInvitationSheet.swift:406`):
  1. typed `DashSdkError.PlatformWallet.AssetLockAlreadyConsumed` (the mapped class for
     FFI code 24, `DashSdkError.kt:246` — the local tombstone written only after our own
     successful consume; match the type, not a numeric code) → **Reclaimed**
     (`statusRaw = 2`);
  2. message contains `"already completely used"` (consensus 10504, exact canonical
     phrase, lowercased-contains — the typed FFI code for this remains the known
     follow-up) → **Claimed** if no prior marker (provably a foreign claim, neutral
     "already claimed" copy, claimant never named); **ambiguous** if the marker was set
     (resolves to the conservative terminal `Claimed` + ambiguity message, never an
     inferred `Reclaimed`);
  3. message contains `"is not tracked"` with the marker set → explicit ambiguity error,
     state unchanged;
  4. else generic error; clear a stale marker only when `!hadPrior && isNotTracked`.

## 5. Work plan (commit slices)

1. **Persistence spine (the unblocker):** `InvitationEntity` + DAO + `MIGRATION_7_8` +
   `8.json`; per-row bridge slots + handler impl (partial upsert); JNI trampoline; flip
   `on_persist_invitations_fn` to `Some`; attest `CAPABILITY_INVITATIONS`; **the
   address-pool silent-skip fix + upgrade-path test (§4.2)** — all one commit
   (capability bit and path are inseparable). Gate: migration test (instrumented) +
   handler mapping tests + the descriptor-resolution instrumented test +
   `cargo check -p rs-unified-sdk-jni`.
2. **Parse + create + claim bindings:** three JNI exports + `external fun`s + SDK
   wrappers + `InvitationPreview` type. Gate: `./build_android.sh --verify` +
   symbol-load smoke.
3. **Reclaim:** JNI flag forwarding (guard dropped in exactly the two bindings) + the
   two new outpoint-taking Kotlin reclaim wrappers. Gate: compile + tests pinning that
   every generic-recovery path still passes `false` and the generic wrappers still
   reject invitation locks.
4. **App UI:** four screens + routes + DashPay-tab entry points + deep-link filter with
   walletless parking + classifier + marker discipline + single-flight/scope rules.
   Gate: `:app:assembleDebug` + classifier/UI tests.
5. **QA + parity bookkeeping:** PARITY.md rows, emulator QA runs (§7), doc updates.

## 6. Security invariants preserved (review checklist)

- **Voucher-key-reuse defense (55937e15c1):** honest capability attestation + durable
  round commits + **no silent skips anywhere in the funding-index persist chain**
  (§4.2 M1 fix). `CAPABILITY_INVITATIONS` is attested only in the commit that wires the
  full path; the handler stages into the round buffer (§3.4); a persist path that cannot
  complete must fail the round, never no-op.
- **Durability residual (documented, accepted):** Room runs WAL +
  `synchronous=NORMAL` — safe across process death (the relevant Android failure mode),
  but a hard **power loss** can roll back the most recent committed transaction. This is
  pre-existing for every already-attested capability bit and the same class of residual
  iOS carries; documented rather than silently assumed. (Optional hardening —
  `synchronous=FULL` for the wallet DB — is Open Question 4.)
- **Bearer-secret hygiene:** the URI embeds the voucher WIF. Never logged (no
  logcat/android_logger, no exception messages carrying the URI — §4.1 rule — no
  crash-report breadcrumbs), never persisted. Clipboard: Android has **no**
  local-only or auto-expiring clipboard primitive (unlike iOS's
  `localOnly + expirationDate:+60s`), and `ClipDescription.EXTRA_IS_SENSITIVE` is
  API 33+ with minSdk 29 — so: set the flag when `SDK_INT >= 33`, and actively
  compare-and-clear the clipboard after ~60 s (matching the iOS window); the missing
  device-scoping half is an accepted, documented platform gap. Share = text-only
  (no secret-bearing temp image files). QR rendered from an in-memory bitmap
  (`util/QrCode.kt` precedent, no file/cache writes).
- **Backup:** the example app's manifest already sets `android:allowBackup="false"`
  (Room DB has no secret column regardless). Note for integrators: this is an app-level
  property the SDK cannot enforce — production apps must set it themselves.
- **Path gate untouched:** zero changes to `rs-platform-wallet` / `rs-platform-wallet-ffi`
  / `rs-sdk-ffi`, so the `9'/coin'/5'/3'/idx'` export gate and the amount caps stay
  exactly as reviewed on iOS.
- **`consume_invitation_voucher` discipline:** `true` appears at exactly two call sites
  (the two reclaim wrappers, reached only from the reclaim sheet); every generic
  resume/top-up path stays `false` and Rust refuses invitation locks there regardless.
- **Single-flight everywhere funds move:** create sheet (submit disabled while
  creating; Rust's per-wallet build-persist mutex is the backstop) **and** reclaim
  sheet (`isReclaiming` gating submit + dismissal — §4.5); claim sheet gates
  back/dismiss while claiming.

## 7. Test / verification plan

- **JVM unit:** classifier matrix (port `ReclaimInvitationClassifierTests` — all arms,
  incl. marker/no-marker ambiguity split and stale-marker clearing);
  `InvitationEntity` upsert/removal key-seam test (outPointHex form drift, mirror of
  `InvitationPersistenceTests`); handler mapping incl. removal → DAO delete and the
  **partial-upsert preserves `statusRaw`/`reclaimInFlight`** pin; status discriminant
  pin (0/1/2); generic wrappers still reject invitation locks.
- **Rust:** `cargo check -p rs-unified-sdk-jni` (+ clippy).
- **Instrumented (CI emulator):** Room `MIGRATION_7_8` against `7.json`;
  `FfiSmokeTest`-style symbol load for the new externs; the **descriptor-resolution
  test over every bridge slot** (§4.2); a capability-bits assertion that the handler
  satisfies `INVITATION_CREATION`; the **upgrade-path test** (missing invitation-account
  row → create aborts before broadcast).
- **Testnet-gated (`-Ptestnet=true`) / emulator QA** (`emulator-control` skill; faucet is
  rate-limited → self-fund): mirror the iOS rows DP-12..DP-19 —
  create (funded, row lands in Room + list), claim (second wallet, no funds → funded
  identity; optional contact bootstrap), malformed/reused/wrong-network rejection (fail
  loud, no side effects), sent-list persistence + upsert-in-place, reclaim-as-top-up
  (balance rises, row → Reclaimed), reclaim-as-register (4-key set), already-consumed
  race (second consume → deterministic "already completely used" → neutral Claimed
  copy). The funded two-wallet race remains manual, as on iOS.
- **Acceptance gate:** the funded create→claim e2e on the emulator against testnet.

### Funded e2e evidence (2026-07-23, arm64 emulator + testnet)

- Instrumented tier: 32/32 green (Room `MIGRATION_7_8` + full v1→v8 chain,
  `persistenceBridgeDescriptorsAllResolve`, FFI smoke; 3 testnet-gated skips).
- Create (DP-12/16): 0.03 DASH voucher funded + InstantSend-locked, legacy link
  emitted, row landed in Room + Sent list as `Created` (outpoint `f9ef7f5b…:0`).
- Claim (DP-13): link pasted → valid preview → new identity `292ebab4…`
  registered with 2 818 643 580 credits, funded solely by the voucher.
- Already-consumed (DP-19 classifier, live): reclaiming the claimed voucher hit
  consensus 10504 → neutral "This invitation was already claimed.", row →
  `Claimed`, reclaim affordance gone.
- Interrupted create: a second create timed out waiting for its InstantSend
  lock — the funded row (`732bf38c…:0`) was already persisted and reclaimable,
  proving the persist-before-proof-wait ordering on Android.
- Reclaim-as-top-up (DP-17): that voucher consumed into identity `7023bed1…`,
  balance 49 818 637 700 → 52 736 112 114 credits, row → `Reclaimed`.
- Malformed link (DP-15): garbage URI → "Invalid invitation link.", claim
  disabled, no side effects.
- Not exercised here: the two-wallet contact-bootstrap (DP-14 — inviter had no
  DPNS username, so the opt-in toggle was correctly disabled) and
  reclaim-as-register (DP-18 — same FFI + wrapper as the seam-tested claim
  path); both remain manual QA, matching iOS.

## 8. Failure modes

- **Capability over-attestation / silent persist skips** → voucher-key reuse (the
  pre-port bug class; adversarial M1 shows a no-crash variant via the address-pool
  skip). Guard: bit + path in one commit; the M1 fail-the-round fix; instrumented
  capability + upgrade-path assertions.
- **Handler write outside the round buffer** → broken round atomicity or lost-on-crash
  rows reported as persisted. Guard: §3.4 rule + mapping tests exercise `stage {}`.
- **JNI descriptor mismatch** → every invitation persist fails at runtime — after real
  funds broadcast, if untested. Guard: lockstep rule + the descriptor-resolution
  instrumented test **before** any funded QA.
- **Crash between broadcast and the invitation-row round** → a funded voucher with no
  Room row: invisible in Sent invitations, unreclaimable from the UI (reclaim is
  row-driven), funds stranded pending manual recovery. Inherent to the shared
  persist-after-broadcast ordering (iOS carries the same window); the
  funded-but-unrecorded error copy must surface the **outpoint** so support/manual
  recovery is possible.
- **Process death mid-reclaim** → marker semantics resolve it (our-tombstone →
  Reclaimed; foreign claim → Claimed; genuinely ambiguous → conservative Claimed +
  message). Double-tap/recomposition races are excluded by the in-memory single-flight
  (§4.5), not by the marker.
- **URI leakage via logs/exceptions/clipboard/share files** → bearer theft. Guard:
  §4.1 exception rule + §6 clipboard/share rules.
- **ChainLock fallback at create** → Rust rejects the link, lock stays reclaimable;
  surface the error copy as iOS does.
- **Deep link with no wallet** → parked, not dropped (§3.3); "create a wallet to claim"
  copy.
- **QR/paste garbage** → `structurallyValid=false` preview, claim button disabled, no
  side effects.

## 9. Out of scope

- Typed FFI code for consensus 10504 already-consumed (shared iOS/Android follow-up).
- A typed funded-failure result carrying the recovery outpoint from
  `create_invitation` (adversarial-review follow-up): on the rare
  funded-but-row-persist-failed path the outpoint doesn't cross the JNI boundary
  (the FFI fills out-params only on success — changing that is a shared-Rust
  change). Recovery exists meanwhile via the Rust-side tracked-lock list
  (diagnostics surface); the common interrupted-create case persists the row
  and is reclaimable from the UI (funded-e2e verified).
- Driving the Rust pre-broadcast abort from a JVM test (the broadcast boundary
  is Rust-side; the Kotlin tier pins the round-failure half —
  `invitationPoolEntryWithoutWalletFailsTheRound` — and Rust's own
  `create_invitation_requires_durable_persistence` pins the abort).
- Any `rs-platform-wallet` / `rs-platform-wallet-ffi` change (incl. Rust-emitted
  Claimed/Reclaimed status changesets — latent on iOS too).
- Fixing the iOS walletless deep-link drop (flagged upstream, §3.3).
- AppsFlyer/OneLink or App-Links-style verified deep-link transport (tracked separately
  for iOS as well).
- Contested-name claim tier (deferred on iOS).

## 10. Decisions (RESOLVED — owner sync, 2026-07-22)

1. **Reclaim JNI guard drop: approved.** `generic_asset_lock_recovery_allowed` is
   removed from exactly the two forwarding bindings; Rust core's
   `authorized_invitation_reclaim` remains the (independently unit-tested) enforcement —
   the same single-gate trust model shipped on iOS.
2. **Deep link: both filters.** `dashpay://invite` custom scheme + the legacy AppsFlyer
   `https://invitations.dashpay.io/applink` host (unverified chooser participation until
   #4096 serves `assetlinks.json`), with walletless parking (§3.3).
3. **PR strategy: one PR, five sliced commits** (§5).
4. **WAL durability: accept + document** the `synchronous=NORMAL` power-loss residual
   (§6) — platform-wide status quo, same class as iOS; `synchronous=FULL` would tax
   every wallet write and belongs to a separate measured change if ever.
