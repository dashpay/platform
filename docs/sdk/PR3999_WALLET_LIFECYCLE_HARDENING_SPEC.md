# PR #3999 — Kotlin SDK wallet-lifecycle hardening spec

Covers the 4 open blocking review findings on PR #3999
(`feat/kotlin-sdk-and-example-app`) that were not yet addressed:

1. `PlatformWalletManager.kt:347` — init cleanup starts after fallible
   child constructors.
2. `PlatformWalletManager.kt:778` — alias cleanup ignores ownership
   recorded only in another wallet's index.
3. `PlatformWalletManager.kt:783` — identity-key persistence can still
   run after wallet deletion.
4. `CreateWalletScreen.kt:85` — configuration changes discard the only
   recovery-phrase copy.

Findings 2 and 3 both live in the `removeWallet` / `WalletStorage`
private-key lifecycle and are designed together (§2). 1 and 4 are
independent (§1, §3).

## Spec review findings (applied)

Three independent review agents (feasibility, security, scope/simplicity)
checked a first draft of this spec against the actual source. §1 and
the overall scope of §2/§3 came back clean; three must-fix issues were
found and are already folded into the sections below:

1. **Compile-breaking (feasibility):** the original §2 sketch declared
   `isOwnedByAnotherWallet` as a `private` extension function on
   `PrivateKeyExclusion`. It's unreachable from `removeWallet`'s
   `withPrivateKeyExclusion { }` lambda (only the interface's own
   members resolve there) and `private` besides. Fixed: declared on
   the `PrivateKeyExclusion` interface itself, like the existing
   `deleteOwnerIndex`.
2. **Deadlock-risk (feasibility + security):** the original §2
   `storeIfAbsent` sketch called `derive()` (a Rust FFI call) *inside*
   `privateKeyMutex.withLock`, directly violating `WalletStorage.kt`'s
   own documented invariant that the locked block must never call into
   native code. Fixed: two-phase check → derive-outside-lock →
   re-check-and-store. The security pass also caught that the
   existence check must treat a present-but-undecryptable ciphertext
   blob (a real, already-supported legacy state — see
   `isPrivateKeyDecryptable`) as absent, or the fix would silently
   defeat that re-derive path.
3. **New risk introduced (security), inconsistent with the rest of the
   screen (feasibility):** the original §3 sketch also moved the
   wallet-creation coroutine's launch from `rememberCoroutineScope()`
   onto `viewModel.viewModelScope`. Feasibility flagged that the
   screen's other captured state (`isCreating`, `error`,
   `navController`) stays composable-scoped, so a `viewModelScope`
   coroutine surviving a config change would mutate/navigate through
   dead references. Security separately flagged that `viewModelScope`
   surviving longer than the composition (e.g. past navigation-away)
   combined with `onCleared()` scrubbing the phrase could wipe the
   *only* copy before the emergency dialog is ever shown, for a
   creation that fails after the user has already left the screen — a
   worse outcome than the bug being fixed. Fixed: keep
   `rememberCoroutineScope()` for the launch; only the *storage* of an
   already-caught phrase moves to the ViewModel.

Additionally, the security pass found one HIGH-severity gap the
feasibility/scope passes didn't have the angle to catch: the
tombstone-set design ("a deleted wallet's id is never reused") is
false — re-importing the same recovery phrase after an accidental
delete is a real, supported flow, and would permanently brick under
the original always-on tombstone. Fixed: `createWallet` (the same
entry point both "new" and "restore from phrase" go through) now
clears any stale tombstone for that wallet id before storing.

Two lower-severity items were surfaced and are intentionally *not*
fixed in this pass (see the "Accepted, not fixed" note in §2 and the
platform-limitation note in §3) — both are pre-existing or
theoretical, not regressions this spec would introduce, and closing
them would widen the diff for gaps that either have no live caller
today or are an accepted platform ceiling.

---

