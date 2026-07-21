# Persistence consumer behavior changes in PR #3999

Investigation target: `investigate/persistence-consumer-behavior-changes` at
`6efa83bb53`, based on the reviewed PR head. This report distinguishes current
source facts from impact inferences. The report was written against that
baseline; the follow-up resolution below records the consumer fixes subsequently
applied on this branch.

### Follow-up resolution

This follow-up branch implements the two consumer fixes recommended below:

- SwiftExampleApp now aborts `clearLocalState` before deleting SwiftData rows
  when no wallet manager is bound or the Rust shielded reset throws, and every
  clear attempt leaves the service rebind-required because Rust may already
  have quiesced or partially reset before returning an error.
- The invitation reclaim classifier now treats only typed FFI code 24 (the
  retained local tombstone) as definitive evidence of a prior local reclaim and
  restores `Reclaimed`; the consensus-message fallback remains conservative.

The remainder of this document records the behavior and verdicts at the
investigated baseline `6efa83bb53`.

## Executive verdict

> **Merge blockers found, but not in the asset-lock tombstone invariant.** Item
> 3's funds-adjacent invariant holds for every production caller found: Rust
> only marks an asset lock `Consumed` after a Platform result has confirmed that
> the consuming state transition landed. A failed or indeterminate submission
> can leave a false-negative non-consumed row, but cannot create a false-positive
> consumed tombstone.
>
> Item 3 nevertheless has a lower-severity consumer correctness blocker. Typed
> code 24 is emitted by a local retained tombstone and therefore proves an
> earlier local reclaim completed; Swift currently merges that evidence with a
> network already-consumed rejection and permanently labels the row `Claimed`.
> This is not funds-unsafe, but it is a deterministic regression from the
> carefully separated PR #4041 reclaim outcomes and should be fixed or receive
> explicit product acceptance before merge.
>
> Item 4 does expose a real in-tree inconsistency: the new Rust failure is
> correct, but SwiftExampleApp catches `clearShielded()` and continues deleting
> all SwiftData shielded rows. The commitment tree can therefore remain full
> while the host rows are wiped—the exact bug the fail-closed Rust change is
> intended to prevent. This branch should not merge until that consumer returns
> without wiping on a Rust clear failure (and handles the no-manager case with
> the same fail-closed rule).

| Item | Verdict | Short reason |
| --- | --- | --- |
| 1. Legacy create capabilities | **Acceptable intentional tradeoff; release-note/migration callout required** | It is a real C-ABI behavior change, but there is no in-tree production legacy caller and no documented external caller was found. |
| 2. Changeset begin | **Acceptable correctness change; release-note callback semantics** | Nonzero now aborts before writes. The current Swift begin callback always returns zero, so the alleged transient iOS regression is not present. |
| 3. Consumed asset-lock tombstones | **No funds-safety gap; merge-blocking Swift semantic regression** | The false-positive tombstone scenario is unreachable, but Swift conflates local typed proof of self-reclaim with a network rejection and writes the wrong terminal status. |
| 4. Shielded clear | **Merge blocker in the SwiftExampleApp consumer** | Rust and Kotlin fail closed; SwiftExampleApp logs the failure and wipes anyway. |

## 1. `platform_wallet_manager_create` and persistence capabilities

### Verified facts

The legacy constructor now unconditionally supplies `NONE`, irrespective of
the callback vtable. In
`packages/rs-platform-wallet-ffi/src/manager.rs:46-58`:

```rust
pub unsafe extern "C" fn platform_wallet_manager_create(/* ... */) {
    platform_wallet_manager_create_impl(
        sdk_ptr,
        persistence,
        event_handler,
        PersistenceCapabilities::NONE,
        out_handle,
    )
}
```

The behavior is deliberate and test-pinned: the
`legacy_create_is_abi_stable_and_fail_closed` test at
`packages/rs-platform-wallet-ffi/src/manager.rs:550-568` creates a manager with
a callback vtable and asserts `out.bits == 0`.

The replacement is the additive C entry point
`platform_wallet_manager_create_with_persistence_capabilities`, defined at
`packages/rs-platform-wallet-ffi/src/manager.rs:61-83`. It accepts a versioned
`PersistenceCapabilitiesFFI`; an unknown version maps to `NONE`
(`manager.rs:19-26`). Effective capabilities remain constrained by what is
structurally wired:

```rust
// packages/rs-platform-wallet-ffi/src/persistence.rs:853-856
self.declared_capabilities
    .intersection(self.callback_capabilities())
```

`callback_capabilities()` checks the required callback shapes at
`persistence.rs:795-841`, so declarations cannot manufacture a capability that
the vtable cannot support.

This is a source-verifiable change, not merely a review inference. Commit
`e63ca85e65ec8076e63600d783fbd5bb0b81dbe1` (`fix(sdk): address parity review
findings`) changed the legacy path from `FFIPersister::new(callbacks)`—whose
capabilities were structurally inferred—to the implementation above and added
the capability-aware constructor.

Both in-tree platform consumers migrated:

- Swift constructs its declaration and calls the new entry point at
  `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletManager.swift:306-336`.
  Its declared bits include atomic changesets, invitations, funding indices,
  and wallet restore (`PlatformWalletPersistenceHandler.swift:1189-1202`).
- JNI constructs `PersistenceCapabilitiesFFI` and calls the new entry point at
  `packages/rs-unified-sdk-jni/src/wallet_manager.rs:155-188`. Kotlin does not
  attest invitation persistence (`packages/kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/persistence/PlatformWalletPersistenceHandler.kt:121-129`),
  consistently with JNI leaving `on_persist_invitations_fn: None`
  (`packages/rs-unified-sdk-jni/src/persistence.rs:169-174`). Android therefore
  intentionally refuses invitation creation rather than claiming incomplete
  durability.

A repository-wide exact-symbol search found only the legacy definition and its
Rust unit test. The only production calls are the Swift and JNI calls to the new
entry point. This included `packages/kotlin-sdk`, `packages/swift-sdk`,
`packages/rs-unified-sdk-jni`, and the rest of the repository. The FFI crate is
published as a C-compatible static/dynamic library, so out-of-tree callers are
plausible, but no actual external caller, sample, or documentation using this
legacy constructor exists in this repository. The README explains C header and
library generation, but its example does not use either platform-wallet manager
constructor. Exact-symbol public web searches also returned no indexed caller.
That search cannot prove private or unindexed consumers do not exist. The
claimed affected external population is therefore **not verified**; it is an
API-risk inference.

### What “invitation creation” concretely gates

`PersistenceCapabilities::INVITATION_CREATION` is the union of atomic
changesets, invitation writes, asset-lock funding-index persistence, and wallet
restore (`packages/rs-platform-wallet/src/changeset/persistence_capabilities.rs:24-56`).
`create_invitation` checks the composite before funds move:

```rust
// packages/rs-platform-wallet/src/wallet/identity/network/invitation.rs:248-264
let capabilities = self.persister.persistence_capabilities();
let required = PersistenceCapabilities::INVITATION_CREATION;
if !capabilities.contains(required) {
    return Err(PlatformWalletError::Persistence(/* missing mask */));
}
```

The reason is security-sensitive but fail-safe: the bearer voucher private key
comes from a funding index. If the index cannot be durably restored, a restart
could export the same key again (`invitation.rs:209-214`). The builder also
persists and flushes that index before broadcast and aborts before broadcast on
failure (`packages/rs-platform-wallet/src/wallet/asset_lock/build.rs:652-678`).

Consequently, a legacy caller does not silently create a non-durable invitation
or lose funds. Manager creation succeeds, but the invitation operation returns
a persistence error before broadcast. Whether that appears as a hidden feature,
a thrown FFI error, or a visible message is host-dependent. The in-tree Swift
wrapper propagates the error, and SwiftExampleApp's invitation sheet catches it
and displays an error.

### Impact and verdict

This is a real compatibility change for any external C client that previously
relied on callback-shape inference. It is not an in-tree regression, and its
failure mode protects against voucher-key reuse before funds move.

**Verdict: acceptable intentional tradeoff, not a merge blocker. A release note
must name the new constructor and state that the legacy constructor now attests
zero capabilities even with a complete vtable.**

## 2. Nonzero changeset-begin callback

### Verified facts

`FFIPersister::store` now treats a nonzero begin result as fatal, closes its
internal round guard, and returns before any per-kind callback
(`packages/rs-platform-wallet-ffi/src/persistence.rs:858-905`):

```rust
if result != 0 {
    let _ = round.end_round();
    return Err(PersistenceError::backend(format!(
        "changeset-begin callback returned error code {result}; \
         round aborted before any write"
    )));
}
```

