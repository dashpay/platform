# DashPay — Needs-Unlock / Verify-Failed UI Signal — Spec

Source backlog item: `SIGNER_SEED_ELIMINATION_SPEC.md` §4.7 / §9-7 (MEDIUM) —
"UI marker for contacts pending an unlock-drain"; `TODO.md` Q2 follow-up
("needs-unlock / verify-failed UI signal").

> **Status:** DONE (`9963923e05` Rust+FFI, `841802c587` Swift). REVIEWED (4-lens:
> feasibility / scope / failure-modes / domain-fit, 2026-06-23); must-fixes folded
> in (§9). The headline design changed materially from the first draft — the count
> tracks only **account-build** ops, and the Swift surface collapsed to one
> Equatable struct. On-device (devnet paloma, iPhone 17 Pro sim): the three banner
> states all verified — "N waiting" + Unlock → "Finishing…" → cleared — and the
> banner does **not** re-trip after a full sweep cadence (the M1 regression check).
> The `seedMismatch` red banner is covered by the `verify_seed_binds` unit test +
> the scoped-catch logic (a live wrong-seed import is destructive, so not staged).

## 1. Problem

A seedless wallet (`WalletType::ExternalSignable`) signs DashPay crypto per-op
through the Keychain-backed `MnemonicResolverCoreSigner`. The recurring sweep
has **no signer** — when it meets a contact whose payment account can't be built
because key material is unavailable (Keychain locked / not yet unlocked this
session), it **enqueues** a `PendingContactCrypto` op (§4.6) instead of failing,
and the contact stays visible but "needs unlock to finish setup." The queue is
**drained** when the signer becomes available (unlock, or any signer-present
action).

Two states reach the user inadequately today:

1. **Pending account-build backlog is invisible.** `PlatformWalletInfo.pending_contact_crypto`
   accumulates deferred `RegisterReceiving` / `RegisterExternal` ops (the ones
   that build a contact's payment account), but nothing surfaces "some contacts
   are waiting for an unlock to finish setup." The only production drain caller is
   the unlock path, and it logs its result `print()`-only inside a fire-and-forget
   `Task.detached` (`PlatformWalletManager.swift:531-556`).

2. **Seed-verification failure is only weakly surfaced.** `verify_seed_binds` (run
   at unlock) returns `SeedMismatch` when the resolver is mapped to the wrong
   seed. The restore loop already classifies it and sets `self.lastError`
   (`PlatformWalletManager.swift:419-429`) — but `lastError` is **global,
   transient, and not keyed by wallet**, so a banner can't latch a per-wallet
   "this wallet's seed doesn't match" state off it. (The first draft wrongly
   called this `print()`-only; corrected per review.)

### Goal

Surface both as observable, per-wallet status the UI can render:
- **A. needs-unlock**: a live count of *deferred account-build ops* (> 0 ⇒ a
  banner "N contact(s) waiting to finish setup" with an Unlock action).
- **B. verify-failed**: a per-wallet `seedMismatch` flag (a hard wrong-seed
  rejection — loud, actionable), distinct from a transient verify error.

### Non-goals

- No new on-chain artifact; no data-contract change.
- **No change to the enqueue / drain / classification logic itself** (§4.6/§4.7
  already landed and tested) — this is purely a read/observe + UI surface. (This
  is why the M1 fix counts a *subset* of ops rather than re-gating the enqueue.)
- Not the §6b upstream per-wallet restore (`LOAD_UNIMPLEMENTED`). We read the
  in-memory queue, which converges on its own (§2), so we don't depend on it.

## 2. Chosen approach — and why it diverges from "mirror `paymentChannelBroken`"

The TODO says "surface … through persistence the way `paymentChannelBroken`
already is." Research traced that pattern end-to-end and it is the **wrong fit**.
`paymentChannelBroken` is a **durable per-contact attribute** of an *established*
contact: Rust `EstablishedContact` field → `ContactChangeSet.established` → SQLite
`contacts.payment_channel_broken` column → `ContactRequestFFI` field on both rows
→ SwiftData `PersistentDashpayContactRequest` column → per-row UI icon. It is
permanent state that must survive restart.

Neither needs-unlock signal is durable:

- **The account-build count is live, self-converging operational state.** It lives
  in-memory on `PlatformWalletInfo.pending_contact_crypto` (`platform_wallet.rs:55`),
  populated by the signerless sweep's enqueue path. Crucially, **a cold restart
  restores no wallet from SQLite at all** — `SqlitePersister::load()` returns empty
  `wallets` (`persister.rs:909-946`, `LOAD_UNIMPLEMENTED = ["ClientStartState::wallets"]`);
  the wallet returns only by **re-import**, which re-syncs contact requests from
  scratch (cursors reset) and re-enqueues the same account-build ops
  (`contact_requests.rs:1115-1156`). So the count converges with zero special
  handling — we do **not** need (and cannot use, today) the persisted-queue
  restore. Persisting it the `paymentChannelBroken` way would duplicate
  self-converging state, depend on the blocked-upstream restore, and risk a
  stale-positive banner after the user already unlocked. **Wrong model.**

  (Side note: `platform_wallet.rs:54-55`'s doc comment claims the queue is
  "restored at load," which is doc-rot — `load.rs:104` restores `Vec::new()`.
  Fix the comment in passing.)

- **The verify outcome is per-session.** `verify_seed_binds` needs the signer, so
  it's only evaluable at unlock; it's re-evaluated each session and a persisted
  value goes stale the moment the user fixes the Keychain mapping.

The **correct, simpler** fit reuses two patterns already in the codebase:

- **Count (A)** → a pollable FFI getter over in-memory state, read by the existing
  ~1 Hz `startProgressPolling()` loop into an inequality-gated `@Published`
  property — like `isDashPaySyncing()` / `spvProgress`
  (`PlatformWalletManager.swift:967-997`). (One caveat: this getter is *per
  wallet*, so the poll is O(wallets)/tick, not the O(1) of `isDashPaySyncing()`.
  Cheap for 1–2 wallets; not literally identical.)
- **Verify (B)** → a per-wallet `@Published` flag set from the verify FFI result
  at the unlock call sites (which already exist).

This is a **scope reduction** vs the literal TODO note: no SQLite column, no
changeset field, no SwiftData migration, no per-row plumbing. One small Rust read
method + one FFI getter + Swift observable wiring + one banner.

## 3. Granularity & semantics

- **Wallet-scoped count (not per-contact icon).** §4.7 phrases it as a marker
  "for deferred contacts," but the remedy (a Keychain unlock) is **wallet-global**
  — `drain_pending_contact_crypto` drains the whole wallet's queue
  (`contact_requests.rs:1308`). A single actionable banner matches the user's next
  step better than per-contact icons, and the TODO asks for a *count*. Per-contact
  icons are deferred (YAGNI).
- **Count is a wallet-scoped upper bound.** The queue is keyed
  `(owner_identity_id, contact_id)`, so a wallet with multiple identities
  aggregates across them; and an op that will resolve to `Permanent`
  (channel-broken) on the next drain is included until drained. Banner copy must
  therefore be honest ("N contact(s) waiting to finish setup"), **not** promise
  "N will succeed on unlock."
- **Count excludes `ContactInfoDecrypt` (the M1 fix).** That op is re-enqueued
  *unconditionally every sweep* (no already-decrypted gate, `contact_info.rs:311`),
  so it is structurally always ≥1 and would make the banner re-trip ~15s after
  every unlock forever. Only `RegisterReceiving` / `RegisterExternal` converge to
  0 once their external account is built (`contact_requests.rs:1137-1144`); the
  count tracks **only those**. A contact whose contactInfo can't decrypt but whose
  payment account *is* built is not "needs unlock" for payment purposes (the
  account-build ops are what gate payability).

## 4. Interface / data flow

### 4.1 Rust — read method (rs-platform-wallet, `network/contact_requests.rs`, next to `drain_pending_contact_crypto`)

```rust
/// Count of deferred **account-build** contact-crypto ops queued for this
/// wallet (in-memory): the `RegisterReceiving` / `RegisterExternal` ops that
/// build a contact's payment account and need a signer unlock to complete.
///
/// `ContactInfoDecrypt` is intentionally excluded: it is re-enqueued every
/// signerless sweep (no already-decrypted gate), so it is structurally always
/// present and is not an actionable backlog. Account-build ops converge to 0
/// once drained (candidate selection skips contacts whose external account
/// already exists), so this count is a true "waiting for unlock" indicator.
pub async fn pending_contact_crypto_count(&self) -> usize {
    use crate::changeset::PendingContactCryptoOp;
    let wm = self.wallet_manager.read().await;
    wm.get_wallet_info(&self.wallet_id)
        .map(|info| {
            info.pending_contact_crypto
                .iter()
                .filter(|e| {
                    matches!(
                        e.op,
                        PendingContactCryptoOp::RegisterReceiving
                            | PendingContactCryptoOp::RegisterExternal { .. }
                    )
                })
                .count()
        })
        .unwrap_or(0)
}
```

Unit test (non-tautological — pins the M1 exclusion): enqueue a mix of
`RegisterReceiving`, `RegisterExternal`, and `ContactInfoDecrypt` into a test
`PlatformWalletInfo`; assert the count == the account-build ops only.

### 4.2 FFI — getter (rs-platform-wallet-ffi/src/dashpay.rs), mirroring the drain FFI minus the signer

```rust
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_pending_contact_crypto_count(
    wallet_handle: Handle,
    out_count: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(out_count);
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.pending_contact_crypto_count().await })
    });
    let count = unwrap_option_or_return!(option); // unknown handle -> NotFound
    unsafe { *out_count = count as u32; }
    PlatformWalletFFIResult::ok()
}
```

FFI tests: null `out_count` → `ErrorNullPointer`; unknown handle → `NotFound`.
(`blocking_read` is an alternative to `block_on_worker`, per
`platform_wallet_get_managed_identity:78`, but `block_on_worker` matches the drain
and keeps the read method `async`/unit-testable; keep it for consistency.)

### 4.3 Swift — one Equatable per-wallet status struct (PlatformWalletManager, `@MainActor`)

Collapse to a single snapshot per wallet (not parallel dicts):

```swift
public struct DashPayUnlockStatus: Equatable {
    public var pendingAccountBuilds: UInt32 = 0
    public var seedMismatch: Bool = false
    public var draining: Bool = false
}
@Published public private(set) var dashPayUnlockStatus: [Data: DashPayUnlockStatus] = [:]
```

**Count (A)** — in the poller, per wallet, inequality-gated *per key* (the struct
is `Equatable`):

```swift
for (walletId, _) in self.wallets {
    if let n = try? self.pendingAccountBuildCount(for: walletId) {
        var s = self.dashPayUnlockStatus[walletId] ?? .init()
        if s.pendingAccountBuilds != n { s.pendingAccountBuilds = n; self.dashPayUnlockStatus[walletId] = s }
    }
}
// prune ghost keys (M2): self.dashPayUnlockStatus.keys not in self.wallets -> removeValue
```

**Verify (B)** — set `seedMismatch` from the **verify FFI result code only**, at
both unlock sites (the M3 fix: do *not* wrap a broad catch around
`unlockWalletFromKeychain`, whose `walletId.count == 32` guard at :493 also throws
`.invalidParameter`). In `unlockWalletFromKeychain` after the verify `.check()`:
`.invalidParameter` ⇒ `seedMismatch = true`; success ⇒ `false`; other ⇒ leave
unchanged (transient). Mirror in the restore-loop catch (`:419-429`).

**Drain (C)** — the drain stays fire-and-forget, but stop the `print()`-only
swallow and add the in-flight guard (failure-modes S1/S3, cross-actor M1):

```swift
// before launching: self.dashPayUnlockStatus[walletId]?.draining = true  (on MainActor)
Task.detached(priority: .utility) {
    let result = ... drain ...
    await MainActor.run {
        self.dashPayUnlockStatus[walletId]?.draining = false
        if /* failed */ { self.lastError = error }   // no longer print-only; no new flag
    }
}
```

No separate `lastDrainFailed` dict: a stuck drain leaves `pendingAccountBuilds > 0`
(banner stays), and the rare cleared-but-failed edge surfaces via `lastError`.

**deleteWallet** — `dashPayUnlockStatus.removeValue(forKey: walletId)` (M2).

**Unlock entry point** — `unlockWalletFromKeychain(_:)` is already `public` and
takes a `ManagedPlatformWallet` (resolve via `walletManager.wallet(for: walletId)`,
as `ContactsView.swift:159` does). It is synchronous + throwing and kicks the
drain into a detached task, so the button calls `try? walletManager
.unlockWalletFromKeychain(wallet)` and lets the poller/`draining` reflect the
outcome (no spinner-until-drained).

### 4.4 UI — banner (SwiftExampleApp, `DashPayTabView`)

Host: `DashPayTabView.content`, between `dashPayBalanceRow` and the segmented
Picker (domain S2 — covers both Contacts and Requests; has `activeIdentity` +
`identity.wallet?.walletId` in scope). It is a **net-new small component** (no
reusable banner exists; only the per-row `paymentChannelBroken` icon — domain S3),
reusing the `exclamationmark.triangle.fill` + `.orange`/`.red` styling. Reads
`walletManager.dashPayUnlockStatus[walletId]`, priority:

1. `seedMismatch` → **red** "Seed verification failed — this wallet's Keychain
   seed doesn't match. DashPay signing is disabled." (precondition; intentionally
   subsumes the count — N1.)
