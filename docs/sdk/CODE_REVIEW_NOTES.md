# Final multi-agent code review: Kotlin DashPay registration keys

**Review date:** 2026-07-21  
**Branch:** `feat/kotlin-sdk-dashpay-registration-keys`  
**Implementation base:** `6efa83bb53`  
**Reviewed implementation commits:** `70b0852edb`, `3a64940af8`,
`c31ca592d4`, `4a25717471`, plus the concurrently-added persistence refactor
`5147d5baa9`

## Review method

Three independent agents reviewed the actual diff and call graph under separate
lenses, followed by a primary-agent source audit and full build/test run:

1. Rust wire format, registration invariants, FFI callers, and consensus-facing
   key policy.
2. Kotlin derivation/persistence/zeroization lifecycle, funding policy, resume
   exclusion, and the helper-reuse decision.
3. Cross-language fixture and edge-case test adequacy, including the invitation
   caller and environment-bound on-chain assertion.

The agents were instructed to review only and made no edits or commits. While
the review was running, `5147d5baa9` was committed by an external concurrent
session; it was reviewed as part of the resulting branch head and was not
rewritten.

## Verified correct in the implementation

- `parse_pubkey_rows` is a pure `&[u8]` parser. JNI conversion is confined to
  thin update/registration adapters.
- Empty-list, duplicate-key-ID, and key-ID-0 = MASTER + AUTHENTICATION checks
  are registration-only. The shared update parser still accepts ordinary
  add-key lists without key ID 0.
- All four Android registration JNI exports use the rich registration decoder:
  resume, Core-funded, Platform-address-funded, and shielded-from-pool.
- The fifth `decode_identity_pubkeys` caller, invitation claim, still uses the
  shared FFI decoder and therefore receives duplicate-ID rejection without
  receiving the JNI-only key-ID-0 policy.
- `role_for_registration_key_id` and `decode_pubkeys_blob` are deleted, not
  merely superseded.
- Kotlin explicitly rebuilds the base roles as MASTER/AUTHENTICATION,
  CRITICAL/AUTHENTICATION, HIGH/AUTHENTICATION, and CRITICAL/TRANSFER.
- Keys 4/5 are ECDSA secp256k1, ENCRYPTION/DECRYPTION, MEDIUM, writable, and
  bounded to the canonical DashPay contract's `contactRequest` document type.
- Fresh registration performs one six-key derivation pass. It does not derive
  an overlapping base-four set first.
- Resume derives and submits only the base four keys. The DashPay pair is not
  added to an already-funded asset-lock resume.
- The checked-in golden binary is genuinely shared: Kotlin compares its encoder
  output to that exact resource, while Rust includes and decodes the same file
  and pins its contract ID to `dashpay_contract::ID_BYTES`.
- Strict codec coverage includes truncation, legacy layout, trailing bytes,
  invalid bounds/boolean bytes, negative IDs, interior NUL, duplicate IDs, and
  update-without-key-0 behavior. FFI tests cover invalid DPP role bytes.

## Findings fixed during this review

### 1. Private material survived two failure windows

`IdentityKeyPreview.decodeAll` wiped the JNI blob only after a complete parse.
A malformed/truncated blob could leave both the source blob and already-copied
private scalars unsanitized. Separately, a blocking JNI preview could finish
after its coroutine was cancelled; `withContext` would then discard the
secret-bearing result before the caller reached provisioning cleanup.

Fix:

- Decode under `try/catch/finally`, wiping the source blob on every exit and
  wiping all partially decoded private arrays on failure.
- Add `opWithCleanupOnCancellation`, which scrubs a completed result if prompt
  cancellation discards it during dispatcher handoff.
- Route both registration preview APIs through that cleanup-aware gate.
- Add malformed-blob and blocking-JNI cancellation regressions. Both tests were
  observed failing before the production fix and passing afterward.

### 2. Six-key Core funding minimum was not enforced

The app accepted any numeric Core amount even though six creation keys raise
the protocol floor from 228,000 to 241,000 duffs. A below-floor asset lock can
be broadcast before Platform rejects the identity create.

Fix:

- Mirror `IdentityCreateTransition::calculate_min_required_fee_v1` for the
  current key count.
- Use one amount-policy function for both submit enablement and the click-time
  preflight.
- Add boundary tests for the four-key/six-key floors and the resume no-new-funds
  case. The new test was observed failing before the implementation and passing
  afterward.

### 3. Invitation claim lacked its required caller-level regression

The existing duplicate-ID unit test called `decode_identity_pubkeys` directly
and only stated in a comment that invitation claim used it. It did not exercise
the fifth caller itself.

Fix:

- Add a valid-invitation/duplicate-row test through
  `platform_wallet_claim_invitation`.
- Assert duplicate rejection happens before wallet lookup, signer use, or
  network work, and that output sentinels remain zeroed.

### 4. Coverage and documentation were weaker than the implementation claims

Fix:

- Correct the Rust module documentation: the pure structural parser does not
  validate DPP role discriminants; the downstream FFI conversion does.
- Add direct invalid key-type/purpose/security-level FFI coverage and a
  MASTER-with-wrong-purpose registration invariant test.
