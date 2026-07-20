# Kotlin DashPay Migration — Follow-up Fixes Spec

**Historical status:** Items A, B, and C are resolved in the consolidated
migration; the completed work is recorded in
[`KOTLIN_MIGRATION_LEFTOVERS.md`](KOTLIN_MIGRATION_LEFTOVERS.md). Item D remains
deferred. The problem/approach sections below preserve the reviewed rationale
and pre-implementation baseline rather than describing current open work.

Scope: three DashPay follow-ups folded into the consolidated migration PR
(`feat/kotlin-sdk-dashpay-migration`, stacked on the base Kotlin SDK PR
`feat/kotlin-sdk-and-example-app`). A fourth item (durable contact-crypto
queue persistence) is **deferred** with rationale in §D.

All three items were verified **still open at the base PR HEAD** (`1fd86fbae5`)
and **not** addressed by the parent PR — none duplicates parent work.

---

## A. Platform-address signing: eliminate the JVM-String seed exposure

### Problem
`KeystoreSigner.signPlatformAddressOnDemand` retrieves the wallet mnemonic as a
`java.lang.String` (`WalletStorage.retrieveMnemonic`) and passes it across JNI to
`SignerNative.signWithMnemonicAndPath(mnemonic: String, …)`. An immutable `String`
cannot be scrubbed; the phrase sits on the JVM heap until GC (if ever),
recoverable from a heap dump. This is the exact anti-pattern the K2 resolver path
eliminated with `resolveMnemonicInto`, and it is explicitly flagged as the tracked
follow-up in three places (`KeystoreSigner.kt:151-157`, `SignerNative.kt:36-38`,
`signer.rs:436-437`). Every on-demand platform-address signature re-exposes the seed.

### Approach (out-buffer discipline, input direction)
Pass the phrase as a **scrubbable `ByteArray`** the caller owns and zeroes, so no
`String` of the seed ever exists on the signing path.

- **Rust JNI** (`packages/rs-unified-sdk-jni/src/signer.rs`): replace
  `Java_…_SignerNative_signWithMnemonicAndPath` with
  `Java_…_SignerNative_signWithMnemonicAndPathInto`; the mnemonic argument becomes
  `JByteArray`. Decode the non-secret args (path, payload) **first** and the
  mnemonic **last**. **Do NOT use `convert_byte_array` + `reserve_exact`** — that
  pattern orphans an unscrubbed plaintext heap copy: `convert_byte_array` returns a
  `cap==len==N` `Vec`, and `CString::new`'s NUL append then forces a growth realloc
  that frees the original buffer unscrubbed (real on Android's Scudo allocator).
  Instead keep the phrase in a `Zeroizing<Vec<u8>>` **end-to-end** — never launder
  it through a `CString` (whose `into_boxed_slice` shrink-to-fit can silently realloc
  and free the plaintext unscrubbed, and which sits outside any zeroize guard across
  the FFI call):
  ```rust
  let n = env.get_array_length(&mnemonic)? as usize;           // reject Err
  let mut plain: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(n + 1));
  plain.resize(n, 0);                                          // len=n, cap=n+1
  let dst = unsafe { slice::from_raw_parts_mut(plain.as_mut_ptr().cast::<i8>(), n) };
  env.get_byte_array_region(&mnemonic, 0, dst)?;              // reject Err
  if plain.contains(&0) { throw; return }                      // reject interior NUL
  plain.push(0);                                               // NUL-terminate in place, no realloc
  // pass plain.as_ptr().cast::<c_char>() to the FFI; drop(plain) scrubs the sole
  // copy after signing — and on any panic unwind, since it never leaves Zeroizing.
  ```
- **Kotlin FFI decl** (`ffi/SignerNative.kt`): replace the `external fun` with
  `signWithMnemonicAndPathInto(mnemonicUtf8: ByteArray, derivationPath: String,
  network: Int, data: ByteArray): ByteArray?`.
- **Caller** (`security/KeystoreSigner.kt`): switch
  `storage.retrieveMnemonic(...)` → `storage.retrieveMnemonicUtf8(...)` (returns
  `ByteArray?`, already exists) and wrap the sign in
  `try { … } finally { mnemonicUtf8.fill(0) }`. Delete the KNOWN-residual comment
  block (151-157) — it is now resolved.

The old String function has a **single caller** (verified), so it is removed
outright; no String seed path remains. The derived key still never crosses JNI —
Rust derives, signs, and scrubs internally, unchanged.

### Failure modes
- Interior NUL in the phrase bytes → `CString::new` fails → scrub the reclaimed
  `Vec`, throw (unchanged behavior).
- Empty byte array → `CString` of just NUL → the inner FFI's mnemonic parse fails →
  invalid-mnemonic throw. There is **no size cap** (a large array just allocates and
  then fails the parse); the caller's `retrieveMnemonicUtf8` only ever returns real
  phrase bytes or null, so this is a defense-in-depth path.
- Caller forgetting to scrub → mitigated by the `finally` block; the array is
  caller-owned per `retrieveMnemonicUtf8`'s contract. The Kotlin `null`-check must
  precede the `try` (nothing fallible between the retrieve and entering the `try`).
- Note: the JNI symbol name (`Java_…_signWithMnemonicAndPathInto`) must match the
  Kotlin `external fun` exactly — a mismatch is a runtime `UnsatisfiedLinkError`, not
  a compile error, and the JVM helper test won't catch it (pinned by `cargo check` +
  an instrumented/on-device sign smoke).

