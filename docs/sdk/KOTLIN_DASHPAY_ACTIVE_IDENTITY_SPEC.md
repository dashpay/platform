# KotlinExampleApp DashPay active identity persistence

Status: reviewed, aligned, implemented, and verified (rev 2).

Scope: APP4 / H3 from `IOS_KOTLIN_PARITY_PLAN.md`. KotlinExampleApp only.
DashPay invitations are explicitly out of scope: no invitation worktree, internals,
or Room schema changes.

> **Review outcome.** Four independent in-process reviewers audited rev 1 for
> coherence, feasibility, interaction-state coverage, and adversarial failure modes.
> The cross-model review route was also attempted, but the sandbox could not resolve
> Anthropic's API host (`ENOTFOUND`), so it produced no findings. Rev 2 folds in every
> actionable in-process finding: a recoverable post-bootstrap restoration state;
> refresh and picker-write gating; a production-wiring restart test; an explicit
> load-before-reconcile test; and awaited DataStore teardown before disk
> reconstruction. The coherence reviewer found no internal contradictions.

## 1. Problem

`DashPayTabScreen` keeps the selected identity in
`remember(network) { mutableStateOf(null) }`. The picker therefore remembers a choice
only while that composition stays alive. After a real process restart it selects the
first eligible identity, even if the user chose another identity before exiting.

A stored identity cannot be trusted blindly. It may have become invalid because:

- the identity row no longer has a wallet owner;
- its wallet is no longer present in the active network manager; or
- the identity is otherwise absent from the same eligible list the DashPay tab uses.

The current startup order matters. `WalletManagerStore.activate` publishes a manager
before `loadPersistedWallets()` finishes populating its wallet map. Validating a stored
identity as soon as that manager appears could therefore erase a valid choice during a
normal cold start or network switch.

## 2. Goals and acceptance criteria

1. A user-selected DashPay identity survives a real process restart.
2. The selection is isolated per `Network`; changing one network never overwrites or
   clears another network's selection.
3. A restored id is accepted only when it is in the exact eligible list used by the
   screen: a wallet-owned identity whose wallet is loaded by the active network manager.
4. A restored id that is no longer eligible is removed from durable storage, and the UI
   falls back to the first eligible identity.
5. Validation waits until wallet restoration for that network has completed.
6. Tests deterministically prove the disk-backed restart, network isolation, valid
   restoration, stale/wallet-ineligible clearing, and the identity rendered by the
   picker.
7. No invitation or Room schema surface changes.

## 3. Research and precedents

- KotlinExampleApp already owns one process-wide Preferences DataStore,
  `example_prefs`, in `AppContainer`. `AppState` uses it for durable network and app
  preferences.
- The app's `DashPayContactMetaStore` uses SharedPreferences for older device-local
  metadata, but Android now recommends DataStore for new small durable state:
  <https://developer.android.com/topic/libraries/architecture/datastore>.
- Android documents `remember` as composition-only and notes that saved UI state is not
  retained when the user dismisses the activity; persistent storage is the appropriate
  backing for a preference expected to survive a real relaunch:
  <https://developer.android.com/develop/ui/compose/state-saving>.
- Swift's `DashPayTabView` stores the Base58 identity id in `@AppStorage`, prefers a
  matching eligible identity, and falls back to the first eligible identity. Kotlin
  needs the same user result, with the APP4 requirement adding per-network isolation and
  stale-value removal.
- `AppContainer.activateManager()` already provides the safe ordering boundary:
  `activate` → `loadPersistedWallets` → bind services. Eligibility validation belongs
  immediately after `loadPersistedWallets`, not in an eager Compose effect.

## 4. Chosen design

### 4.1 A small DataStore-backed selection store

Add an app-internal `DashPayActiveIdentityStore` using the existing
`AppContainer.dataStore`; do not create another DataStore instance.

Its durable key is:

```text
dashpay.activeIdentityId.<networkName>
```

`Network.networkName` is already documented as the canonical persistence-key name and
is more stable/readable than an FFI ordinal.

The store exposes only the durable operations APP4 needs:

```kotlin
fun observe(network: Network): Flow<DashPayActiveIdentityPreference>
suspend fun select(network: Network, identityIdBase58: String)
suspend fun clearIfIneligible(
    network: Network,
    eligibleIdentityIdsBase58: Set<String>,
)
```

`observe` emits `Loading` before the DataStore's first value,
`Ready(id-or-null)` afterward, and `Failed(cause)` if the preference read fails.
This prevents the UI from briefly rendering the first identity while a valid restored
choice is still being read and gives post-bootstrap I/O failures a visible,
retryable state.

`clearIfIneligible` performs one atomic DataStore `edit`: it reads the current value
inside the transaction and removes it only if it is absent from the supplied eligible
set. Reading inside the edit avoids a stale-check/new-selection race.

### 4.2 One shared eligibility function

Extract the current eligibility calculation into an internal pure function used by both
startup reconciliation and `DashPayTabScreen`:

```kotlin
eligibleDashPayIdentities(
    walletOwnedIdentities,
    loadedWalletIdsHex,
)
```

It preserves current behavior exactly:

- input rows come from `IdentityDao.observeWalletOwnedByNetwork`, so the Room relationship
  is present and network-scoped;
- the owning wallet id must also exist in the active manager's loaded wallet map; and
- results stay sorted by `createdAt` for stable fallback/picker order.

APP4 does not broaden or redefine DashPay identity eligibility. The extraction prevents
startup reconciliation and rendering from drifting apart.

### 4.3 Reconcile only after wallet restoration

`AppContainer` owns the store plus a small
`DashPayActiveIdentityRestorationCoordinator`. The coordinator exposes a
network-scoped `StateFlow` with `Loading`, `Ready`, and `Failed(cause)` states and
owns this strict sequence inside `activateManager()`:

1. activate the network-locked manager;
2. load persisted wallets completely;
3. take the current network's wallet-owned identity snapshot from the existing DAO Flow;
4. compute eligibility with the shared function and the now-restored wallet map;
5. atomically clear a stored id if it is not eligible; then
6. publish that this network's active-identity restoration is ready and continue the
   existing service binding.

The coordinator, rather than `AppContainer` prose ordering alone, is the test seam for
the load-before-reconcile invariant. It takes the concrete wallet-load and
identity-snapshot operations as suspending collaborators, keeps the network in
`Loading` while the wallet load is suspended, and never calls
`clearIfIneligible` before that load completes.

The restoration state is in-memory and network-scoped. It enters `Loading` before each
manager activation and reaches `Ready` only after reconciliation. Initial bootstrap
activation failures continue to flow into `BootstrapState.Failed`. After bootstrap,
the long-lived SDK collector catches activation failures so collection continues; the
coordinator remains `Failed` for that network and the DashPay screen offers retry
instead of hanging forever or cancelling future activations.

`DashPayTabScreen` renders a content-blocking indeterminate progress state with a
readable status label until:

- the manager's network matches `AppState.currentNetwork`;
- that network is marked restoration-ready; and
- the preference has emitted its first DataStore value.

This avoids three transient errors: validating against an empty wallet map, using the old
network's manager, and flashing the first identity before the stored id arrives.
Both refresh entry points are disabled or guarded while restoration is not ready: the
toolbar button is disabled and pull-to-refresh cannot call a mismatched or partially
restored manager.

If no identity is eligible after a completed restore, the persisted id is removed and
the existing “No identities yet” or “No wallet loaded” state is rendered.

If restoration or preference observation fails after bootstrap, the content-blocking
state shows a concise error and Retry action. Retry re-runs manager activation when the
coordinator failed, or re-subscribes to the preference Flow when observation failed.

### 4.4 Screen selection flow

After readiness:

1. derive `eligible` with the shared helper;
2. if the stored Base58 id matches an eligible identity, use it;
3. otherwise use the first eligible identity (the stale value was already removed during
   reconciliation); and
4. on picker selection, disable the picker while `store.select` runs and keep the
   currently confirmed identity visible;
5. let the preference Flow's `Ready(selectedBase58)` emission confirm and render the new
   selection; and