- Strengthen Kotlin tests to prove the exact public-key storage key, non-zero
  scalar at persistence time, wallet ownership, post-persist scrubbing,
  DashPay key type/purpose/security/read-only flags, and exact bounds.
- Correct the helper rationale: the add-key flow persists before broadcast;
  the real incompatibilities are its existing-key/max+1 update policy and
  per-slot derivation, versus registration's fixed 0..N batch policy.

## PR #4173 automated-review follow-up

The first automated PR review found three additional blocking regressions. All
three were independently traced to the current source and fixed:

1. **Mutable controls could redirect an in-flight registration.** The click
   handler captured the key-count choice before suspension but later reread
   `fundingSource` and `selectedRecoveryLock`. It now resolves source, amount,
   identity index, and a defensive copy of the selected lock into one immutable
   submission snapshot before the coroutine starts. Preparation, dispatch,
   coordinator tracking, and navigation use only that snapshot. A regression
   mutates the backing form values and lock txid after capture and verifies the
   submission is unchanged.
2. **Platform-address packing did not reserve the six-key creation fee.** The
   default native strategy deducts the fee from the post-spend remainder of
   BTreeMap input 0. The packer now mirrors the consensus formula
   `2,000,000 + 6,500,000 * keyCount + 500,000 * inputCount`, models Rust's
   `(address type, unsigned hash)` ordering, and selects the smallest input set
   that contributes the requested identity balance while leaving the full fee
   on input 0. This preflight runs before key derivation/persistence. The
   reviewer's 70M-balance/30M-spend case was observed failing before the fix;
   exact one-input and two-input boundaries are also covered.
3. **Resume lost its HD-slot provenance check.** Rich consensus rows do not
   carry a derivation index, so replacing `IdentityKeyPreview` had deleted the
   old per-key guard. Provisioning now returns a `RegistrationKeySet` that keeps
   the common preview identity index beside the rich rows, rejects mixed-slot
   previews before persistence, and resume verifies the set index matches the
   tracked lock before JNI. The wrong-slot regression was observed calling JNI
   before the fix and passing afterward.

The full Kotlin SDK/app build, unit-test suites, and instrumented-test-source
compilation passed after these fixes. The PR's initial emulator check hit the
workflow's documented Android Keystore unlock-state race; its failed-job rerun
passed the complete native build and API-35 instrumented suite without a code or
workflow change.

## Left for human judgment or environment-bound verification

1. **On-chain bounds assertion is still manual.** No unit test can prove what a
   testnet node persisted. Register a fresh identity, fetch it, and assert keys
   4/5 contain the exact DashPay contract ID plus `contactRequest` bounds. Then
   run Add Contact as a separate smoke test; Add Contact success alone is not
   proof of the bounds.
2. **Public Kotlin/logical wire compatibility.** Public fresh-registration
   methods now consume rich `IdentityPubkey` lists, resume consumes a
   provenance-carrying `RegistrationKeySet`, and the opaque `byte[]` layout
   changed without a version/magic prefix. The parser fails closed on
   legacy/malformed shapes, but an old Kotlin artifact must not be paired with
   the new native library. Release coordination or an explicit compatibility
   layer remains a product/API decision.
3. **No full Compose four-way dispatch seam test.** Static tracing confirms all
   three fresh paths pass the same six-row list and resume passes the four-row
   list; policy tests cover the funding-source gate and immutable submission
   snapshot. Extracting a larger pure screen-dispatch seam solely for
   orchestration testing is a maintainability choice, not a discovered
   production defect.
4. **Kotlin still trusts Rust's preview row keypair correspondence.** Swift
   independently revalidates private/public correspondence. Kotlin documents
   why it does not duplicate private-key math; whether to add an equivalent
   native validation surface remains a parity-hardening decision.
5. **No existing-identity backfill.** Already-created Android identities still
   need the Add Identity Key flow. This remains the reviewed non-goal.
6. **Dedicated provisioning helper.** Keeping `DashpayKeyProvisioning` is
   justified by the fixed registration IDs and one batch derivation. Commit
   `5147d5baa9` shares the dangerous persist-and-scrub primitive with
   `IdentityKeyAdditionFlow`, avoiding lifecycle duplication while preserving
   the distinct policies.

## Verification results

All required local verification passed:

```text
cargo test -p rs-unified-sdk-jni --lib
  29 passed

cargo test -p platform-wallet-ffi --lib
  201 passed

env RUSTC_WRAPPER= cargo clippy --workspace --all-features
  passed (the configured sccache was not permitted in the sandbox)

cargo fmt --check --all
  passed

JAVA_HOME=/opt/homebrew/opt/openjdk@17 \
ANDROID_HOME=/opt/homebrew/share/android-commandlinetools \
./gradlew :sdk:assembleDebug :sdk:testDebugUnitTest \
  :app:assembleDebug :app:testDebugUnitTest \
  :sdk:compileDebugAndroidTestKotlin
  passed
```

Testnet/device registration, the on-chain bounds assertion, and Add Contact
were intentionally not attempted because they are environment-bound.
