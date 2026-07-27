# Shielded sync timing — SwiftExampleApp spec (revised)

## Goal

When the user runs SwiftExampleApp against a devnet whose chain has a
pre-seeded shielded pool (N=1M notes via `dashpay/drive:3.1-shielded.2`),
**they need to see in the UI how long a shielded sync pass took, AND
have a live signal that an in-flight sync is making progress.**

Primary use case: confirming initial wallet sync against a 1M-note
devnet completes within expected wall-clock (~10 min on M-series), and
that the user can tell from the UI whether sync is alive vs hung during
the long initial pass.

## Why the goal expanded vs the first draft

Adversarial review surfaced that a post-hoc-only duration is theatre at
N=1M: the user stares at a "Syncing..." spinner for ten minutes with no
signal "alive vs hung" — exactly the failure mode this exercise is
meant to detect. A live elapsed display is therefore mandatory for the
primary use case, not optional.

## Scope (two surfaces)

### Surface 1 — timing display (the primary task)

The existing "ZK Shielded Sync Status" section in
`CoreContentView.swift` (≈ lines 391–510). One inline addition to that
section, two new `@Published` fields on `ShieldedService`, one console
log line per pass. No existing functionality changes.

### Surface 2 — TEMPORARY test-wallet import (to make the timing meaningful)

To validate against `dashpay/drive:3.1-shielded.2` and recover the
seeded 400 000 balance, the iOS app needs to sync wallet A whose
ZIP-32 seed is the **raw bytes `[0x73; 32]`** — not derivable from
any BIP-39 mnemonic.

This requires:

1. **New FFI** `platform_wallet_manager_bind_shielded_with_raw_seed`
   in `packages/rs-platform-wallet-ffi/src/shielded_sync.rs` — sibling
   to the existing `platform_wallet_manager_bind_shielded`, but
   accepts a `seed_bytes: *const u8` + `seed_len: usize` instead of
   a `MnemonicResolverHandle`. Calls
   `wallet.bind_shielded(raw_seed.as_slice(), ...)` directly.
2. **Swift wrapper** `bindShieldedRawSeed(walletId:rawSeed:accounts:)`
   in `Sources/SwiftDashSDK/PlatformWallet/...`
3. **Debug-only UI button** "Bind Test Wallet A (Shielded)" in the
   existing ZK Shielded Sync Status section. Hardcodes `[0x73; 32]`
   and calls the new wrapper.

**Everything in Surface 2 is tagged with explicit removal TODOs.**
Example tag:
```
// TODO(shielded-snapshot-devnet-test): remove this FFI entry once
// SwiftExampleApp adopts a proper test-wallet import flow.
// Tracked: dashpay/platform#3714.
```

Surface 2 lives behind no `#[cfg]` gate (would complicate the
release build), so the TODO comments are the contract: this surface
is provisional and removed before merging the ultimate version of
PR #3732.

## Non-goals

- Not building a separate sync dashboard.
- Not orchestrating sync from Swift — `walletManager.startShieldedSync`
  already runs the loop; we observe.
- Not exposing per-cmx mid-pass progress (e.g. "412k/1M scanned"). That
  needs a Rust→Swift signal we don't have today. Out of scope.
- Not measuring "whole catch-up cycle" duration as a single value (see
  §"Known limitation: catch-up vs per-pass" below).
- Not persisting timing across app launches — display state only.
- Not building a `timeSinceBind` / "cumulative since bind" number — it
  grows unbounded and answers no concrete question.

## Existing surface (recap)

**Service:** `ShieldedService` (`Core/Services/ShieldedService.swift`)

Existing `@Published`:
- `isSyncing: Bool`
- `lastSyncTime: Date?` (when the most recent sync **completed**)
- `syncCountSinceLaunch: Int`
- `totalScanned: UInt64`, `totalNewNotes: UInt64`, `totalNewlySpent: UInt64`
- counters, balance, address fields

**UI:** `CoreContentView.swift` "ZK Shielded Sync Status" section
already shows `ProgressView()` + "Syncing..." while in-flight,
"Last sync: <relative>" via `lastSyncTime`, cumulative counters,
balance + Notes Synced watermark, Sync Now / Clear buttons.

**Underlying flow (Rust → Swift):**
1. `walletManager.$shieldedSyncIsSyncing` publishes `Bool`. Flips true
   at sync-pass start, false at completion.
2. `walletManager.$lastShieldedSyncEvent` publishes a
   `ShieldedSyncEvent` on each pass completion.

**Gap:** UI shows COUNTERS and relative completion time, but no
wall-clock duration of a single pass, and no live indication during a
10-minute initial sync.

## Spec

### S1. Service-level fields

Two new `@Published` fields on `ShieldedService` (read by the UI):

| Field | Type | Semantic |
|---|---|---|
| `lastSyncDuration: TimeInterval?` | seconds | wall-clock of the most recent non-cooldown sync pass (set at completion) |
| `currentSyncElapsed: TimeInterval?` | seconds | running wall-clock of the in-flight sync; ticks while `isSyncing == true`, nil otherwise |