## §1. Init cleanup ordering (finding @ `PlatformWalletManager.kt:347`)

### Problem

`PlatformWalletManager`'s primary constructor initializes, in source
order: `scope` (192) → `teardownGate`/`_syncEvents`/`eventBridge`
(203-317) → `mnemonicResolver` (325) → `signer` (326-327) →
`identityKeyDeriver` (337-341) → `persistenceHandler` (343-347) → …
→ `nativeInitialization = initializePlatformWalletNativeManager(...)`
(477-490).

`initializePlatformWalletNativeManager` already contains a correct
cleanup transaction — on failure it cancels `scope` and closes
`mnemonicResolver`/`signer`/`persistenceHandler` (in that order,
suppressing secondary failures) before rethrowing. But it only guards
failures inside *itself* (native bundle create, manager-handle fetch,
capability reads). It cannot guard `mnemonicResolver`, `signer`, or
`persistenceHandler`'s own constructors, because those already ran to
completion (or threw) *before* `nativeInitialization` is reached — a
throw there aborts the whole primary constructor with no
`PlatformWalletManager` instance to call `close()` on, so:

- `mnemonicResolver`'s JNI handle (`MnemonicNative.createResolver`)
  leaks if `signer`, `identityKeyDeriver`, or `persistenceHandler`
  throws after it.
- `signer`'s JNI handle (`SignerNative.createSigner`) leaks if
  `identityKeyDeriver` or `persistenceHandler` throws after it (and
  `database.platformAddressDao()`, evaluated as `signer`'s 4th
  constructor arg, can itself throw before `signer`'s own constructor
  even runs).
- `persistenceHandler`'s owned single-thread `Executor`
  (`Executors.newSingleThreadExecutor { "dash-persistence" }`) leaks
  if it's the one that throws, or leaks the executor while everything
  *before* it (resolver, signer) also leaks.

`identityKeyDeriver` itself holds no releasable resource (confirmed:
plain Kotlin object, no native handle, not `AutoCloseable`).

### Chosen approach

