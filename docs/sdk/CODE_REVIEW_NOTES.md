# Code-review notes: Kotlin SDK PR #3999 follow-ups (Keystore + docs)

**Review date:** 2026-07-21
**Reviewed commits:** `6efa83bb53..ee484e97a1` (the 4 implementation commits) on
`fix/kotlin-sdk-pr3999-followups`, implementing
[`KOTLIN_SDK_PR3999_FOLLOWUPS_SPEC.md`](KOTLIN_SDK_PR3999_FOLLOWUPS_SPEC.md).
**Method:** final multi-agent code review (the mandatory last pipeline stage) —
four independent reviewer lenses (correctness/concurrency, Android-Keystore
security, test adequacy, doc accuracy), each re-derived from source, then
verified by building and running the tests on an arm64 emulator.

## What was verified

- `:sdk:assembleDebug :sdk:testDebugUnitTest :app:assembleDebug
  :app:testDebugUnitTest :sdk:compileDebugAndroidTestKotlin` — **BUILD
  SUCCESSFUL** (in-place; this worktree is on APFS, so the exFAT sparse-image
  redirect in the SDK CLAUDE.md is not needed).
- `:sdk:connectedDebugAndroidTest` on `kotlin_sdk_ci` (arm64-v8a, API 35) —
  **11 tests passed**, including all four new/changed cases:
  `mnemonicAndPathSignerSymbolLoadsAndSigns`,
  `canSignWithRejectsAKeyEncryptedUnderAReplacedKeysAlias`,
  `retrievePrivateKeyRejectsBlobFromReplacedKeysAlias`,
  `keysAliasEncryptionCarriesTheFingerprintOfItsEncryptionKey`.
- `python3 scripts/check_sdk_parity_manifest.py` — `18 capabilities; summary
  current`, exit 0 (still green after the reason-text edit).

## Reviewer conclusions

The four safety-critical spec claims are **correctly implemented**:

