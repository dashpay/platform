# Kotlin SDK — DashPay registration-key provisioning spec

**Status:** REVIEWED (2 rounds) — feasibility/security pass, then an
independent codex cross-check with a much deeper call-graph trace; several
material corrections applied. The resume-path funding question is resolved
by the parity mandate itself (§0.2); one small implementation-detail
decision remains (§0.3)
**Branch:** own PR/worktree, stacked on PR #3999's `feat/kotlin-sdk-and-example-app`
head (split out from the sibling Keystore/docs spec — different subsystem,
own rollback unit, per the repo's own PR-slicing convention at
`KOTLIN_SWIFT_SHARED_PARITY_SPEC.md:508`)
**Scope:** `packages/rs-unified-sdk-jni`, `packages/rs-platform-wallet-ffi`,
`packages/rs-platform-wallet`, `packages/kotlin-sdk`,
`packages/kotlin-sdk/KotlinExampleApp`

## 0. Spec review findings

### 0.1 Round 1 (feasibility + security)

1. **Factual error, corrected: `decode_contract_bounds` is NOT usable from
   the JNI decoder** (wrong crate, `pub(crate)`-scoped, operates on an
   already-parsed struct not the raw blob). Real target:
   `decode_update_pubkeys_blob` + `DecodedPubkeyRow`
   (`rs-unified-sdk-jni/src/transactions.rs:291,305`) — **superseded by
   §0.2 item 1 below**, which found promoting these alone is still
   insufficient.
2. Confirmed: no protocol/FFI-struct change needed — `IdentityPubkeyFFI`
   already carries the needed fields.
3. `IdentityUpdates.kt`'s `encodeAddPubkeys` (and its helper
   `contractBoundsKind`) are `private` — need hoisting.
4. Security LOW: contract-bounds not validated at CREATE time —
   **corrected by §0.2 item 3 below**, which found this claim was
   overstated.
5. Security LOW: row-0-MASTER invariant path-dependent —
   **corrected by §0.2 item 2 below**: it's a key-ID-0 invariant, not a
   row-position invariant, and the fix location matters (must not touch
   the shared update decoder).
6. Confirmed safe: security-level/purpose escalation not exploitable
   server-side — **reconfirmed by round 2**, still holds.
7. Confirmed: no test references the functions being deleted — safe to
   remove.

### 0.2 Round 2 (independent codex cross-check — supersedes several round-1 items)

A second, independent review re-derived every claim from source rather than
trusting the first draft, and found the design still has real gaps:

1. **MAJOR — promoting the decoder alone doesn't make it usable.**
   `DecodedPubkeyRow`'s fields are private
   (`transactions.rs:291-301`) — making the struct and function
   `pub(crate)` doesn't let `identity.rs`/`funding.rs` read the fields to
   build `IdentityPubkeyFFI` rows. Need field accessors, a crate-private
   `into_ffi`-style conversion helper, or move the row+decoder into a
   shared module. Additionally, the current decoder takes `JNIEnv`/
   `JByteArray` directly (`transactions.rs:305-306`) — split it into a
   pure `&[u8]` parser plus a thin JNI adapter, or the proposed Rust
   round-trip unit tests can't be written as ordinary Rust tests.
2. **MAJOR — the invariant is "key ID 0", not "row 0", and must NOT go in
   the shared update decoder.** `decode_update_pubkeys_blob` is also used
   by `TransactionsNative.updateIdentity` (`transactions.rs:193-198`), and
   an ordinary add-key update legitimately has no key ID 0 at all — adding
   an unconditional key-0 check there would break normal key-addition
   updates. Put the invariant in a registration-only seam instead:
   `rs-platform-wallet-ffi::decode_identity_pubkeys`
   (`identity_registration_with_signer.rs:273-323`) or a registration-only
   JNI wrapper. **Also: `decode_identity_pubkeys` has a 5th caller the
   original spec missed** — Swift's invitation-claim path
   (`rs-platform-wallet-ffi/src/invitation.rs:244-300`). Moving the
   key-0 check here affects that path too; it already has the same
   downstream invariant separately
   (`rs-platform-wallet/src/wallet/identity/network/invitation.rs:127-153`)
   so the change should be behaviorally benign, but needs its own explicit
   test, and the spec must stop saying "four paths."