### Test plan (red→green)
`KeystoreSigner` cannot be constructed on the JVM tier (its constructor eagerly
calls native `createSigner`), and `WalletStorage` is a final class with no mock
framework in the module — so a seam *on the class* is untestable. Instead extract
a **pure `internal` scrub-and-sign helper** that owns the discipline:

```kotlin
internal inline fun signWithScrubbedMnemonic(
    mnemonicUtf8: ByteArray,
    derivationPath: String,
    network: Int,
    data: ByteArray,
    sign: (ByteArray, String, Int, ByteArray) -> ByteArray?,
): ByteArray? = try { sign(mnemonicUtf8, derivationPath, network, data) }
                finally { mnemonicUtf8.fill(0) }
```

`KeystoreSigner.signPlatformAddressOnDemand` calls it with
`sign = SignerNative::signWithMnemonicAndPathInto`. JVM test
(`SignerMnemonicScrubTest`): call the helper with a fake `sign` that records a
**copy** of the bytes it received and returns a dummy signature, then assert the
input array is all-zero afterward, and that the dummy signature is returned
(scrub happens after the sign, not before). Red→green is demonstrated by toggling
only the `finally { fill(0) }` — without it the array retains the phrase (red),
with it the array is zeroed (green). This pins the scrub invariant honestly (the
String-path removal itself is a structural guarantee, verified by the code + the
absence of any `retrieveMnemonic`/String-sign call on the signing path).
Plus `cargo check -p rs-unified-sdk-jni` for the Rust side.

---

## B. Payment-sheet dispose-mid-send: pin the double-send guard

### Problem
`SendDashPayPaymentSheet.send()` broadcasts a payment and then records the txid +
triggers the durability refresh (`onSent` → `refreshDashPayPayments`). The K3 fix
wraps the broadcast + bookkeeping in `withContext(NonCancellable)` (plus a Compose
dismissal gate in `ContactDetailScreen`) so a dispose-mid-send cannot skip the
durability refresh — which would otherwise invite a **double-send** on retry (the
JNI broadcast is uncancellable; the coin leaves the wallet regardless). This guard
has **no regression coverage**; a future refactor could drop `NonCancellable`
silently.

### Approach (extract the send body + inject the sender)
`send()` is a local function inside a `@Composable`, uncallable from `runTest`.
**Extract the coroutine body** (`SendDashPayPaymentSheet.kt:90-120`) into a
non-composable `internal suspend fun performDashPaySend(...)` that takes a minimal
`fun interface PaymentSender { suspend fun send(...): ByteArray? }` plus the
`onSent`/`onClose`/status callbacks. The composable calls it inside its existing
`scope.launch { … }`, so runtime behavior (including the launching Job) is
identical; the production `PaymentSender` closes over `wallet`/`manager` and calls
`w.dashpay.sendPayment(...)`. `performDashPaySend` keeps the `withContext(
NonCancellable) { sender.send(...); onSent() }` wrapper — the guard under test.

**Critical extraction constraints** (else the test is invalid):
- `performDashPaySend` is a plain `suspend fun` inheriting the caller's Job — it
  must NOT introduce a new `CoroutineScope`/`coroutineScope { }`, which would
  change the cancellation semantics being tested.
