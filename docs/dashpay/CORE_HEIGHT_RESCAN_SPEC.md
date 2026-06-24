# DashPay coreHeight block-rescan — implementation spec

> **Problem.** DIP-15 §8.7 + §12.6 require that a wallet not miss an incoming
> DashPay payment that landed on a contact's receival address *before* that address
> was being watched. `$coreHeightCreatedAt` is captured and persisted on every
> `ContactRequest` (`types/dashpay/contact_request.rs:38`) but is **never used** to
> drive an L1 re-scan. New contact receival accounts are watched **forward-only**
> from the current sync pointer.
>
> **Source.** `DIP_CONFORMANCE_GAPS.md` §1.1 (the code-verified 2026-06-24 re-audit).
>
> **Status.** REVIEWED (4-lens panel 2026-06-24: SPV-feasibility, regression-safety,
> correctness/failure-modes, scope/simplicity). Must-fixes folded in below; the
> review reshaped both the approach and the task split — see §8 for what changed and
> why. **Ready to implement.**

---

## 0. What the review changed (read this first)

The first draft hung the rescan off *"a new account was registered in the G1b
sweep."* The panel proved that is **structurally dead on the headline case** and
**over-scoped** in two ways. The corrected design:

1. **The genuine rescan case is narrow.** Of the three symptoms originally claimed,
   only **running-wallet offline-accept→pay** needs the rescan trigger.
   **Restore-from-seed / second-device** is a *separate, simpler* bug — a birth-height
   default — fixed by passing `Some(0)` on re-import (§3.1), not by a rescan.