3. **MAJOR — the CREATE-time-bounds-validation claim was overstated in the
   wrong direction.** It's true there's no explicit
   `validate_identity_public_keys_contract_bounds` call on the CREATE path
   (that validator is UPDATE-only,
   `rs-drive-abci/.../identity_update/state/v0/mod.rs:97-110`). But Drive's
   actual key-insertion path (`create_key_tree_with_keys_operations` →
   `add_potential_contract_info_for_contract_bounded_key`,
   `rs-drive/src/drive/identity/key/insert/create_key_tree_with_keys/v0/mod.rs:98-123`)
   **does** fetch the referenced contract, error if it doesn't exist,
   resolve the document type, and require that type to declare the
   matching bounded-key requirement
   (`rs-drive/.../add_potential_contract_info_for_contract_bounded_key/v0/mod.rs:430-465`).
   So "any wrong constant registers successfully" is **false** — a
   nonexistent contract or missing document type fails registration. The
   real residual risk is narrower: a wrong-but-*existing-and-compatible*
   contract/doc-type would still be silently accepted under the wrong
   scope. Test plan corrected accordingly (§4).
   **Additional correction**: manual Add Contact success is not proof the
   bounds are actually correct — `select_own_encryption_key` only checks
   enabled/type/purpose, not bounds, and contact-request validation
   intentionally accepts bound *or* unbound ENCRYPTION keys
   (`rs-platform-wallet/src/wallet/identity/crypto/validation.rs:111-121,
   164-167`). Add an explicit on-chain bounds assertion to the test plan —
   a passing Add Contact smoke does not cover this.
4. **MAJOR — my plan never specified how the BASE 4 keys' roles get
   rebuilt after `role_for_registration_key_id` is deleted.** That function
   is the *only* current place IDs 0-3 get their roles (MASTER/CRITICAL/
   HIGH/TRANSFER) assigned. The original approach only described stamping
   the 2 new DashPay keys — it left the base 4 keys' rich-row construction
   completely unspecified. Fixed in §2 approach below: the Kotlin side must
   build rich rows for ALL 6 keys (0-5), not just the 2 new ones.