Commit `cac692737ca699997c4eaabc6aafc4a60d2b8cf1` (`fix(platform-wallet):
freeze sync watermark on persistence fault ... (#4069) (#4071)`) changed the
previous branch from only
`eprintln!("Changeset-begin callback returned error code {}", result)` to this
early error. Thus the review's before/after description is accurate. The
`nonzero_begin_aborts_the_round` test verifies the behavior and that a rejected
round does not wedge the next one.

The claimed iOS scenario is not supported by the current Swift implementation.
`beginChangeset` is non-throwing and only sets a flag on the serialized queue
(`packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletPersistenceHandler.swift:1253-1270`).
More decisively, the C shim always returns zero:

```swift
// PlatformWalletPersistenceHandler.swift:6128-6141
handler.beginChangeset(walletId: walletId)
return 0
```

There is no SwiftData save, disk access, or conflict resolution in this begin
callback. Disk-full, SQLite, and SwiftData save failures occur in
`endChangeset`, whose `backgroundContext.save()` catch rolls back and returns
false (`PlatformWalletPersistenceHandler.swift:1289-1318`); its C shim converts
that to nonzero (`:6148-6165`). Those end failures were already meant to reject
the round.

### End-user effect

For a custom C/iOS host that actually returns nonzero, the result is a
`PersistenceError`, not a Rust panic or process abort. No per-kind write runs.
Foreground operations receive an FFI error for their host UI to display or
retry. For the Core-event background bridge, a rejected store faults the wallet
watermark so it cannot advance beyond missing rows; the next scan can re-emit
the idempotent writes
(`packages/rs-platform-wallet/src/changeset/core_bridge.rs:240-250`). That is a
data-loss-avoidance failure, although logging/visibility and retry timing remain
flow-specific.

For the in-tree Swift host, a “transiently failing changeset begin” cannot occur
without future code changing the shim's unconditional return. Real transient
storage failures are end-callback failures, not evidence for this behavior
change.

### Impact and verdict

**Verdict: acceptable correctness change, not a merge blocker.** Document the
callback contract for external vtable implementers: nonzero means “transaction
did not open” and now aborts the round. The specific alleged current iOS
production impact is a non-issue.

## 3. Consumed asset-lock tombstone semantics

### Verified facts: retention, restore, and code 24

`AssetLockStatus::Consumed` is explicitly a retained terminal tombstone
(`packages/rs-platform-wallet/src/wallet/asset_lock/tracked.rs:1-6,48-57`). The
consume operation mutates the existing entry and queues a full upsert rather
than removing it:

```rust
// packages/rs-platform-wallet/src/wallet/asset_lock/sync/tracking.rs:145-166
Some(entry) if entry.status != AssetLockStatus::Consumed => {
    entry.status = AssetLockStatus::Consumed;
    entry.proof = None;
    cs.asset_locks.insert(*out_point, (&*entry).into());
}
// ...
self.queue_asset_lock_changeset(cs.clone());
```

Swift upserts `statusRaw` rather than deleting the row
(`PlatformWalletPersistenceHandler.swift:185-231`), loads all wallet rows
without a status filter (`:318-347`), and supplies them to Rust restore
(`:4566-4577`). FFI restore maps discriminant 4 back to `Consumed`
(`packages/rs-platform-wallet-ffi/src/persistence.rs:4346-4361,4483-4497`).
`PlatformWalletInfo::apply_changeset` also retains replayed consumed entries
(`packages/rs-platform-wallet/src/wallet/apply.rs:315-346`), with a restart
regression test at `apply.rs:683-744`.

An exact-outpoint retry checks that local status and returns
`PlatformWalletError::AssetLockAlreadyConsumed` before broadcasting
(`packages/rs-platform-wallet/src/wallet/asset_lock/orchestration.rs:431-440`;
the lower resume path repeats the guard at
`wallet/asset_lock/sync/recovery.rs:295-315`). FFI result code **24** is
`ErrorAssetLockAlreadyConsumed`; code 23 is the distinct
`ErrorAssetLockNotTracked`
(`packages/rs-platform-wallet-ffi/src/error.rs:164-170,323-330`). Code 24 is an
FFI wallet error code, not Platform consensus code 10504.

Commit `448d112209086e20b93c18119e08b43ef254c1cb` changed both runtime consume
from `remove()` to `get_mut()` and restore/apply from dropping `Consumed` to
retaining it. The review's semantic delta—typed 24 instead of not-tracked—is
therefore verified.