2. else `draining` → **orange** "Finishing contact setup…" (no action; the
   in-flight guard that prevents a double-drain — S1/S3).
3. else `pendingAccountBuilds > 0` → **orange** "N contact(s) waiting to finish
   setup" + **Unlock** button.
4. else → nothing (`.unknown`/healthy renders no banner).

UI-only SwiftUI; verified on-device per the CLAUDE.md UI exception.

## 5. Failure modes (post-review)

- **Permanent false "needs unlock" (M1) — FIXED** by counting only account-build
  ops; `ContactInfoDecrypt` excluded. Without this the banner re-trips every ~15s.
- **Ghost banner after wipe (M2) — FIXED** by purging `dashPayUnlockStatus` in
  `deleteWallet` + pruning poller keys (walletIds are deterministic from the
  mnemonic, so a reused id would otherwise inherit a stale banner).
- **False wrong-seed banner (M3) — FIXED** by setting `seedMismatch` from the
  verify FFI result only, not a broad call-site catch.
- **Cross-actor `@Published` write (M1-conc) — FIXED** by `await MainActor.run`
  in the detached drain.
- **Double-drain / flicker (S1/S3) — MITIGATED** by the `draining` guard (button
  disabled, "Finishing…" shown) for the whole multi-contact drain window.
- **Count is an upper bound (S2 + multi-identity M3)**: includes poison/Permanent
  ops and sibling identities; copy says "waiting," not "will succeed."