5. **MAJOR — "wire into all 4 funding paths" is not Swift parity and misses
   a funding constraint.** Swift deliberately **excludes** the
   unused-asset-lock resume path from DashPay provisioning
   (`CreateIdentityView.swift:152-177,896-905`) and **raises the minimum
   registration funding** based on the added key count (`:127-149`). Adding
   2 keys costs an additional 13,000 duffs at the current per-key fee
   (`rs-platform-version/.../fee/state_transition_min_fees/v1.rs:17`) and
   reaches exactly `max_public_keys_in_creation = 6` — the protocol
   ceiling, not headroom under it. Kotlin's `TrackedAssetLock` doesn't even
   expose the underlying lock's `amount` field
   (`TrackedAssetLock.kt:11-19` vs. Rust's `tracked.rs:63-72`), so there's
   currently no way to check whether a previously-created (four-key-sized)
   lock has enough value for a six-key create. **Resolved, not an open
   decision (see §0.2 resolution below): this is exactly the problem iOS's
   own exclusion already solves — match it, don't re-derive a Kotlin-
   specific answer.**
6. **Medium/high — `decode_identity_pubkeys` silently last-wins on
   duplicate key IDs**, not fail-closed as claimed: it inserts into a
   `BTreeMap` with no check for an existing entry
   (`identity_registration_with_signer.rs:273-323`). The surviving key is
   still structurally validated (not a privilege escalation), but explicit
   duplicate-ID rejection should be added to the decoder.
7. **Medium/high — the proposed contract-ID pin test is impossible as
   described.** A Rust test in `rs-unified-sdk-jni` cannot read a private
   Kotlin source constant to compare against
   (`AddIdentityKeyScreen.kt:382-397` vs.
   `packages/dashpay-contract/src/lib.rs:9-16`) — there's no cross-language
   visibility. Use a shared generated/checked-in golden fixture instead
   (§4).
8. **New scope items the original spec didn't consider at all:**
   - **Wire-format skew risk**: the JNI method descriptors stay identical,
     but the opaque `byte[]` payload shape changes incompatibly. An old
     Kotlin artifact paired with a new native `.so` (or vice versa) would
     be silently misparsed, not rejected. At minimum, reject legacy-shaped
     or trailing/malformed input deterministically; a version/magic byte
     would make skew diagnosable if that's judged worth the extra
     complexity.
   - **Public Kotlin API compatibility**: `IdentityKeyPreview` is public
     (`IdentityKeyPreview.kt:17-22`) and public registration methods accept
     `List<IdentityKeyPreview>` (`IdentityRegistration.kt:117-146,237-291`,
     `PlatformWalletManager.kt:1344-1371`). Replacing or extending this
     type has source/binary compatibility implications for SDK consumers
     — needs an explicit compatibility strategy, not treatment as an
     app-only refactor.
   - **No backfill for existing identities**: this only fixes freshly
     registered identities. Existing Android-created identities remain
     without the DashPay pair and still need the existing Add Identity Key
     repair flow. State this explicitly as non-goal, or add detection/
     guidance if backfill is actually wanted.
   - **Existing reusable plumbing**: `KotlinExampleApp` already has a
     derive → validate → persist → zero → `IdentityPubkey` pipeline in
     `IdentityKeyAdditionFlow.kt:84-182` (used by `AddIdentityKeyScreen.kt`).
     Before writing a parallel `DashpayKeyProvisioning` helper, decide
     whether a thin policy wrapper around this existing flow is sufficient
     — duplicating the lifecycle is another drift surface.
   - **Keypair-correspondence validation gap**: Swift explicitly verifies
     the derived private scalar matches the public key before persisting
     (`IdentityRegistrationKeys.swift:66-82`); the current Kotlin
     registration-preview path doesn't do this check at all
     (`CreateIdentityScreen.kt:202-235`). Pre-existing, not caused by this
     change, but a "ported from Swift" claim should either add the same
     defense or explicitly document the parity gap.
   - **Secret lifecycle risk**: deriving `base + 2` while also describing
     "the existing base list plus an appended pair" risks deriving the
     first four private-key byte arrays twice with a discarded, unzeroed
     copy. Since proof-of-possession signs every submitted key, losing a
     private half after the corresponding pubkey is registered makes that
     key permanently, silently unusable. Spec must be explicit: one
     six-key preview derivation, not two overlapping ones, with every
     preview/private array (including on failure paths) zeroed.
9. **Verification command was wrong.** `cargo test -p rs-platform-wallet-ffi
   --lib` fails immediately — the crate is named `platform-wallet-ffi`
   (`rs-platform-wallet-ffi/Cargo.toml:1-2`), confirmed by actually running
   both commands (196 tests pass under the correct name). Corrected in §5.

### 0.2 Resolved: resume-path policy

**Not actually an open decision — resolved by the project's own parity
mandate, no Kotlin-specific design call needed.** iOS resume finishes an
interrupted registration with exactly the key set it originally started
with and never changes that set mid-resume; it does not attempt to
determine or verify whether the pre-existing lock is large enough for a
bigger key set, because it simply never asks the lock to cover more than
what it already committed to. That's iOS's own answer to the exact
fund-safety problem in §0.2 item 5 above (a resumed lock is a fixed,
already-spent-on-chain amount; retroactively growing the transition it
funds risks a resume that fails after the user's DASH is already
irreversibly locked). Since this whole PR exists to port Swift's behavior,
not invent new Kotlin policy, the answer is simply: **leave the resume path
alone.** Do not extend DashPay provisioning to
`resumeIdentityWithExistingAssetLock` at all — it keeps using the base
4-key set exactly as it does today. A user who resumes registration and
wants DashPay capability gets it afterward through the existing Add
Identity Key flow, same as iOS. This requires **zero new FFI surface**
(no `TrackedAssetLock.amount` exposure needed) and removes an entire
branch of scope from this PR rather than adding one.

### 0.3 Remaining open decision

- **`DashpayKeyProvisioning` vs. reusing `IdentityKeyAdditionFlow`**
  (§0.2 item 8): recommend evaluating the existing flow first; only write
  a new helper if it genuinely can't be adapted (e.g. its persistence
  timing assumes post-registration, not pre-registration).