### Safety analysis: can a false consumed tombstone be created?

No production caller found invokes `consume_asset_lock` before confirmed
Platform success:

1. Identity registration waits through
   `put_to_platform_and_wait_for_response_with_signer` and only then consumes
   (`wallet/identity/network/registration.rs:226-265,343-361`).
2. Identity top-up similarly obtains the successful new balance before its
   cleanup consume (`registration.rs:459-496,534-546`).
3. Platform-address funding waits for proof-attested address information, then
   persists the reconciled balances. If that persistence fails it explicitly
   leaves the lock non-consumed; otherwise it consumes
   (`wallet/platform_addresses/fund_from_asset_lock.rs:207-265,310-378`).
4. Shielded funding returns success only after
   `broadcast_and_wait::<StateTransitionProofResult>` confirms proven
   execution (`wallet/shielded/fund_from_asset_lock.rs:653-715`), then performs
   the consume (`:429-455`).

A repository-wide search found no other production calls; the remaining call
is a recovery unit test. The invitation reclaim flow reaches the same identity
registration/top-up paths with explicit authorization, rather than a separate
pre-confirmation tombstone path.

Failure windows are one-sided:

- Failure, cancellation, or an indeterminate transport result before a proven
  success skips `consume_asset_lock`; it cannot create a tombstone.
- Platform may land the transition while the client observes an error or the
  process dies before cleanup. That leaves a non-consumed row (a false
  negative), and a later retry is rejected by Platform as already used.
- A persistence failure after the in-memory status changes can leave a
  same-process consumed marker that is absent after restart. The operation has
  already landed, so this also cannot falsely reject a genuinely reusable lock.
- A crash after Platform success and before local consume has the same
  false-negative shape. There is no reverse window in which the local consumed
  marker is written before Platform confirmation.

Therefore the requested invariant—“a retained/restored tombstone cannot exist
for a consume that did not land”—holds for the production state machine as
implemented. This conclusion assumes the SDK's successful
`broadcast_and_wait`/proof result is the authoritative landed result; that is
the contract the surrounding code uses. It does not assume a transport error
means the transition did not land.

### Swift reclaim and PR #4041 semantics

The local squash commit for PR #4041 was found:
`f74465227fd7ca352b66b6439a1b259ba30c9807` (`feat(platform-wallet): dip-13
dashpay invitations (#4041)`). Its conservative persisted `reclaimInFlight`
semantics are still visible in the current source. The marker is saved only
immediately before the irreversible operation; a failed save aborts before the
consume (`packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/DashPay/ReclaimInvitationSheet.swift:198-225`).

The semantic change flagged by the review is real. On a retry with a prior
marker, typed `assetLockAlreadyConsumed` is classified as
`.consumedAmbiguous` (`ReclaimInvitationSheet.swift:386-412,457-469`), and that
branch writes `statusRaw = 1` (`Claimed`) at `:286-301`. Before tombstone
retention, the same locally consumed lock was removed/dropped during restore,
so the retry produced “not tracked”; `.untrackedAfterOwnAttempt` left status
and marker untouched (`:302-310`).

This does not introduce the feared false-consumed case. Typed code 24 comes
from the local retained tombstone, which these invitation paths only create
after the local reclaim succeeded on Platform. Swift deliberately still calls
that outcome ambiguous because its marker does not cryptographically identify
which competing transition consumed the voucher. The fallback for a Platform
consensus “already completely used” error remains message-based and can also
represent an invitee claim; it is distinct from the local typed code 24.

The new `Claimed` label is not merely less precise for typed 24. Code 24 is
emitted only by the local wallet tombstone guards, and an invitation tombstone
can only be created through the explicitly authorized local reclaim path after
Platform success. A foreign invitee claim does not write this inviter-local
tombstone. Thus typed 24 proves a prior local reclaim completed, whereas the
generic consensus fallback remains ambiguous. Merging those evidence classes
can permanently show a successful self-reclaim as an invitee `Claimed` row
(after the transient explanatory message is gone).

### Impact and verdict

**Verdict: no funds-safety/tombstone-invariant blocker, but a real
merge-blocking Swift consumer semantic regression.** The critical
false-positive tombstone gap was not found, and tombstone retention itself is
correct. Before merge, the reclaim classifier should preserve the distinction
between local typed code 24 and the network/message fallback, or product owners
should explicitly accept the permanent `Claimed` mislabel as a deliberate
revision to PR #4041. Release notes should also mention the new stable code 24
and retained consumed history.