- **Cold restart**: count reads 0 until re-import re-syncs and re-enqueues
  (convergent; the queue restore is blocked upstream and intentionally unused).
- **Watch-only wallet**: `unlockWalletFromKeychain` early-returns (`hasMnemonic`
  false), so `seedMismatch` stays false and no banner shows — correct.

## 6. Test / verification plan

- **Rust unit** (`pending_contact_crypto_count`): mixed-op queue → count equals
  the account-build ops only (pins the M1 exclusion; fails if `ContactInfoDecrypt`
  is counted).
- **FFI** (`dashpay.rs` tests): null `out_count` → `ErrorNullPointer`; unknown
  handle → `NotFound`; seeded queue → correct filtered count marshalled.
- **Swift build**: `build_ios.sh` (xcframework + app) green.
- **On-device** (UI exception): seedless wallet, locked → sweep discovers an
  inbound contact needing an external account → banner "1 contact waiting to
  finish setup"; tap Unlock → "Finishing…" → banner clears and does NOT re-trip
  after the next sweep (the M1 regression check). Wrong-seed import → red banner.
- **Acceptance**: the drain catch no longer `print()`-only; verify sets
  `seedMismatch` at both unlock sites.

## 7. Alternatives rejected

- **Full `paymentChannelBroken`-style persistence**: persists ephemeral,
  self-converging state; depends on the blocked-upstream restore (not buildable
  today); risks staleness. §2.