## 1. Problem

`role_for_registration_key_id` (`rs-unified-sdk-jni/src/identity.rs:122-131`)
reconstructs purpose/security-level *positionally* from a bare `keyId`, and
every registration call site hardcodes `contract_bounds_kind: 0`. The wire
format all 4 sites decode (`decode_pubkeys_blob`, `identity.rs:933`) carries
only `keyId` + pubkey bytes. Consequently, an identity created by the Android
app reaches its own advertised Add Contact flow without an enabled
ECDSA_SECP256K1 encryption key, and `select_own_encryption_key` rejects the
request (confirmed: `contact_requests.rs:455-465,979-1005`).

Swift's reference flow (`IdentityRegistrationKeys.swift::makeDashpayKeyPair`)
derives 2 extra keys bounded to DashPay's `contactRequest` document type and
appends them before dispatching to registration — for eligible fresh-funding
paths only, explicitly excluding resume (§0.2 item 5). This is Swift
application-level code, not shared Rust.

## 2. Approach

1. **Rust — decoder.** Extract a pure `&[u8]` parser from
   `decode_update_pubkeys_blob`'s logic (§0.2 item 1), with a
   crate-private conversion path from the parsed rows to `IdentityPubkeyFFI`
   that exposes what registration needs (not just `pub(crate)` on an
   already-private-fielded struct). Keep the existing JNI-facing function as
   a thin adapter over the pure parser for the update path; add a
   registration-facing entry point that uses the same pure parser plus a
   **registration-only** key-ID-0 = MASTER+AUTHENTICATION check and
   explicit duplicate-key-ID rejection (§0.2 items 2, 6) — do NOT add
   either check to the shared update-path decoder. Reject trailing/
   malformed bytes deterministically in the same pass (§0.2 item 8).
2. **Rust — call sites.** Call from all 4 registration JNI sites
   (`identity.rs:609/753/868`, `funding.rs:734`) **and** account for the
   5th caller of `decode_identity_pubkeys`, the invitation-claim path
   (`invitation.rs:244-300`) — add explicit test coverage there too rather
   than silently changing its behavior (§0.2 item 2). Delete
   `role_for_registration_key_id` and `decode_pubkeys_blob` (confirmed
   dead — no test references) along with the now-dead local role constants
   (`identity.rs:79-90`).
3. **Kotlin — base + DashPay rows, not just DashPay rows.** Build rich rows
   for the **complete** 6-key set in one derivation pass — base 4 (IDs 0-3,
   replicating today's MASTER/CRITICAL/HIGH/TRANSFER roles explicitly, since
   `role_for_registration_key_id` no longer exists to do it implicitly) plus
   the 2 DashPay keys (IDs 4-5, ENCRYPTION/DECRYPTION, MEDIUM, DashPay
   `SingleContractDocumentType` bounds) — not two separate/overlapping
   derivations (§0.2 item 8, secret lifecycle). Hoist `encodeAddPubkeys` +
   `contractBoundsKind` out of `IdentityUpdates` into a shared location.
   Hoist the DashPay contract-id constant out of `AddIdentityKeyScreen.kt`
   into one shared location instead of a third copy. Evaluate reusing
   `IdentityKeyAdditionFlow.kt`'s existing derive→validate→persist→zero
   pipeline before writing a new parallel helper (§0.3 remaining decision).