One new private field:

| Field | Set when |
|---|---|
| `currentSyncStartedAt: Date?` | `isSyncing` Swift mirror transitions false → true |

Both `@Published` fields stay nil until they have a real value to
show. Both reset to nil on `bind()` / `reset()` / `clearLocalState`.

**No `lastBindCompletedAt`, no `lastSyncCompletedAt`, no `timeSinceBind`** —
dropped per Scope reviewer.

### S2. Pass boundaries — Swift edges only

Both pass endpoints are observed from the **Swift mirror of
`$shieldedSyncIsSyncing`**, not from Rust event timestamps.

- **Start:** false → true transition of `isSyncing`.
- **End:** true → false transition of `isSyncing`.

Rationale: `event.syncUnixSeconds` is integer-second resolution; mixing
it with `Date()` on the Swift edge can render negative or grossly
inflated durations for sub-second steady-state passes. Using the Swift
edge for both endpoints means the Rust↔Swift latency cancels out
symmetrically.

### S3. Live ticker

A 1-second `Timer` lives on the `ShieldedService` and:

- Starts on the false → true transition.
- Tick handler: updates `currentSyncElapsed = Date() − currentSyncStartedAt`.
- Stops + nils `currentSyncElapsed` on the true → false transition.

One timer source on the service rather than a per-view source — the
view subscribes to `$currentSyncElapsed` like any other published
field. Service is `@MainActor` so timer fires on main thread already.

### S4. Edge handling (bug-fixes from review)

- **B1 (clock skew):** Solved by S2 — Swift-edge for both endpoints.
- **B2 (failure leaves start stamped):** `currentSyncStartedAt` is
  cleared on EVERY true → false transition, regardless of whether the
  emitted `ShieldedSyncEvent` reports success or failure. Same for the
  ticker.
- **B3 (`switchTo(walletId:)` silently resets timing):** Documented in
  `bind()`'s comment block. UI behaviour after a wallet switch is the
  same as a fresh bind — the row reappears after the next pass.
