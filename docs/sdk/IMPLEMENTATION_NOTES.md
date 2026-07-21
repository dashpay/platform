# Kotlin SDK PR #3999 follow-ups — implementation notes

## Generation-aware invalidation cleanup

The post-implementation review found an interleaving not covered by the
finalized spec's unconditional `deleteEntry(KEYS_ALIAS)` approach: two
decryptors can hold the same invalidated private-key handle, the first can
delete it, and a writer can generate a replacement before the second decryptor
reaches cleanup. Unconditional cleanup by the second decryptor would then delete
the replacement and orphan ciphertext the writer just persisted.

After explicit authorization to fix the review findings, cleanup was made
generation-aware. The private-key handle is paired with the fingerprint of its
public key. Under `KEYS_ALIAS_LOCK`, cleanup deletes the alias only when the
current certificate still has that fingerprint. This differs from the review
note's alternative of re-initializing a cipher against the current private key:
fingerprint comparison identifies the generation without making the cleanup
decision depend on the device's current authentication state, and exposes a
deterministic test seam.

`staleInvalidationCleanupDoesNotDeleteAReplacementKeysAlias` was run on the
arm64 emulator with the seam wired to the old unconditional behavior (red), then
with generation matching (green).

## Verification limits

The complete Gradle build/unit/androidTest-compilation gate and parity-manifest
checker pass. The full connected suite reports 29 tests: 21 passed, 3 testnet
tests skipped, and 5 existing wallet tests failed because Android Keystore
reported `DEVICE_LOCKED` (`-72`) for the emulator. The new invalidation-cleanup,
atomic fingerprint, signer-capability, and mnemonic-path JNI tests all passed in
that run. No device credential was entered or changed.

The real biometric-enrollment/secure-lock change that produces
`KeyPermanentlyInvalidatedException` remains the manual device-bound gate called
out by the spec.