2. **The trigger must not key off fresh registration.** The G1b sweep only *enqueues*
   deferred crypto (it has no signer, `contact_requests.rs:1457`); registration runs
   in the async `drain_pending_contact_crypto` (`:1630,1820`) — and on relaunch the
   account restores verbatim so registration early-exits and nothing rewinds. The
   rescan is therefore a **reconciliation over established contacts** (the gap between
   a contact's funding height and `synced_height`), read off `EstablishedContact`
   (§3.2) — not an event on registration.
3. **No upstream change.** The forward-only guard is only on the `WalletManager`
   *wrapper*; the inner `ManagedWalletInfo::update_synced_height` is unconditional
   (`key-wallet/.../wallet_info_interface.rs:392`) and reachable via the already-public
   `WalletManager::get_wallet_info_mut` (`accessors.rs:24`, called ~15× in
   platform-wallet today). **The original upstream task T1 is deleted.**
4. **The regression is safe** (full consumer audit, §3.4) — reuse `synced_height`; do
   **not** add a separate rescan pointer.

---

## 1. Research — how block-rescan already works in this stack

From the dash-spv audit (rev `b4779fc`):

**dash-spv already has a targeted backfill-rescan engine; it is just never triggered
for DashPay.** The compact-filter (BIP157/158) manager, each coordinator tick:
- `committed = progress.committed_height()`;
- `behind = wallet.wallets_behind(committed)` → wallet ids where `synced_height <
  committed` (strict `<`, `process_block.rs:250-261`);
- `stale_min = min(synced_height over behind)`;
- if any: `reset_for_rescan()` (drops in-flight batches/pipeline — but **keeps stored
  filters**), `update_committed_height(stale_min)`, `start_download()`
  (`sync/filters/sync_manager.rs:213-236`, `manager.rs:129-139`).

Pinned by `test_tick_rescans_from_wallet_synced_height_not_genesis`. So the trigger we
need is *"lower the wallet's `synced_height` to the contact's funding height"*; the
engine does the rest.

Three facts the review corrected vs. the first draft:
- **Stored filters are reused from disk, not re-downloaded.** `reset_for_rescan` does
  **not** clear `filter_storage`; `start_download` loads stored filters from
  `stored_filters_tip` and only fetches the never-stored tail
  (`manager.rs:179,214,248-261`). Re-scan of an already-synced range is disk-cheap.
- **Two extra floors.** `scan_start = max(wallet_birth_height, committed+1,
  header_start_height)` (`manager.rs:185-192`). A rewind **cannot** go below the
  seeded header checkpoint *or* the wallet birth height — the engine clamps silently
  (no warning today).
- **Filter-unavailable = silent infinite retry.** For a *never-stored* range that no
  peer will serve, `enqueue_retry` has **no retry cap** (`download_coordinator.rs:120-125`)
  → sync stalls on that range, retrying every 30s, never erroring. Only bites a
  from-checkpoint wallet rewound into an un-stored range.

Other facts in place:
- `$coreHeightCreatedAt` is on `ContactRequest` (`types/dashpay/contact_request.rs:38`);
  **both** directions live on `EstablishedContact` (`established_contact.rs:18-22`).
- Incoming contact payments are matched **only** against `dashpay_receival_accounts`
  (`contacts.rs:319`, `payments.rs:226`); `DashpayExternalAccount` is watch-only
  outbound (`contacts.rs:483`) and never scanned for incoming.
- The receival account is built in the **async drain** (`contact_requests.rs:1630`),
  not the sweep; the sweep only enqueues (`:1486`).
- On relaunch, accounts restore verbatim (`persistence.rs:3008-3013`, `load.rs:97-98`)
  and `synced_height` restores to its persisted **high** value when `> 0`
  (`persistence.rs:2897-2900`) — so registration early-exits and `wallets_behind`
  skips the wallet.

### Alternatives considered (and rejected)

| Approach | Why rejected |
|---|---|
| `clear_storage()` + full resync | Sledgehammer — re-downloads all headers+filters per new contact. |
| New dash-spv per-range block-rescan API | Duplicates the `synced_height`-keyed engine that already exists. |
| Trigger off fresh account registration | **Structurally dead on restore** (early-exit) and misses the seedless drain path — the original draft's flaw. |
| Separate DashPay-scoped rescan pointer | Redundant — `synced_height` regression is safe (§3.4); re-implements `wallets_behind` for no gain. |
| Lower `synced_height` via reconciliation (chosen) | Reuses the engine; fires on both restore and offline-accept→pay; no upstream change. |

---

## 2. The three exposure flavors (precise scope)

| Flavor | What happens today | Genuinely uncovered? | Fix |
|---|---|---|---|
| **Relaunch-restore** (SwiftData persister) | accounts restore verbatim; `synced_height` restores **high**; registration early-exits → no rewind | **Yes** — a payment to a receival address at `h < synced_height` is never re-matched | §3.2 reconcile |
| **Second-device / re-import from seed** | `create_wallet_from_seed_with_birth_height(None)` defaults birth to **current tip** → history skipped | **Yes** — but the right fix is the birth-height default, not a rescan | §3.1 `Some(0)` |
| **Running wallet, offline-accept→pay** | already at tip; contact establishes; payment landed at `h < tip`; nothing lowers `synced_height` | **Yes** — the core rescan case | §3.2 reconcile |
| Cold launch, normal forward sync | scans forward from `synced_height` | No | — |

(`synced_height == 0` restores fall back to `birth_height − 1`, `persistence.rs:2897-2900`
+ `wallet_restore_types.rs:536-539` — so the bug is **intermittent**; a test must avoid
that masking path — see §5.)

---

## 3. Chosen approach

Two independent fixes for two independent flavors, plus the safety rationale.

### 3.1 Restore birth-height default (`Some(0)` on re-import)

Second-device / seed re-import calls the no-birth-height FFI variant
(`PlatformWalletManager.swift:317` → `manager.rs:245`, passing `None`), which defaults
`birth_height` to the current SPV tip (`wallet_lifecycle.rs:156-166`) and **skips
history**. The mechanism to fix it already exists: `Some(0)` "always requests a full
historical scan from genesis … required when an address may have received funds before
the wallet was first registered" (`wallet_lifecycle.rs:63-72`). **Fix:** the
DashPay-capable re-import path passes `Some(0)` (or a known earliest funding height).
This is **durable** (birth height is persisted) and is a one-liner-class change,
**separate** from the rescan trigger.

### 3.2 Rescan reconciliation off `EstablishedContact` (not fresh registration)

Add a **reconcile step** (a local-only pass, sibling to `reconcile_incoming_payments`,
runnable from both wallet-load and the recurring `dashpay_sync`) that:

1. For every established contact with a **receival** account, read the pair's funding
   floor `f = min(outgoing.core_height_created_at, incoming.core_height_created_at)`
   off `EstablishedContact` (both are present — sidesteps the unreachable-height
   problem; the receival channel is payable only once both requests exist, so the min
   is the conservative-correct floor).
2. `floor = min(f) − 1` over all such contacts **whose `f < current synced_height`**
   (contacts funded at/above our tip need no backfill).
3. **Coalesce:** only act if `floor < current committed scan height` (not merely
   `< synced_height`) and no rewind is already in flight — a monotonically-decreasing
   drip-feed of floors must not thrash the shared filter re-download (§4.2).
4. Clamp `floor` to the engine's floors (header checkpoint, birth height); if the
   requested floor is below them, **log a warning** (the engine clamps silently today).
5. `assert!(floor < synced_height.saturating_sub(TIP_GUARD))` — never rewind into the
   last ~10 blocks (DIP-15 §12.6); the `f < synced_height` bound already implies this.
6. Lower `synced_height` to `floor` via a new `SpvRuntime` helper (§3.3). The next
   `FiltersManager` tick self-triggers `reset_for_rescan` + backfill.

Receival-only (external accounts are outbound, never scanned for incoming — including
them only deepens the rewind for zero benefit). `min(core_height) − 1` is the floor —
**no `REWIND_SLACK`**: the confirmed path uses BIP157 exact-script re-matching, which
has no bloom false-positive ambiguity to preserve (the §12.6 "slightly beyond" applies
to BIP37 bloom).

### 3.3 The rewind primitive — platform-wallet only, no upstream change

```rust
// SpvRuntime — holds Arc<RwLock<WalletManager<PlatformWalletInfo>>>
pub async fn rewind_synced_height(&self, wallet_id: &WalletId, floor: u32) {
    let mut wm = self.wallet_manager.write().await;
    if let Some(info) = wm.get_wallet_info_mut(wallet_id) {     // already public, accessors.rs:24
        if floor < info.synced_height() {
            info.update_synced_height(floor);                  // inner setter is unconditional, wallet_info_interface.rs:392
        }
    }
}
```

The forward-only guard is only on `WalletManager::update_wallet_synced_height`
(`process_block.rs:267-269`); the inner setter accepts a lower value. **No
`key-wallet-manager` / rust-dashcore change.** Caller drops the wallet-manager write
guard before/after per the existing non-reentrant-`RwLock` discipline. Note the
unguarded setter does **not** emit `WalletEvent::SyncHeightAdvanced` — correct, since a
rewind is not an advance; confirm in §3.4 that no consumer needs a change-event.

### 3.4 `synced_height` regression safety — SAFE (gate resolved)

Full consumer audit (regression-safety review). `synced_height` (filter-scan
checkpoint, what we lower) is **decoupled** from `last_processed_height`
(block-application high-water, monotonic, untouched). Every persisted consumer is
**monotonic-max guarded**, so a transient regression cannot corrupt, double-count, or
persist a lie:

| Consumer | Verdict |
|---|---|
| dash-spv rescan engine (`wallets_behind`→`reset_for_rescan`) | SAFE — this is what the rewind drives |
| `last_processed_height` re-advance on block re-delivery (`process_block.rs:391-394`, doc'd) | SAFE — surfaces UTXO changes without dragging height back |
| block re-processing dedup `is_new_transaction` (txid-keyed) | SAFE — re-delivered tx is an update, not insert |
| changeset merge (`changeset.rs:191-192`) + SQLite `upsert_sync_state` (`core_state.rs:199-233`) | SAFE — monotonic-max; persisted cursor never regresses |
| FFI `synced_height` signal (`SyncHeightAdvanced`-gated) | SAFE — no event on regression; UI display never goes backward |
| DashPay contact-request high-water cursor (`contact_requests.rs:770`) | SAFE — **millisecond `$createdAt`** namespace, independent of L1 height; not rewound, no duplicate ingest |
| identity/dashpay/address sync managers | SAFE — time-cadence + own Platform cursors; no L1-height gating |
| payment recording (`payments.rs:68,239,346`) | SAFE — txid-keyed idempotent; reconcile re-derives from outpoints |

**Invariant to preserve:** `synced_height` is the min-across-wallets filter checkpoint
and may regress on a deliberate rescan; `last_processed_height` is max-across-wallets
and must stay monotonic. The rewind touches only the former.

**Accepted limitation (durability).** The regression is **non-durable**: SQLite
sync-state is monotonic-max and reloads at the high-water on restart
(`persistence.rs:2898-2899`). If the app is **killed mid-backfill**, the rewind intent
is lost and the backfill silently does not resume. This is acceptable because: (a) the
*restore* flavor uses the **durable** birth-height path (§3.1), not the transient
rewind; (b) the offline-accept→pay backfill (§3.2) is a small near-tip range, so the
crash window is brief. If at-least-once-across-restart is later required, a single
persisted "pending rescan floor" breadcrumb (not a separate sync pointer) is the
minimal addition — deferred (§7 optional D).

---

## 4. Failure modes

1. **Filter-unavailable never-stored range → silent infinite 30s retry, no error**
   (`download_coordinator.rs:120-125`). Bites only a from-checkpoint wallet rewound
   into an un-stored range. Clamp to the header floor (§3.2.4) keeps us in
   stored-or-servable territory; log if clamped.
2. **Rewind-storm / shared filter re-download.** Match-attribution is per-wallet, but
   the filter re-download is **global** to the manager (one wallet's deep rewind
   re-traverses the whole range). A monotonically-decreasing floor sequence thrashes →
   the §3.2.3 coalescing guard (act only on `floor < committed`, skip if in-flight) is
   **required**, not optional.
3. **Idempotent re-delivery.** A block re-matched by the backfill cannot double-record:
   `record_incoming_dashpay_payments` / `reconcile_incoming_payments` /
   `confirm_sent_payment_by_txid` all gate on txid / outpoint
   (`payments.rs:68,239,346`).
4. **Reconcile cost.** The reconcile pass iterates established contacts each run; it is
   O(contacts) and only *acts* when a floor is below committed — cheap in steady state.
5. **Multi-wallet.** Per-wallet `synced_height`; lowering one wallet does not regress
   another's height (`process_block.rs:250-265`). Shared cost is the filter re-traversal
   only (#2).

---

## 5. Verification plan

- **§3.1 restore default:** a re-import with a contact funded before the old tip
  surfaces the historical payment (red with `None`/tip-default, green with `Some(0)`).
  Test must **not** persist `synced_height == 0` (that masks the bug via the
  birth-fallback, `persistence.rs:2897-2900`).
- **§3.2 reconcile (unit):**
  - `rescan_floor_is_min_over_established_receival_pairs` (min of both directions).
  - `reconcile_skips_contacts_funded_at_or_above_synced_height`.
  - `reconcile_coalesces_and_does_not_rewind_when_already_below_committed`.
  - `rewind_clamped_to_header_floor_logs_warning`.
  - `second_reconcile_pass_is_a_noop` (idempotency — no thrash).
- **§3.3 primitive (unit):** `rewind_synced_height` lowers; ignores a floor ≥ current.
- **Integration (rides #3549 `dp_*` e2e + devnet funding):** offline-accept→pay — pay a
  contact's receival address at `h`, establish on an already-synced wallet, assert the
  payment surfaces after the reconcile rewind (red before, green after).
- **On-device (manual, when funded):** restore-from-seed with a pre-funded DashPay
  contact → historical incoming payment appears after sync.

---

## 6. Resolved decisions (were open questions)

| Q | Decision |
|---|---|
| External-account inclusion | **Receival-only** — external is watch-only outbound, never scanned for incoming. |
| Height source | **`min(outgoing, incoming)`** off `EstablishedContact`, not "our own". |
| `REWIND_SLACK` | **Dropped** — BIP157 exact-match has no bloom ambiguity; floor = `min − 1`. |
| Floor below checkpoint | **Clamp + warn** (engine clamps silently; we add the log). |
| Reuse `synced_height` vs separate pointer | **Reuse** — regression is safe (§3.4). |
| Trigger location | **Reconcile over established contacts** (load + `dashpay_sync`), not registration. |
| Upstream method | **None** — `get_wallet_info_mut().update_synced_height()` already bypasses the wrapper guard. |

---

## 7. Task breakdown

Independent, separately-mergeable. **No rust-dashcore / upstream task.**

| Task | Scope | Depends on |
|---|---|---|
| **A — restore birth-height default** | DashPay re-import passes `Some(0)` (or known funding height) instead of `None` (`PlatformWalletManager.swift:317`/`manager.rs:245`). Durable; covers restore/second-device. Can ship alone. | — |
| **B — rescan reconcile + primitive** | `SpvRuntime::rewind_synced_height` (§3.3) + a `reconcile_dashpay_rescan` step (§3.2) reading floors off `EstablishedContact`, coalesced, receival-only, clamp+warn; wired into wallet-load and `dashpay_sync` (sibling to `reconcile_incoming_payments`). Covers offline-accept→pay + fresh-establish. | — |
| **C — tests** | §5 unit suite; `dp_*` e2e folded behind #3549 + devnet funding. | A, B |
| **D — durability breadcrumb (OPTIONAL, deferred)** | Persist a "pending rescan floor" so a crash mid-backfill resumes (§3.4 accepted-limitation). Only if at-least-once-across-restart becomes a requirement. | B |
| ~~T1 upstream guard-bypass~~ | **DELETED** — no upstream change needed (§3.3). | — |

(`DIP_CONFORMANCE_GAPS.md` §1.2 account-label padding is a **separate, unrelated** bug,
tracked in TODO P1 — not part of this spec.)

---

## 8. Review record (2026-06-24, 4 lenses)

- **SPV feasibility (CORRECT-WITH-CAVEATS):** mechanism real & test-pinned; corrected
  "re-download"→"reuse stored filters"; surfaced the filter-unavailable silent-stall;
  confirmed no upstream method needed.
- **Regression safety (SAFE — gate resolved):** full consumer audit; reuse
  `synced_height`; decoupled from `last_processed_height`; DashPay cursor is a separate
  ms namespace; one durability caveat (crash mid-backfill, §3.4).
- **Correctness/failure-modes (1 BLOCKING, fixed):** trigger was dead on restore
  (sweep enqueues, drain registers, restore early-exits); height source wrong and
  unreachable at the registration site → restructured to reconcile-off-`EstablishedContact`;
  receival-only confirmed; idempotency confirmed; coalescing required.
- **Scope/simplicity (OVER-SCOPED, fixed):** deleted upstream T1; split restore into the
  birth-height one-liner; dropped `REWIND_SLACK`; resolved the open questions into
  decisions; minimal first slice = task B (or A) as a single platform-wallet PR.