4. **Kotlin — funding paths.** Wire the six-key set into fresh Core-funded,
   Platform-address-funded, and shielded registration. **Exclude
   unused-asset-lock resume** from DashPay provisioning entirely, matching
   Swift (§0.2, resolved — not a design choice, just parity) — resume
   continues to use the base 4-key set as today; a user recovering via
   resume can add DashPay keys afterward through the existing Add Identity
   Key flow. Document this explicitly as intentional, not an oversight.
5. **No backfill.** Existing already-registered identities are explicitly
   out of scope (§0.2 item 8) — they keep using the existing Add Identity
   Key repair flow, unchanged by this PR.
6. **Stale documentation.** Update positional-role wording that becomes
   incorrect once `role_for_registration_key_id` is gone:
   `identity.rs:258-270,682-697,813-815`, `funding.rs:650-660`,
   `identity_key_preview.rs:77-94,257-280`, `IdentityNative.kt:148-178`,
   `IdentityRegistration.kt:191-210`.

## 3. Failure modes (as reviewed, 2 rounds)

- Wire-format skew between an old Kotlin artifact and a new native library
  (or vice versa) would silently misparse rather than fail loud — mitigated
  by deterministic rejection of malformed/trailing/legacy-shaped input
  (§0.2 item 8); a version/magic byte is a further option if judged
  worthwhile.
- A wrong-but-*existing-and-compatible* DashPay contract/doc-type constant
  would still register successfully under the wrong scope (narrower than
  originally claimed — Drive DOES reject a nonexistent contract/doc-type,
  §0.2 item 3) — mitigated by pinning the constant against a genuine
  cross-language golden fixture (§4), not a same-literal echo.
- Duplicate key IDs in a caller-supplied row list silently collapse to
  last-wins in `decode_identity_pubkeys`'s `BTreeMap` — mitigated by
  explicit duplicate-ID rejection (§0.2 item 6).
- Key-ID-0 invariant, if misplaced in the shared update decoder, would
  break ordinary key-addition updates that don't include ID 0 — mitigated
  by keeping it registration-only (§0.2 item 2).
- A previously-created four-key-sized unused asset lock is excluded from
  six-key resume entirely (§0.2, resolved by parity — resume never grows
  past what it originally committed to), so no insufficient-funds failure
  mode exists there by construction; fresh funding paths need the six-key
  minimum reflected in their fee/sufficiency calculation.
- Security-level/purpose escalation via a malformed row list — confirmed
  not exploitable; `validate_identity_public_keys_structure_v0` rejects the
  whole transition server-side regardless of what Kotlin sends.
- Deriving the base 4 keys' private material twice (once for preview, once
  overlapping with DashPay-key derivation) risks an unzeroed discarded
  copy — mitigated by one single six-key derivation pass (§2 point 3).

## 4. Test plan (red → green, per repo TDD discipline)

- **Rust (red first), pure-parser round trip**: encode a row list including
  the full 6-key policy (base 4 + bounded ENCRYPTION/DECRYPTION), decode via
  the new pure `&[u8]` parser (§0.2 item 1 — required for this test to be
  an ordinary Rust unit test, not JNI-environment-dependent), assert every
  field round-trips. Write against the CURRENT decoder first to confirm no
  such fields exist today.
- **Rust, cross-language golden fixture** (§0.2 item 7, replacing the
  infeasible constant-pin approach): check in one fixture — 6 canonical
  rows encoded to exact bytes. A Kotlin unit test asserts its encoder
  produces those exact bytes; a Rust unit test asserts its parser decodes
  those exact bytes to the expected fields. This catches byte-order/
  field-order skew that two independently-written, independently-passing
  tests would miss.