Group the four child constructions into one `init`-time block that
builds each as a local `var`, wraps construction in `try`, and on any
`Throwable` closes whichever locals were already assigned (in reverse
construction order) before rethrowing — the same "roll back the
locals, only adopt on success" shape the Swift equivalent
(`PlatformWalletManager.swift`'s `configure(...)`) already uses,
adapted to Kotlin's val-in-constructor idiom via a small holder:

```kotlin
private class CoreChildren(
    val mnemonicResolver: MnemonicResolverAndPersister,
    val signer: KeystoreSigner,
    val identityKeyDeriver: IdentityKeyPrivateKeyDeriver,
    val persistenceHandler: PlatformWalletPersistenceHandler,
)

private val coreChildren: CoreChildren = run {
    var mnemonicResolver: MnemonicResolverAndPersister? = null
    var signer: KeystoreSigner? = null
    try {
        val resolver = MnemonicResolverAndPersister(walletStorage)
            .also { mnemonicResolver = it }
        val keySigner = KeystoreSigner(
            walletStorage, network, biometricGate, database.platformAddressDao(),
        ).also { signer = it }
        val deriver = IdentityKeyPrivateKeyDeriver(
            network = network,
            mnemonicResolverHandle = resolver.nativeHandle,
            walletStorage = walletStorage,
        )
        val handler = PlatformWalletPersistenceHandler(
            database = database, privateKeyDeriver = deriver, network = network,
        )
        CoreChildren(resolver, keySigner, deriver, handler)
    } catch (e: Throwable) {
        runCatching { signer?.close() }
        runCatching { mnemonicResolver?.close() }
        scope.cancel()
        throw e
    }
}
private val mnemonicResolver get() = coreChildren.mnemonicResolver
private val signer get() = coreChildren.signer
private val identityKeyDeriver get() = coreChildren.identityKeyDeriver
private val persistenceHandler get() = coreChildren.persistenceHandler
```

- `persistenceHandler` needs no explicit close-on-catch: if its own
  constructor throws, it never assigned itself anywhere, and it's the
  last child built, so there's nothing after it to fail and orphan it.
- `scope.cancel()` on the failure path mirrors what
  `initializePlatformWalletNativeManager`'s own cleanup already does
  for *its* failures; nothing has been launched on `scope` yet at this
  point (it's created earlier, at line 192, unused until later), so
  cancelling it here is cheap and keeps both failure paths consistent.
- All downstream references to `mnemonicResolver`, `signer`,
  `identityKeyDeriver`, `persistenceHandler` throughout the file are
  unaffected — they become `private val ... get() = ...` delegating
  properties instead of directly-initialized `private val`s, same
  read-only surface, no call-site changes.
- `nativeInitialization`'s own cleanup transaction (109-128) is
  untouched — it still guards its own failure window exactly as
  today.

### Alternatives rejected

- **Wrap the whole primary constructor body in try/catch.** Kotlin
  doesn't allow arbitrary try/catch around property initializers
  mixed with the constructor parameter list in a readable way, and it
  would force converting *every* property after this point (including
  `identityRegistration`, `voteCasting`, etc., none of which are
  fallible or hold resources) into part of the guarded region for no
  benefit — larger surface than necessary.
- **`private constructor` + `companion object { fun create(...) }`
  factory** (the pattern the finding suggests by analogy to Swift).
  Rejected for this specific class: every call site that currently
  does `PlatformWalletManager(...)` (`WalletManagerStore`'s factory
  lambda, tests) would need to change to `PlatformWalletManager.create(...)`,
  and the class already exposes a large public API assuming direct
  construction succeeds or throws synchronously — converting to an
  external factory is a bigger surface change for the same outcome as
  the local-var-holder approach above, which achieves the identical
  rollback guarantee without touching any call site.

### Failure modes covered / not covered

- Covered: any single child's constructor throwing, at any position in
  the four, cleans up every JNI-owning child constructed strictly
  before it.
- Not covered (explicitly out of scope for this finding, flagged for
  a separate follow-up): `KeystoreSigner.close()` itself does not
  cancel `KeystoreSigner`'s own internal `scope`
  (`CoroutineScope(SupervisorJob() + Dispatchers.IO)`, `KeystoreSigner.kt:53`)
  — that's a pre-existing gap in `KeystoreSigner`'s own `close()`
  logic, orthogonal to *when* cleanup runs (which is what this finding
  is about). Noting it here so it isn't lost, not fixing it in this
  pass to keep the diff surgical.

### Test plan

Add to `PlatformWalletManagerInitializationTest.kt` (or a new
`PlatformWalletManagerConstructionTest.kt` if constructing a real
`PlatformWalletManager` needs more fixture setup than that file
currently has): a test that injects a `walletStorage`/`database`/etc.
combination where `KeystoreSigner`'s construction throws (e.g. via a
fake `platformAddressDao()` or a `SignerNative.createSigner` stub that
throws — check what's fake-able without real JNI, per that file's
existing lambda-injection pattern for `nativeCreate`/`nativeManagerHandle`/etc.,
since `PlatformWalletManagerInitializationTest` already fakes native
calls at that granularity). Assert:
- The thrown exception propagates (construction still fails).
- `mnemonicResolver`'s `close()` was invoked (spy/count).
- No leaked JNI handle assertion is directly measurable from a JVM
  unit test without a real native lib loaded; the practical proof is
  "close() was called on every child constructed before the failing
  one," which is what the test asserts.

---

## §2. Cross-wallet private-key ownership (findings @ `:778`, `:783`)

### Shared root cause

`WalletStorage` (`sdk/src/main/kotlin/.../security/WalletStorage.kt`)
stores private-key ciphertext **globally**, keyed only by pubkey hex
(`privkey.<hex>`, no wallet-id component), because sibling network
wallets derived from one mnemonic (Testnet/Devnet/Regtest all share
DIP-9's non-mainnet derivation path) can legitimately derive the same
pubkey and are expected to share that one ciphertext entry. Ownership
is tracked **per-wallet** via a separate index (`privkeyowners.<walletIdHex>`
→ `Set<pubkeyHex>`), and all wallets share one process-wide
`WalletStorage` instance and one `privateKeyMutex`.

Two related gaps fall out of that design, both inside `removeWallet`
(`PlatformWalletManager.kt:718-791`):

**(a) finding @ `:778`.** `aliasesToDelete` (764-770) decides an alias
is safe to delete when no *other Room `public_keys` row* (a committed,
on-chain-registered key) references it outside this wallet's
identities. It never checks whether a *sibling wallet's durable owner
index* (`privkeyowners.<otherWalletIdHex>`) already claims the alias —
so a sibling wallet that pre-stored (but hasn't yet committed a
`public_keys` row for) the same shared alias loses its ciphertext when
this wallet is deleted.

**(b) finding @ `:783`.** Two independent gaps, not one:
- `PlatformWalletPersistenceHandler`'s own persist-callback path
  (`onPersistIdentityKeyUpsert` → `IdentityKeyPrivateKeyDeriver.hasStored`
  then `.deriveAndStore` → `WalletStorage.storePrivateKey`) *is*
  exclusion-fenced (`withCallbackExclusion`), but `hasStored` and the
  eventual `storePrivateKey` inside `deriveAndStore` are two separate
  calls with no single lock spanning both — a sibling caller can store
  between the check and the write.
- Two **app-level** call sites — `CreateIdentityScreen.kt:219-227`
  and `IdentityKeyAdditionFlow.kt:161-165` — call
  `walletStorage.storePrivateKey(...)` directly from their own
  `scope.launch`/coroutine, entirely outside
  `withPrivateKeyExclusion`/`withCallbackExclusion`/`teardownGate`.
  Neither `WalletStorage` nor `storePrivateKey` has any concept of
  "this wallet was just deleted," so a store that was already
  in-flight when `removeWallet` ran completes anyway, resurrecting the
  just-deleted wallet's owner-index entry with fresh ciphertext.

### Chosen approach

*(Revised after multi-agent spec review — see "Spec review findings"
below for what changed and why.)*

Three additions to `WalletStorage`, all executed under the existing
`privateKeyMutex` (via `withPrivateKeyExclusion`/its internal scope),
so no new lock is introduced:

**1. Cross-wallet ownership query**, used to fix `:778`. Added to the
`PrivateKeyExclusion` **interface** (not a private extension function
— extension functions can't see `WalletStorage`'s private members and
aren't reachable from inside a `withPrivateKeyExclusion { }` lambda,
where only the interface's own receiver is in scope), implemented in
`privateKeyExclusionScope` alongside the existing `deletePrivateKeys`/
`deleteOwnerIndex`:

```kotlin
interface PrivateKeyExclusion {
    suspend fun deletePrivateKeys(pubkeyHexes: Collection<String>)
    suspend fun deleteOwnerIndex(walletId: ByteArray)

    /** True if any wallet OTHER than [excludingWalletId] still claims
     *  [pubkeyHex] in its durable owner index. */
    suspend fun isOwnedByAnotherWallet(pubkeyHex: String, excludingWalletId: ByteArray): Boolean
}

private val privateKeyExclusionScope = object : PrivateKeyExclusion {
    // ...existing overrides...
    override suspend fun isOwnedByAnotherWallet(
        pubkeyHex: String,
        excludingWalletId: ByteArray,
    ): Boolean {
        val excludingHex = excludingWalletId.toHex()
        val prefs = store.data.first()
        return prefs.asMap().any { (key, value) ->
            key.name.startsWith(PRIVKEY_OWNERS_PREFIX) &&
                key.name.removePrefix(PRIVKEY_OWNERS_PREFIX) != excludingHex &&
                (value as? Set<*>)?.contains(pubkeyHex.lowercase()) == true
        }
    }
}
```

`removeWallet`'s `aliasesToDelete` computation (`PlatformWalletManager.kt:764-770`)
becomes:

```kotlin
val aliasesToDelete = buildList {
    for ((pubkeyHex, publicKeyData) in keysByPubkeyHex) {
        val referencedElsewhere = database.publicKeyDao()
            .countReferencesOutsideIdentities(publicKeyData, ownedIdentityIds) > 0
        val ownedElsewhere = isOwnedByAnotherWallet(pubkeyHex, walletId)
        if (!referencedElsewhere && !ownedElsewhere) add(pubkeyHex)
    }
}
```

Already runs inside `walletStorage.withPrivateKeyExclusion { ... }`
(736) — as an interface method, `isOwnedByAnotherWallet` resolves on
that lambda's `PrivateKeyExclusion` receiver directly, same as
`deleteOwnerIndex` does today, keeping it under the same lock as the
delete that follows — no TOCTOU between the check and
`deletePrivateKeys`/`deleteOwnerIndex`.

**2. Atomic-enough check-and-store**, used to fix the `hasStored`/
`deriveAndStore` half of `:783` (and the same race underlies `:778`'s
"rollback path" note). **Not a single lock-held call** — `WalletStorage.kt`'s
own documented invariant on `withPrivateKeyExclusion` is explicit: *"must
also never call into native code (a persistence callback parked on this
lock can be holding native locks)."* `derive()` is a Rust FFI call
(`IdentityNative.deriveIdentityPrivateKeyWithResolver`), so it cannot run
inside `privateKeyMutex.withLock`. Instead, a double-checked pattern —
lock only for the existence check/write, derive in between, re-check
before writing:

```kotlin
/** If [pubkeyHex] has no *usable* stored ciphertext (absent, or present
 *  but undecryptable — see [isPrivateKeyDecryptable]), derive it via
 *  [derive] and store it; either way record [ownerWalletId] in the
 *  owner index. Returns whether a derive+store actually happened. */
suspend fun storeIfAbsent(
    pubkeyHex: String,
    ownerWalletId: ByteArray,
    derive: suspend () -> ByteArray,
): Boolean {
    // Fast path: already usable under any owner — just record ownership.
    if (privateKeyMutex.withLock { addOwnerIfUsableLocked(pubkeyHex, ownerWalletId) }) {
        return false
    }
    // Derive OUTSIDE the lock (native call) — another writer may store
    // the same alias while this runs.
    val derived = derive()
    return privateKeyMutex.withLock {
        if (addOwnerIfUsableLocked(pubkeyHex, ownerWalletId)) {
            false // lost the race while deriving; the winner's copy stands
        } else {
            storePrivateKeyLocked(pubkeyHex, derived, ownerWalletId)
            true
        }
    }
}
```

`addOwnerIfUsableLocked` treats "present but not
`isPrivateKeyDecryptable`" the same as absent (so the legacy-blob
re-derive path this codebase already supports keeps working — see
"Spec review findings" #2) — only a present-and-decryptable entry
short-circuits to "just add ownership." `IdentityKeyPrivateKeyDeriver.deriveAndStore`
calls `storeIfAbsent` instead of the current separate
`hasStored`/`storePrivateKey` pair; `PlatformWalletPersistenceHandler`'s
`existedBefore` becomes `!storeIfAbsent(...)`'s result directly,
removing the `runCatching { hasStored }.getOrDefault(true)` fallback
entirely (a genuine simplification, not just a safety fix). This isn't
a single atomic transaction (derivation still happens outside the
lock, matching `storePrivateKey`'s existing outside-the-lock derive
pattern today), but it closes the specific gap the finding names: the
existence check and the eventual write are no longer two calls with an
unguarded window where a sibling wallet's write is invisible to the
first check *and* silently overwritten by the second.

**3. Deletion tombstone**, used to fix the app-level-bypass half of
`:783`:

```kotlin
// WalletStorage — guarded by privateKeyMutex. Process-lifetime, but
// explicitly cleared on wallet (re-)creation (see below) — a deleted
// wallet's id CAN be reused within one process (re-import of the same
// recovery phrase after an accidental delete is a real, supported
// flow), so this must not be "set once, never cleared."
private val tombstonedWalletIds = mutableSetOf<String>()
```

`removeWallet`'s locked section additionally calls
`walletStorage.tombstoneWallet(walletId)` (same
`withPrivateKeyExclusion` block, right alongside `deleteOwnerIndex`,
so tombstoning happens atomically with the alias cleanup it's
protecting). `PlatformWalletManager.createWallet` (`:541-...`, the
single entry point for both "new wallet" and "restore/re-import from
mnemonic" — same function, both paths pass a mnemonic) calls
`walletStorage.clearTombstone(walletId)` right after the native create
succeeds and before `storeMnemonic`, so a re-imported wallet with a
previously-tombstoned id is immediately usable again, matching
`storePrivateKey`'s pre-existing "keyed globally by wallet id,
deterministic from seed+network" contract.

`storePrivateKey` (and the new `storeIfAbsent`, via the same
lock-held check) reject writes for a tombstoned `ownerWalletId`,
throwing a new `WalletTombstonedException(walletId)` — the two
app-level call sites (`CreateIdentityScreen.kt:219-227`,
`IdentityKeyAdditionFlow.kt:161-165`) need to catch it. **Accepted,
not fixed, in this pass:** `storePrivateKey`'s `ownerWalletId` param is
nullable (`= null`); a null-owner call bypasses both the tombstone
check and the owner-index union `removeWallet` reads. No current
caller on the derive/register path passes null, so this is a
theoretical gap, not a live bug — noted rather than closed by, e.g.,
making the parameter non-nullable (a wider API change touching every
call site for a path nothing currently exercises). What they *do* on
catch is a product decision, not purely mechanical:

- `CreateIdentityScreen`: the key was already derived and the identity
  isn't registered yet — the safest behavior is to abort registration
  with a clear "wallet was removed during setup" error, since there's
  no wallet left to register against.
- `IdentityKeyAdditionFlow`: adding a key to an *existing* identity on
  a now-deleted wallet — same abort-with-error behavior; the identity
  itself is decoupled from the wallet's local state at this point.

**I want to confirm that "abort with a clear error, don't silently
drop it" is the right call for both sites before implementing** — it's
the conservative default but it's the one piece of this section that's
a product/UX decision rather than a pure correctness fix, so flagging
it explicitly for sync rather than assuming.

### Alternatives rejected

- **Namespace ciphertext storage per-wallet** (so sibling wallets each
  get their own copy instead of sharing one `privkey.<hex>` entry).
  Rejected: this is a deliberate existing design choice (DIP-9 sibling
  wallets sharing one mnemonic are expected to share key material by
  construction), not a bug — re-namespacing would be a much larger,
  riskier storage-format migration for a problem the ownership-index
  fix already solves without touching the ciphertext layer.
- **A single process-wide "wallet lifecycle" lock instead of the
  targeted tombstone check.** Rejected: `privateKeyMutex` already
  serializes every `WalletStorage` mutation; adding a *second*,
  coarser lock spanning arbitrary app-level coroutines
  (registration flows, key-addition flows) would be far more invasive
  and risks new deadlocks between `withCallbackExclusion` (native
  callback path) and app-driven UI coroutines. The tombstone check
  reuses the existing lock and only adds a rejection condition.

### Failure modes

- `isOwnedByAnotherWallet` false positive/negative: a false negative
  (misses a legitimate other-wallet claim) reproduces today's bug; a
  false positive (over-retains an alias nobody else needs) only costs
  a stray ciphertext entry, not a lost key — asymmetric risk correctly
  favors retention.
- `storeIfAbsent` racing two *concurrent* callers for the *same* new
  pubkey: the mutex serializes them, so the second caller's `derive()`
  result is discarded once `hasPrivateKeyLocked` sees the first
  caller's write — matches today's `deriveAndStore`'s intended
  idempotency, now actually atomic.
- Tombstone check: a wallet ID is only ever tombstoned by `removeWallet`
  itself, under the same lock as the check, so there's no window where
  a store could race the tombstone write.

### Test plan

New `WalletStorageTest.kt` (currently doesn't exist — flagged as a gap
by the research pass):
- `isOwnedByAnotherWallet` returns true when wallet B's owner index
  contains the alias and wallet A is being deleted; false when no
  other wallet claims it.
- `storeIfAbsent` returns `false` and does not overwrite ciphertext
  when the pubkey already has an entry (from any owner); returns
  `true` and writes on first store; two concurrent calls for the same
  new pubkey result in exactly one ciphertext write.
- `storePrivateKey`/`storeIfAbsent` throw `WalletTombstonedException`
  for a tombstoned wallet id; a non-tombstoned sibling wallet's calls
  are unaffected.

Extend `removeWallet`'s coverage (no dedicated test file currently
exercises `removeWallet` at all — another gap flagged by the research
pass) with a sibling-wallet scenario: wallet A and B share a derived
pubkey via B's *pending* (not-yet-committed) owner-index entry; delete
A; assert B's ciphertext and owner-index entry survive.

---

## §3. `CreateWalletScreen` mnemonic loss on config change (finding @ `:85`)

### Problem

`unrecoverablePhrase` (the sole surviving copy of a mnemonic when every
durable store *and* rollback attempt failed, carried by
`WalletCreateRollbackException.mnemonic`) is held in plain
`remember { mutableStateOf<String?>(null) }` inside the `@Composable`.
That's deliberately not `rememberSaveable` (so the plaintext never
serializes into the saved-state `Bundle` — correct instinct) but as a
result it also doesn't survive activity recreation from rotation,
locale, or theme changes, which discards the composition and loses the
only copy — the underlying wallet creation already ran, Room rows
exist, but they're now permanently seedless with no recovery path.

There is no existing ViewModel for this screen; wallet creation runs
on `rememberCoroutineScope()`, not `viewModelScope`.

### Chosen approach

*(Revised after multi-agent spec review — the coroutine-scope change
originally proposed here was dropped; see "Spec review findings"
below.)*

Add `CreateWalletViewModel : ViewModel()` holding
`unrecoverablePhrase` as a plain `mutableStateOf<String?>` property —
the same shape `TokenActionViewModel` already establishes elsewhere in
this app module (plain `ViewModel`-retained `mutableStateOf`, not
`SavedStateHandle`-backed) — so it survives config-change-driven
recreation via the normal `viewModel()` retention contract, without
ever touching `SavedStateHandle`/the Bundle:

```kotlin
class CreateWalletViewModel : ViewModel() {
    var unrecoverablePhrase by mutableStateOf<String?>(null)
        private set

    fun recordUnrecoverablePhrase(phrase: String) {
        unrecoverablePhrase = phrase
    }

    /** Explicit scrub once the user has acknowledged the backup dialog. */
    fun clearUnrecoverablePhrase() {
        unrecoverablePhrase = null
    }

    override fun onCleared() {
        unrecoverablePhrase = null
        super.onCleared()
    }
}
```

`CreateWalletScreen` takes `viewModel: CreateWalletViewModel =
viewModel()` (standard Compose factory). **The wallet-creation
coroutine itself keeps launching on `rememberCoroutineScope()`, as
today** — only *where the caught phrase is stored* changes. The
`WalletCreateRollbackException` catch calls
`viewModel.recordUnrecoverablePhrase(phrase)` instead of assigning
local `remember` state; the emergency dialog reads
`viewModel.unrecoverablePhrase` and calls
`viewModel.clearUnrecoverablePhrase()` on acknowledgement — same UX,
same non-dismissable-until-acknowledged shape, just backed by
ViewModel state instead of composition-scoped `remember`. This is
deliberately the smallest change that closes the named finding: the
finding is about the phrase being lost *after* it was already
successfully caught (a later rotation wipes the composition's
`remember` state); it is not about the creation coroutine surviving
mid-flight cancellation, which is a different (and not reported)
concern. Moving the *launch* itself onto `viewModelScope` was
considered and rejected — see "Spec review findings" #3.

`String` immutability means true byte-level zeroization isn't possible
here (same platform limitation the finding implicitly accepts by
saying "explicitly scrubbed... field," not "zeroized bytes") — the
`clear`/`onCleared` calls null the reference promptly, which is the
practical ceiling for a JVM `String`. No existing code in this app
module holds secrets in a `ViewModel`, so this establishes a new (but
narrow, single-field) pattern rather than reusing one — noted for
awareness, not a blocker.

### Alternatives rejected

- **`rememberSaveable` with a custom `Saver` that encrypts before
  serializing.** Rejected: still writes ciphertext into the
  saved-state `Bundle`, which can be included in Android's automatic
  backup / restored on a different security surface than intended;
  the finding explicitly calls for avoiding `SavedStateHandle`/Bundle
  entirely, and `ViewModel` retention already solves the actual
  problem (surviving config change) without that exposure.
- **Process-level singleton / repository holding the phrase.**
  Rejected: broader lifetime than needed (a `ViewModel` scoped to this
  screen's `NavBackStackEntry` already outlives config changes and is
  cleared on real navigation-away, which is the right lifetime — a
  singleton would need its own manual clearing discipline for no
  benefit).

### Test plan

Kotlin unit test (Robolectric not required — pure `ViewModel` logic,
no Room/Compose): `CreateWalletViewModelTest.kt` —
`recordUnrecoverablePhrase` sets the field;
`clearUnrecoverablePhrase`/`onCleared()` (via
`ViewModelStore.clear()`) null it. This is a pure-JVM test unlike the
findings in §1/§2, so it's the one item in this spec I can actually
compile-check locally now that JDK 17 + the Android SDK were located
on this machine — I'll run `./gradlew :app:testDebugUnitTest` for it
alongside the others.

---

## Cross-cutting test plan

All four items get real `./gradlew :sdk:testDebugUnitTest
:app:testDebugUnitTest` runs before this is called done — the earlier
report that I couldn't test Kotlin locally was wrong; JDK 17
(`openjdk@17` via Homebrew) and the Android SDK (`platforms/android-35`,
`build-tools/35.0.0`) are both present on this machine, just not on
the default `java_home`/`ANDROID_HOME` search path this session
started with.

## Open questions for sync (before coding)

1. §2, tombstone rejection UX: confirm "abort registration/key-add
   with a clear error" is correct for both
   `CreateIdentityScreen`/`IdentityKeyAdditionFlow`, vs. some retry/
   silent-skip behavior.
2. §2 is the largest, most fund/key-safety-critical piece of this
   spec (touches `WalletStorage`'s locking and storage format
   indirectly via the new tombstone set). Confirm you want it done in
   this pass rather than split into its own follow-up PR after #3999
   lands — it's the one item where "ship it now" vs. "land the other 3
   and follow up" is a real tradeoff given the review-cycle history on
   this branch already (140 review passes, most from automated
   reviewers finding new issues on each push).
3. §2, accepted gap: `storePrivateKey`'s nullable `ownerWalletId`
   bypasses the tombstone/owner-index union entirely for a null-owner
   call. No current caller passes null on the derive/register path, so
   this is flagged rather than closed (closing it means auditing every
   `storePrivateKey` call site to require an owner). Confirm you're
   fine leaving this as a documented gap rather than widening the API
   change to cover it now.