- **B4 (negative or absurd duration):** Clamp to `max(0, …)` in the
  view formatter; if `currentSyncStartedAt` is nil at completion (which
  shouldn't happen with S2 but is defensible defence-in-depth), set
  `lastSyncDuration = nil` and skip the log line.
- **B5 (re-fire of `true`):** Set `currentSyncStartedAt` only when
  transitioning **from** `false`. Track the previous mirror value
  inside the `.sink` to detect the edge — the existing
  `syncStateCancellable` `.sink` is the right place.
- **Cooldown skip:** `result.cooldownSkip == true` events do NOT update
  `lastSyncDuration` (zeros are not signal). The ticker stops on the
  edge regardless. No noisy log spam: emit the cooldown-skip log only
  on the first one in a row; suppress contiguous skips.

### S5. Console log

In `handleShieldedSyncEvent`, when `result.success && !result.cooldownSkip`,
emit one line via `SDKLogger.log(.medium)`:

```
Shielded sync done  pass=<N>  elapsed=<X.XXs>  rate=<Y/s>  scanned=<n>  new=<n>  spent=<n>  balance=<credits>
```

- `.medium` (not `.high`) so devnet operators on default presets see
  these without changing log settings.
- `rate` is suppressed when `elapsed ≈ 0` or `scanned == 0`.
- Format kept stable for `xcrun simctl spawn log` scraping.

Skipped (cooldown) passes log a single `"Shielded sync skipped (cooldown)"`
at `.medium`; contiguous skips suppressed.

Started passes log `"Shielded sync started"` at `.medium` on the
false → true edge. Paired with the "done" line for offline analysis.

### S6. UI

Inside the existing "ZK Shielded Sync Status" section, immediately
under the existing "Queries Since Launch" row (and above the badges):

**While `isSyncing` is true AND `currentSyncElapsed != nil`:**
```
Syncing… elapsed: 4.2 s
```

**While `isSyncing` is false AND `lastSyncDuration != nil`:**
```
Last sync duration: 12.4 s
```

**Otherwise:** no row (clean state pre-first-sync, matches existing
"Not synced yet" behaviour).

Layout: same `HStack { Text(label) ; Spacer() ; Text(value).monospacedDigit() }`
pattern as the surrounding rows. Mono digits keep the number readable
as it ticks.

The existing `ProgressView()` + "Syncing..." spinner stays unchanged —
it's the qualitative "is something happening" affordance; the new row
is the quantitative "how long has it been going" affordance.

### S7. Reset sites

All three private/published fields nil'd in:
- `bind()` — before the new bind runs (so post-bind sync gets a clean
  baseline, not stale from a prior wallet).
- `reset()` — full teardown.
- `clearLocalState` — global clear.
- True → false edge — both `currentSyncStartedAt` and
  `currentSyncElapsed` nil'd (S4).

### S8. What this does NOT change

- No new FFI signatures.
- No changes to `rs-platform-wallet-ffi` or `rs-platform-wallet`.
- No changes to `PlatformWalletManager`.
- No SwiftData schema change.
- No new view / screen / sheet / menu entry.

## Known limitation: catch-up vs per-pass

At N=1M, the manager loop runs MANY internal passes during initial
catch-up — each is one `ShieldedSyncEvent`. We measure **per-pass**
wall-clock. The user-facing reality is that for an initial catch-up
the "Last sync duration" they see at the end will be the **last
pass**'s time (likely a few seconds — the trailing partial chunk),
NOT the whole catch-up's 10 minutes.

This is acceptable for the live use case (the live ticker covers the
"is it alive" question), but means the post-hoc number reads smaller
than the user's perceived wall-clock for the first catch-up. The
**console log** mitigates by recording every pass — sum from the log
to get total catch-up time.

A proper "catch-up completed" signal would require either:
- A Rust-side signal `next_start_index == tree_size` exposed as a
  derived `isCaughtUp` event, OR
- Synthesizing it Swift-side from `event.totalScanned` + a tree-size
  read.

Both expand the scope materially. Deferred. The current per-pass
number + live ticker covers the primary use case ("is it alive, how
long is each pass taking").

## Architecture conformance (`swift-sdk/CLAUDE.md`)

- ✅ Persist / load / bridge only. Timing fields are display state
  derived from existing Combine publishers — no business logic, no
  decisions, no orchestration.
- ✅ No new FFI surface.
- ✅ Timer is a UI-driving mechanism, not a policy loop. The decision
  to keep syncing lives on the Rust side; the timer just animates
  `currentSyncElapsed` for the view.

## Test plan

1. **Smoke (local dashmate devnet):**
   - Bind a wallet; observe one sync pass.
   - UI shows "Syncing… elapsed: X.Xs" with X ticking visibly.
   - On completion: UI shows "Last sync duration: Y.Ys" with Y a
     reasonable positive value.
   - Console: paired "started" + "done" log lines at `.medium`.

2. **Reset / clear behaviour:**
   - After a successful sync, hit "Clear". UI row disappears.
   - Hit "Sync Now". Row reappears live, then settles to post-hoc.

3. **Cooldown skip:**
   - After steady-state, force a cooldown-skip pass. UI does not
     update `lastSyncDuration` (stays at the prior value). Console
     shows one "skipped (cooldown)" line and no further skips until
     the next real pass.

4. **Failure path (offline):**
   - Disconnect the gateway, hit Sync Now. Confirm `lastSyncDuration`
     is NOT updated by the failed pass. `currentSyncStartedAt` IS
     cleared (verified indirectly: next successful pass shows correct
     duration, not absurdly inflated).

5. **N=1M devnet (validation):** point the app at
   `dashpay/drive:3.1-shielded.2` running on devnet. Bind a fresh
   wallet:
   - Live ticker increments visibly through the long initial pass.
   - At the end, console log lines sum to the user-perceived wall-clock.
   - Each subsequent pass shows a small (seconds) duration.

## File touch list

### Surface 1 (timing, permanent)

- `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Core/Services/ShieldedService.swift`
  - +1 private field (`currentSyncStartedAt`), +2 `@Published`
    fields (`lastSyncDuration`, `currentSyncElapsed`).
  - +1 Timer (`syncTickTimer: Timer?`).
  - +1 previous-mirror state on the existing `syncStateCancellable.sink`
    to detect false → true edges.
  - Reset sites (`bind`, `reset`, `clearLocalState`) get the new
    fields nil'd.
  - `handleShieldedSyncEvent` logs at `.medium` on success path;
    cooldown-skip path emits one suppressed log + leaves duration
    alone.
- `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Core/Views/CoreContentView.swift`
  - +1 inline row in the existing "ZK Shielded Sync Status" section
    (≈ line 439 area, alongside "Queries Since Launch").

### Surface 2 (raw-seed test wallet bind, TEMPORARY)

All marked with the same removal TODO tag.

- `packages/rs-platform-wallet-ffi/src/shielded_sync.rs`
  - +1 new FFI entry
    `platform_wallet_manager_bind_shielded_with_raw_seed`
    alongside the existing `platform_wallet_manager_bind_shielded`.
    Same parameters EXCEPT replaces `mnemonic_resolver_handle` with
    `seed_bytes: *const u8 + seed_len: usize`.
- `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletManagerShieldedBind.swift` (or wherever `bindShielded` lives)
  - +1 thin wrapper method
    `bindShieldedRawSeed(walletId:rawSeed:accounts:)` that calls
    the new FFI.
- `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Core/Views/CoreContentView.swift`
  - +1 button "Bind Test Wallet A (Shielded)" inside the ZK
    Shielded Sync Status section. Hardcodes `[0x73; 32]` and calls
    the new wrapper. Button is visible only when the active wallet
    has no shielded binding (matches the existing "Sync Now"
    affordance's gating).

### Diff size estimate

- Surface 1 (timing): ~60 lines additive.
- Surface 2 (raw-seed test wallet): ~70 lines additive.
- Total: ~130 lines, no removals, no API drift to existing
  functionality.