6. on a picker-write failure, keep the prior identity and show the existing
   `ErrorAlertDialog`.

The UI continues to pass the active identity's id to Contacts, Requests, Add Contact,
Profile, Ignored, and Hidden routes exactly as it does today.

The preference-to-active-identity calculation lives in a production composable state
helper used by `DashPayTabScreen`. The instrumentation test calls this same helper over
a reconstructed disk-backed store; it does not inject an already-resolved active
identity into an isolated picker.

## 5. Alternatives rejected

### SharedPreferences

It would provide synchronous reads and mirror the older contact metadata store, but it
would introduce a second preference mechanism for new state. Android explicitly
recommends DataStore for new small durable values, and KotlinExampleApp already has the
correct singleton DataStore.

### `rememberSaveable` / `SavedStateHandle`

These are suitable for transient UI restoration but are not the durable per-network
preference APP4 requires. In particular, saved state is not retained after a
user-dismissed activity, and it would not provide a natural stale-value cleanup point.

### Room column or table

The selection is a tiny app preference, not relational SDK state. Room would require a
schema version and migrations and would violate the explicit “no invitation/Room schema”
boundary for no benefit.

### Clear stale values directly from Compose

An effect watching `eligible` looks smaller, but the manager publishes before its wallets
finish restoring and the DAO collector initially has no value. Such an effect can erase a
valid id during cold start or a network switch. Post-`loadPersistedWallets` reconciliation
is the first trustworthy boundary.

### Keep the stale value and only fall back

That matches current Swift fallback behavior but not APP4's explicit clearing
requirement. It would also retry a known-invalid selection on every launch.

## 6. Files and interfaces

Expected production changes:

- `packages/kotlin-sdk/KotlinExampleApp/app/src/main/java/org/dashfoundation/example/ui/dashpay/DashPayActiveIdentityStore.kt`
  - DataStore adapter, preference load state, shared eligibility/selection helpers, and
    the restoration coordinator that enforces load-before-reconcile ordering.
- `packages/kotlin-sdk/KotlinExampleApp/app/src/main/java/org/dashfoundation/example/di/AppContainer.kt`
  - own the store/coordinator; add the post-wallet-load reconciliation boundary and
    catch post-bootstrap activation failures without terminating the SDK collector.
- `packages/kotlin-sdk/KotlinExampleApp/app/src/main/java/org/dashfoundation/example/ui/dashpay/DashPayTabScreen.kt`
  - observe the durable selection/restoration state; gate refresh; retry failures;
    persist picker changes with pending/error handling; expose the production selection
    state helper used by the UI test.
- `packages/kotlin-sdk/KotlinExampleApp/app/src/main/java/org/dashfoundation/example/ui/components/AccessiblePicker.kt`
  - add a defaulted `enabled` parameter so an in-flight durable selection cannot accept
    another picker action; existing callers remain unchanged.

Expected tests:

- `packages/kotlin-sdk/KotlinExampleApp/app/src/test/java/org/dashfoundation/example/ui/dashpay/DashPayActiveIdentityStoreTest.kt`
- `packages/kotlin-sdk/KotlinExampleApp/app/src/androidTest/java/org/dashfoundation/example/DashPayActiveIdentityUITest.kt`

No Gradle dependency, DAO, entity, database version, migration, JNI, Rust, Swift, or
invitation changes are expected.

## 7. Failure modes and handling

- **Preference not loaded yet:** show the blocking selection loading state, disable
  refresh, and never choose the first identity speculatively.
- **Preference observation fails:** show a retryable screen error; do not guess an active
  identity.
- **Manager belongs to another network:** remain loading; never read or clear using the
  wrong wallet set.
- **Manager wallet restore incomplete:** readiness stays false until reconciliation.
- **Stored id malformed or unknown:** it cannot match an eligible Base58 id, so the atomic
  reconciliation removes it.
- **Stored id's identity row became wallet-orphaned:** the wallet-owned DAO query omits it,
  so reconciliation removes it.
- **Owning wallet is not restored/loaded:** shared eligibility excludes it, so
  reconciliation removes it.