## 4. `shielded_clear` without a coordinator

### Verified facts

Rust now quiesces and returns `ShieldedStoreError` when no coordinator exists:

```rust
// packages/rs-platform-wallet/src/manager/mod.rs:373-389
self.shielded_sync_manager.quiesce().await;
match self.shielded_coordinator().await {
    Some(coord) => coord.clear().await,
    None => Err(PlatformWalletError::ShieldedStoreError(
        "... on-disk tree not reset".to_string(),
    )),
}
```

Commit `050625c81c3cc20cbbb853d6563469086fe7d8e8` changed the previous
`if let Some(coord) { coord.clear().await? } Ok(())` implementation. The FFI
maps the error to `ErrorWalletOperation`
(`packages/rs-platform-wallet-ffi/src/shielded_sync.rs:390-426`), and the Swift
SDK wrapper throws it before advancing its local generation
(`PlatformWalletManagerShieldedSync.swift:314-325`).

The new Rust behavior is correct. With no coordinator, there is no handle to
reset the on-disk commitment tree. Reporting success allows a host to delete
its note, activity, and watermark rows while the tree and its size remain,
causing a later cold resync to skip positions. A loud failure is the only
truthful result unless the API itself can independently locate and reset the
tree.

Kotlin honors the contract. It requires a manager, does not catch
`clearShieldedStorage()`, and only wipes Room after it returns successfully
(`packages/kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/services/ShieldedService.kt:429-485`).
KotlinExampleApp catches the propagated exception and presents an error
(`SyncStatusScreen.kt:372-380`).

SwiftExampleApp does not honor it. Its comments call the reset best-effort, its
catch only logs, and deletion continues:

```swift
// packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Core/Services/ShieldedService.swift:746-770
if let managerForStop {
    do { try managerForStop.clearShielded() }
    catch { SDKLogger.error(/* ... */) }
}
try modelContext.delete(model: PersistentShieldedNote.self)
// ... all other shielded rows ...
try modelContext.save()
```

If `walletManager` is nil, the same method skips the Rust reset entirely and
still wipes SwiftData. Thus the caller can reproduce the exact divergence even
though the Rust API now reports the missing coordinator correctly.

This is corroborated by a not-yet-merged local remote commit,
`f03e12ff092b075f3610d2cd5d94d934ceab933f` (`fix(swift-sdk/SwiftExampleApp):
fail-closed on clearShielded throw in clearLocalState`), which adds an error and
`return` in that catch. It is not an ancestor of this branch. This report does
not apply that implementation change.

### Impact and verdict

No legitimate caller can safely rely on old silent success: without a
coordinator, the tree was not cleared. Kotlin's new loud failure is intended
and preserves data. The Swift SDK wrapper also breaks loudly as intended, but
SwiftExampleApp suppresses it and performs the unsafe action anyway.

**Verdict: real merge blocker requiring a Swift consumer code change.** The
host must not delete SwiftData rows if the Rust clear throws, and absence of a
manager must not be treated as permission to wipe rows while the Rust tree may
remain. The Rust error behavior itself should be retained.

## Verification performed

The following targeted Rust tests passed on this worktree:

- `cargo test -p platform-wallet-ffi legacy_create_is_abi_stable_and_fail_closed`
- `cargo test -p platform-wallet-ffi nonzero_begin_aborts_the_round`
- `cargo test -p platform-wallet built_resume_rebroadcasts_original_and_typed_failures_do_not_broadcast`
- `cargo test -p platform-wallet apply_asset_locks_field_rename`

The follow-up Swift changes were also validated with:

- `xcodebuild ... build-for-testing` for an arm64 iOS simulator, which compiled
  the app, unit-test bundle, and UI-test bundle successfully.
- Focused XCTest execution on an iOS 26.5 simulator: all 4 shielded-clear tests
  and all 16 invitation-classifier tests passed. The new stale-binding assertion
  failed against the pre-fix implementation and passed after the fix.
- `xcrun swiftc -parse` for the changed Swift source and test files.
- `git diff --check`.

No local Kotlin build was run. Git history was inspected locally for the named
commits and PR #4041 squash commit. No external population of legacy C callers
was independently discoverable from this repository or exact-symbol public web
searches; private and unindexed callers remain unknowable.