- **Count all queue entries** (`len()`): broken — `ContactInfoDecrypt` re-enqueues
  every sweep → permanent false positive. §3 / M1.
- **Re-gate the contactInfo enqueue to fix the count**: out of scope (changes the
  drain/enqueue logic, a non-goal); counting the right subset is the surgical fix.
- **4-state `SeedVerification` enum + `lastDrainFailed` dict + 3 parallel dicts**:
  over-modeled — 3 enum cases had no UI consumer, the drain flag duplicates the
  count, parallel dicts break the one-snapshot-per-wallet convention. Collapsed to
  one Equatable struct with `seedMismatch` + `draining`.
- **Per-contact needs-unlock icon**: deferred — the remedy is wallet-global. §3.
- **Surface verify via `lastError` only**: global/transient/unkeyed; can't latch a
  per-wallet banner.

## 8. Layer-by-layer change list

| Layer | File | Change |
|---|---|---|
| Rust read | `rs-platform-wallet/src/wallet/identity/network/contact_requests.rs` (next to `drain_pending_contact_crypto`) | `pending_contact_crypto_count(&self) -> usize` (account-build ops only) + unit test |
| Rust doc | `rs-platform-wallet/src/wallet/platform_wallet.rs:54-55` | fix the "restored at load" doc-rot |
| FFI | `rs-platform-wallet-ffi/src/dashpay.rs` | `platform_wallet_pending_contact_crypto_count` + tests |
| Swift wrap | `swift-sdk/.../PlatformWalletManager.swift` | `pendingAccountBuildCount(for:) throws -> UInt32` FFI wrapper |
| Swift observe | `swift-sdk/.../PlatformWalletManager.swift` | `DashPayUnlockStatus` struct + `@Published dashPayUnlockStatus`; poller line + prune; verify publish at 2 sites; drain `draining` guard + MainActor hop; `deleteWallet` purge |
| Swift UI | `SwiftExampleApp/.../DashPay/DashPayTabView.swift` (+ small banner view) | banner between balance row and Picker; Unlock action |

## 9. Review resolutions (4-lens, 2026-06-23)

- **Feasibility (M1, critical):** count account-build ops only; `ContactInfoDecrypt`
  re-enqueues every sweep. Corrected the self-heal rationale (re-import, not SQLite
  restore) and noted the `platform_wallet.rs:54-55` doc-rot + the O(wallets) poll.
- **Scope:** confirmed poller-over-persistence; fixed the overstated §1 verify
  premise; collapsed the 4-state enum to `seedMismatch`; dropped `lastDrainFailed`.
- **Failure-modes:** M1 cross-actor write, M2 ghost banner, M3 false-mismatch,
  S1/S3 drain races (→ `draining` guard), S2 upper-bound copy.
- **Domain-fit:** one Equatable struct (M1/M2), wallet-vs-identity scoping in copy
  (M3), `DashPayTabView` host + net-new banner (S2/S3), verify publish at both
  unlock sites (S4).