- **Initial reconciliation write failure:** rethrow into `BootstrapState.Failed`.
- **Post-bootstrap reconciliation/activation failure:** the SDK collector catches it,
  preserves collection, and leaves a retryable network-scoped `Failed` restoration
  state.
- **Picker write failure:** keep the previously confirmed identity selected, re-enable
  the picker, and show `ErrorAlertDialog`.
- **No eligible identities:** clear any stored id and preserve the existing empty state.
- **Concurrent picker write and reconciliation:** DataStore serializes edits, and
  reconciliation checks the transaction's current value against the current eligible set.

## 8. TDD and verification plan

Follow the repository bug-fix discipline:

1. Add the named tests first and run the targeted app unit test task. The unfixed tree
   must fail before production code is added.
2. Implement the store/readiness/screen wiring.
3. Re-run the same tests and capture the red-to-green transition.

Deterministic JVM tests:

1. **`chosen identity survives DataStore reconstruction and remains active`**
   - create a Preferences DataStore on a temporary disk file;
   - select the second of two eligible identities on Testnet;
   - cancel and await the first DataStore scope's completion to model process teardown
     and avoid two live DataStore instances for one file;
   - create a new DataStore/store instance over the same file;
   - reconcile and assert the second identity resolves active, not the first.
2. **`selections are isolated per network`**
   - persist different ids for Mainnet and Testnet and assert each restores independently.
3. **`stale restored identity is cleared and falls back`**
   - persist id B, reconstruct the store with only A eligible, reconcile, assert A is
     active and the Testnet preference is absent afterward.
4. **`restored identity without a loaded owning wallet is cleared`**
   - keep B's wallet-owned row but omit its wallet id from the loaded wallet set, reconcile,
     and assert the durable selection is removed.
5. **`wallet-orphaned restored identity is cleared`**
   - omit B from the wallet-owned DAO-shaped input, reconcile, and assert removal.
6. **`reconciling one network never clears another`**
   - make Testnet stale while Mainnet remains valid and assert only Testnet is removed.
7. **`restoration waits for wallet load before reconciling`**
   - start the production coordinator with a controllable suspended wallet loader;
   - assert restoration is `Loading` and the stored id remains intact while the loaded
     wallet set is still empty;
   - complete the loader with the selected identity's wallet present;
   - assert reconciliation runs afterward, the id remains valid, and state becomes
     `Ready`.
8. **`post-bootstrap restoration failure is recoverable`**
   - fail reconciliation once and assert the network enters `Failed`;
   - retry with successful collaborators and assert it reaches `Ready` without requiring
     a new SDK Flow collector.

Deterministic Compose instrumentation test:

- **`reconstructed preference selects the visible identity`**
  - seed the second of two identities into a disk-backed DataStore;
  - tear down and reconstruct the DataStore/store;
  - render the production preference-to-active-identity state helper used by
    `DashPayTabScreen` with both identities eligible;
  - assert the second identity's label is displayed, proving the durable value reaches
    the visible picker through production selection wiring.

Targeted commands:

```bash
cd packages/kotlin-sdk
./gradlew :app:testDebugUnitTest --tests \
  'org.dashfoundation.example.ui.dashpay.DashPayActiveIdentityStoreTest'
./gradlew :app:connectedDebugAndroidTest \
  -Pandroid.testInstrumentationRunnerArguments.class=\
org.dashfoundation.example.DashPayActiveIdentityUITest
```

If no emulator/device is available, the instrumentation row must be reported as not run,
not as passing. Also run `./gradlew :app:assembleDebug` after the focused tests.

## 9. Definition of done

- The selected identity is durable per network across DataStore/process reconstruction.
- Bootstrap/network activation clears a restored selection only after wallet and identity
  eligibility are trustworthy.
- Valid restoration drives the same active identity and route inputs the user selected.
- Stale, wallet-orphaned, and unloaded-wallet selections are durably removed.
- Unit and Compose tests pin the restart regression and visible picker result.
- Targeted tests/build pass with no skipped verification hidden.
- The diff contains no invitation, Room schema, SDK/JNI, or unrelated cleanup changes.