- **Rust, strict codec edge cases**: both bounds kinds, truncation at each
  variable-length field, invalid bounds-kind byte, interior NUL, negative
  key ID, invalid boolean byte for `readOnly`, duplicate key ID (must
  reject, not last-wins), trailing bytes (must reject), and explicit
  legacy-format rejection.
- **Rust, registration-only invariant test**: key ID 0 must be
  MASTER+AUTHENTICATION regardless of row order; missing/wrong ID 0 fails.
  **Separately**, a regression test proving identity-UPDATE add-key lists
  *without* key ID 0 remain accepted (proves the invariant didn't leak into
  the shared decoder).
- **Rust, FFI decoder test**: duplicate IDs rejected (not silently
  overwritten); ENCRYPTION/DECRYPTION without bounds rejected (existing
  behavior, confirm still true after the refactor). If the key-0 check
  lives in `decode_identity_pubkeys`, include a test for the invitation
  caller's behavior too (§0.2 item 2).
- **Kotlin (red first), provisioning/lifecycle test**: assert exact IDs 0-5
  with correct type/purpose/securityLevel/readOnly/bounds for all 6 (not
  just the 2 new ones), correct derivation indices, storage under matching
  public bytes, and private-array zeroing on both success and failure
  paths. Write against the stub/nonexistent helper first to confirm no such
  rows are produced today.
- **Kotlin, four-way orchestration test**: extract a small pure
  preparation/dispatch seam from `CreateIdentityScreen` and verify every
  eligible funding source receives the identical rich 6-row set, and that
  resume does NOT (per the resume-exclusion policy) — cheaper than
  exercising all 4 JNI/JVM paths directly.
- **On-chain bounds assertion** (§0.2 item 3 — Add Contact success alone
  does not prove this): after a local/testnet create, fetch the identity
  and assert IDs 4/5 carry the exact DashPay contract ID and `contactRequest`
  document-type bounds.
- **Resume-funding case**: test the chosen policy explicitly — resume uses
  the base 4-key set only (recommended), or an amount-aware six-key
  sufficiency check if policy (b) is chosen instead.
- **Manual/device (environment-bound)**: register a fresh identity via
  KotlinExampleApp on testnet, confirm Add Contact now succeeds. This
  remains useful as an end-to-end product check but is explicitly NOT
  sufficient on its own (§0.2 item 3) — the on-chain bounds assertion above
  is the actual correctness proof.
- Existing test needing mechanical update: `IdentityAssetLockRecoveryTest.kt:169`
  constructs the legacy `IdentityKeyPreview` shape used by resume tests —
  will need updating if the registration row type changes shape (even
  though resume itself stays on the base 4-key policy).

## 5. Verification plan

```bash
cargo test -p rs-unified-sdk-jni --lib
cargo test -p platform-wallet-ffi --lib   # NOT rs-platform-wallet-ffi — that name doesn't exist (§0.2 item 9)
cargo clippy --workspace --all-features
cargo fmt --check --all

cd packages/kotlin-sdk
JAVA_HOME=/opt/homebrew/opt/openjdk@17 ./gradlew \
  :sdk:assembleDebug :sdk:testDebugUnitTest \
  :app:assembleDebug :app:testDebugUnitTest \
  :sdk:compileDebugAndroidTestKotlin
```

Baseline (pre-change) confirmed by the round-2 review:
`cargo test -p rs-unified-sdk-jni --lib` → 10 passed;
`cargo test -p platform-wallet-ffi --lib` → 196 passed.

Manual/device-bound: testnet identity registration + Add Contact flow smoke,
**plus** the on-chain bounds assertion above (Add Contact success alone is
not sufficient evidence per §0.2 item 3).