1. **Atomic encrypt+fingerprint (spec §4 step 6 — the trickiest part).**
   `KeystoreManager.encrypt()` captures the public key once into a local, inits
   the cipher with it, and computes the fingerprint from that *same* detached
   JCE `RSAPublicKey` object; `WalletStorage.storePrivateKeyEntryLocked`
   persists `blob.keyFingerprint`, not a fresh alias lookup. The mislabel race
   the independent codex review found (old-key ciphertext persisted with the
   new key's fingerprint) is **genuinely closed**, not merely narrowed — a
   concurrent rotation cannot mutate the captured key object, so the persisted
   `(ciphertext, fingerprint)` pair is internally consistent by construction.
2. **Rotation-under-lock on `KeyPermanentlyInvalidatedException`.** The catch is
   scoped to `cipher.init` only (not `doFinal`), deletes `KEYS_ALIAS` under
   `KEYS_ALIAS_LOCK`, and re-throws the typed exception (delete failures added
   as suppressed). See the one accepted-tradeoff caveat below.
3. **`retrievePrivateKey` fingerprint pre-check** uses a single `prefs`
   snapshot for both blob and stored fingerprint (no tear), and is honestly
   documented as TOCTOU/fail-closed.
4. **KPIE propagation skips the BiometricGate retry.**
   `KeyPermanentlyInvalidatedException` and `UserNotAuthenticatedException` are
   **siblings** (both extend `java.security.InvalidKeyException`; confirmed via
   `javap` against `android-35/android.jar`), so `retrieveKeyWithAuth`'s
   `catch (UserNotAuthenticatedException)` cannot swallow it — it propagates to
   a single fail-closed `completeSign(error)` with no prompt storm.

Android security posture is **sound**: deletion is not attacker-triggerable, the
blast radius is inherent (an invalidated private key already made every
`privkey.*` blob undecryptable), recovery survives because the mnemonic lives
under the non-auth `MASTER_ALIAS`, `canSignWith`'s `catch { false }` fails in
the safe direction, and the fingerprint carries no private material.

## Fixes applied in this review pass

These were clear defects (misleading test assertion, factually wrong doc text,
a weak test that didn't pin the property it guards). None touch production
crypto logic, so no red→green production test was required.

1. **`FfiSmokeTest.kt` — removed a tautological, spec-violating assertion.**
   The committed test ended with `assertArrayEquals(ByteArray(size),
   mnemonicUtf8)` — but that ran *after* the test's own `finally { fill(0) }`,
   so it only proved `fill(0)` works and said nothing about JNI. Worse, it
   re-introduced the exact "the native call scrubs the caller's array"
   impression that spec §6 explicitly said **not** to assert ("do not assert
   the native call zeroed it, because it doesn't and isn't supposed to").
   Removed the assertion and its now-unused import; the test still proves the
   real target (native symbol binds, returns a 65-byte compact recoverable
   signature) and still scrubs its own array as any correct caller must.
2. **`keysAliasEncryptionCarriesTheFingerprintOfItsEncryptionKey` — strengthened
   from a steady-state snapshot into a real regression guard.** As committed it
   only asserted `keysAliasFingerprint() == blob.keyFingerprint` with no
   rotation, so it would have passed even against the *pre-fix* separate-lookup
   design. It now rotates `KEYS_ALIAS` after encryption and asserts the blob's
   captured fingerprint no longer matches the current alias — pinning that the
   fingerprint is bound to the key *used at encrypt time* and is not a live
   re-read (the property that makes the mislabel race unreachable). Verified
   green on-device. (This does not reproduce the *concurrent* rotation-inside-
   `encrypt()` interleaving — see the coverage gap below — but it guards the
   observable contract of the new field.)
3. **`KOTLIN_MIGRATION_SPEC.md` — fixed a broken cross-reference.** The
   historical-baseline banner linked `[docs/sdk/PARITY_SUMMARY.md]
   (../sdk/PARITY_SUMMARY.md)`; that file does not exist. The real file is
   `packages/kotlin-sdk/PARITY_SUMMARY.md`; corrected both the label and the
   relative path (`../../packages/kotlin-sdk/PARITY_SUMMARY.md`).
4. **`sdk-parity-manifest.json` — tightened the `network.masternode_discovery`
   reason text.** The edit claimed `443` "remains only in the deprecated
   single-argument compatibility overload." Both qualifiers were imprecise:
   `443` is *also* the legitimate mainnet port on the live path
   (`defaultDapiPort` returns 443 for MAINNET, 1443 otherwise, `Sdk.kt:324-325`),
   and the single-arg overload (`Sdk.kt:269-270`) carries no `@Deprecated`
   annotation. Reworded to: the live path selects the port per network
   (443 mainnet / 1443 otherwise) and the hardcoded non-mainnet `443` survives
   only in the legacy single-argument compatibility overload. Checker still
   green.

## Left for human judgment (deliberately NOT changed unilaterally)

These are genuine findings, but each is a judgment call — an accepted-tradeoff
robustness nit in safety-critical crypto, or a coverage gap that cannot be
red→green tested without new plumbing. Per the repo pipeline, judgment calls are
surfaced here rather than fixed unilaterally.

### J1 — Unconditional `deleteEntry(KEYS_ALIAS)` in the invalidation catch (LOW, self-healing)

`KeystoreManager.kt:95-98` deletes the alias unconditionally under the lock,
without the double-check that `ensureKeysKeyPair` (`:235-243`) deliberately
performs before deleting. Two independent reviewers found the same narrow
interleaving: two threads both fetch the invalidated handle and both enter the
catch; the first deletes; a concurrent `encrypt()` regenerates a fresh valid
pair and stores a blob under it; the second thread then deletes that
**fresh, valid** alias, orphaning the just-stored ciphertext.

- **Impact:** one extra forced re-derivation. Fail-closed, no secret exposure,
  identity keys are re-derivable from the surviving mnemonic. The window is
  tiny (regenerate+encrypt must land between two deletes) and additionally
  requires a secure lock screen to have been re-enabled so regeneration can
  even succeed.
- **Both security and correctness reviewers explicitly called this an
  acceptable tradeoff under the re-derive recovery model and said not to block
  on it.** The spec's "the invalidation catch is one-shot" framing (§4 failure
  modes) understates it, but does not mislead about the outcome.
- **Why not fixed here:** it cannot be exercised without a concurrency seam
  (real permanent invalidation is device-bound and not reliably scriptable), so
  a fix would land in safety-critical deletion logic with no red→green test to
  prove it correct — higher regression risk than the self-healing nit it
  removes.
- **Proposed hardening if a maintainer wants it:** make the delete conditional —
  inside `KEYS_ALIAS_LOCK`, re-init a decrypt cipher against the *currently
  present* private key and delete only if it *still* throws
  `KeyPermanentlyInvalidatedException` (a valid replacement inits cleanly and is
  kept; an auth-expired replacement throws `UserNotAuthenticatedException` and
  is likewise kept). This mirrors `ensureKeysKeyPair`'s existing
  reuse-don't-delete discipline.

### J2 — Concurrent rotation/write atomicity is untested and not disclosed as deferred (LOW)

The atomic-fingerprint property is now guarded for the *sequential* case (fix #2
above), but the true *concurrent* interleaving — a rotation landing inside
`encrypt()` between key capture and the DataStore write — is structurally
guaranteed by the single-method capture, not by a test. The spec's device-bound
section discloses only the biometric-enrollment path as untestable; it does not
list this concurrent-rotation property. Recommend a one-line honest disclosure
in the spec's verification/deferred list (Rule 11 "fail loud"), or accepting the
structural guarantee as sufficient. No code change proposed — the property holds
by construction.

### J3 — `canSignWith` conflates "no key" with "device not ready" (INFORMATIONAL)

`catch (_: Exception) { false }` reports "not capable" both when no key is
stored and when Keystore access fails because no secure lock screen is
configured. For a boolean capability probe this is the correct (safe) direction,
and the actionable error still surfaces on the real sign attempt. Noted only as
a UX-messaging consideration; no code change warranted.

## Bottom line

The implementation faithfully delivers the reviewed spec, the hard part (the
atomic encrypt/fingerprint race fix) is correct, the security posture is sound,
and the build + all instrumented tests pass on-device. After the four fixes
above, the only open items are the three documented judgment calls, none of
which block. **Ready for a human to push and open the PR.**