- The `CancellationException` rethrow stays; the best-effort tail
  (`kickDashPaySync`/`delay`/`onClose`) stays OUTSIDE the `NonCancellable` block.

### Test plan (red→green)
JVM `runTest` (`PerformDashPaySendDoubleSendGuardTest`): launch
`performDashPaySend` in a child `Job` with a fake `PaymentSender` that records the
broadcast **before** suspending on a test gate (models "broadcast completed,
bookkeeping pending" — the real hazard, not "cancelled before broadcast"):
1. launch the send; await the recorded broadcast entry;
2. cancel the child Job (simulate dispose-mid-send);
3. release the gate; join.
Assert **`onSent` fired exactly once** (count 0 pre-fix vs 1 post-fix — the
decisive assertion; the `NonCancellable` block completed despite cancellation).
Against the pre-fix code (no `NonCancellable`), cancellation observed on resume
after the broadcast skips `onSent` → count 0 → red. Assert on `onSent` count, NOT
`sendPayment` count (which is 1 either way).

The Compose **dismissal gate** (secondary, defense-in-depth) is left to the
existing instrumented UI tier; it is not the double-send regression and is not
deterministically JVM-testable without Robolectric (not configured).

---

## C. DataContractRef: add the NativeCleaner GC backstop

### Problem
`DataContractRef` (`queries/PlatformQueries.kt:621-633`) destroys its native handle
inline in `close()` but registers **no** `NativeCleaner`, so a leaked (never-closed,
never-`use{}`) ref leaks the native contract handle forever. Every other owned
handle — `Sdk`, `ManagedPlatformWallet`, and the K3 `ContactRequestRef`/
`EstablishedContactRef` — has the GC backstop. Consistency gap.

### Approach
Match the established idiom exactly: `import NativeCleaner`; register
`private val cleanable = NativeCleaner.register(this, HandleCleanup(handleRef))`;
`close()` → `cleanable.clean()`; standalone inner
`private class HandleCleanup(handleRef: AtomicLong) : Runnable` whose `run()` does
`handleRef.getAndSet(0)` then `QueriesNative.dataContractDestroy(h)` iff non-zero
(destroys exactly once, whichever of close/GC fires first). `AtomicLong handleRef`
and `value` are unchanged; no call sites change.

### Test plan
`close()` idempotency + registration are the testable surface; the native destroy
and GC-timing of the phantom backstop are not deterministically unit-testable. Rely
on parity with the already-shipped `ContactRequestRef` pattern + compile; add a
`NativeCleaner`-level idempotency assertion only if a fake action seam already
exists. (Noted as best-effort per the untestable-path carve-out.)

---

## D. Durable contact-crypto queue persistence — DEFERRED (own PR)

The DashPay deferred contact-crypto queue (`PlatformWalletChangeSet.
pending_contact_crypto_added/_cleared`) is not durable across process death on FFI
hosts. Research (see below) shows only the **write** half is cleanly addable; the
**restore** half is blocked upstream:

- The in-repo SQLite persister already writes the queue, but its reader is
  `#[cfg(test)]`-only "because production load restore is blocked upstream
  (`LOAD_UNIMPLEMENTED: ClientStartState::wallets`)".
- `rs-platform-wallet/src/wallet/apply.rs:111-116` drops the queue fields on the
  changeset-replay path; restore is meant to flow through the wallet **start-state**
  path, which isn't wired for this field.

Adding the FFI `on_persist_pending_contact_crypto_fn` slot + JNI trampoline + a
`DashDatabase` v3→v4 Room migration + Swift SwiftData model would persist rows that
**nothing reads back** — a large, irreversible, cross-cutting surface (including a
schema migration) for **zero cross-restart durability** until the upstream
start-state restore exists. This is worse than the current honest deferral: the
recurring signerless sweep already re-enqueues the work after restart, so the only
exposure is delayed (not lost) contact-crypto work between sweeps.

**Decision:** keep as a documented leftover; implement as a dedicated PR once the
`rs-platform-wallet` start-state restore path lands. Full data-path map (producers,
SQLite schema, FFI/JNI/Kotlin/Swift layers, the add-a-persisted-field recipe) is
recorded for that future PR.
